//! Sentinel string handling for `Request::Run`.
//!
//! `Run` works by appending a "done" sentinel that prints the previous
//! command's exit code so the server can detect completion in PTY output:
//!
//! ```text
//! <cmd>; printf '\n__KIRI_DONE_<nonce>__%d__\n' "$?"
//! ```
//!
//! The server then watches the PTY byte stream for the sentinel pattern
//! `__KIRI_DONE_<nonce>__([0-9]+)__` and extracts the exit code.

use regex::Regex;

pub struct Sentinel {
    pub nonce: String,
    pattern: Regex,
}

impl Sentinel {
    pub fn new(nonce: impl Into<String>) -> Self {
        let nonce = nonce.into();
        let pattern_str = format!(r"__KIRI_DONE_{}__([0-9]+)__", regex::escape(&nonce));
        let pattern = Regex::new(&pattern_str).expect("sentinel pattern compiles");
        Self { nonce, pattern }
    }

    /// Build the shell payload to write into the PTY.
    pub fn payload(&self, cmd: &str) -> String {
        format!(
            "{cmd}; printf '\\n__KIRI_DONE_{nonce}__%d__\\n' \"$?\"\n",
            cmd = cmd,
            nonce = self.nonce,
        )
    }

    /// Search `data` for the sentinel. Returns
    /// `(exit_code, start_byte_index, end_byte_index)` of the match.
    pub fn find(&self, data: &[u8]) -> Option<(i32, usize, usize)> {
        let s = std::str::from_utf8(data).ok()?;
        let m = self.pattern.captures(s)?;
        let exit: i32 = m.get(1)?.as_str().parse().ok()?;
        let whole = m.get(0)?;
        Some((exit, whole.start(), whole.end()))
    }
}

/// Trim the command echo and the sentinel line(s) from `data`, returning
/// just the command's textual output.
///
/// 1. Use the regex match's `sentinel_start` to find the start of the
///    line containing the actual sentinel, and slice everything before it.
///    (Matching the literal substring `__KIRI_DONE_` is unsafe — when the
///    shell echoes our payload back inline, that substring appears in the
///    `printf` invocation too, and we'd slice off the real output.)
/// 2. Drop the leading line(s) that echoed our payload back at us. We
///    handle three observed shapes: bare `<cmd>` echo, `<cmd>` echo with
///    our injected `; printf '...'` suffix, and ANSI-redraw-mangled echo
///    that still contains a recognisable sentinel-marker fragment.
pub fn extract_output(
    data: &[u8],
    cmd: &str,
    sentinel_start: usize,
    sentinel_end: usize,
) -> String {
    let _ = sentinel_end; // kept for symmetry with `find`; line cut uses start.
    let start = sentinel_start.min(data.len());

    // Walk back from `sentinel_start` to the previous '\n' to find the
    // start of the sentinel line. Operate on raw bytes so indexing matches
    // `Sentinel::find`'s byte offsets even if `data` has invalid UTF-8.
    let sentinel_line_start = data[..start]
        .iter()
        .rposition(|&b| b == b'\n')
        .map(|i| i + 1)
        .unwrap_or(0);

    let text_bytes = &data[..sentinel_line_start];
    let text = String::from_utf8_lossy(text_bytes);
    let mut text: &str = text.as_ref();

    let cmd_first_line = cmd.lines().next().unwrap_or("");
    if let Some(stripped) = strip_leading_payload_echo(text, cmd_first_line) {
        text = stripped;
    }

    // Trim a single trailing '\n' if present so the caller-visible
    // output doesn't end with a stray newline produced by the sentinel
    // printf prefix.
    let trimmed = text.strip_suffix('\n').unwrap_or(text);
    trimmed.to_string()
}

/// If the first line of `text` looks like the shell's echo of our payload,
/// strip it and return the rest. Returns `None` when nothing was stripped.
///
/// Three accepted shapes:
/// 1. Exactly `cmd_first_line` — bracketed-paste hid the printf.
/// 2. Starts/ends with `cmd_first_line` — full payload echoed inline.
/// 3. Contains `__KIRI_DONE_` — ANSI-mangled echo where (1) and (2) failed
///    but the marker substring still leaks through.
fn strip_leading_payload_echo<'a>(text: &'a str, cmd_first_line: &str) -> Option<&'a str> {
    let mut iter = text.splitn(2, '\n');
    let first = iter.next()?;
    let rest = iter.next().unwrap_or("");
    let first_clean = first.trim_end_matches('\r');

    if !cmd_first_line.is_empty()
        && (first_clean == cmd_first_line
            || first_clean.starts_with(cmd_first_line)
            || first_clean.ends_with(cmd_first_line))
    {
        return Some(rest);
    }

    // Cmd is present anywhere in the line AND our injected `printf '`
    // is also present — this catches ANSI-redraw-mangled echoes where
    // the cmd is preceded by control bytes (e.g. `g\x08git status...`)
    // so plain prefix/suffix checks miss it. Pairing with `printf '`
    // avoids false-positives on real output that happens to mention
    // the cmd verbatim.
    if !cmd_first_line.is_empty()
        && first_clean.contains(cmd_first_line)
        && first_clean.contains("printf '")
    {
        return Some(rest);
    }

    if first_clean.contains("__KIRI_DONE_") {
        return Some(rest);
    }

    None
}

/// Return the last `n` lines of `text` (using `split_inclusive('\n')`),
/// plus the number of leading lines that were dropped. If `n >= total`,
/// returns the full text and `0`.
pub fn tail_lines(text: &str, n: usize) -> (String, usize) {
    let chunks: Vec<&str> = text.split_inclusive('\n').collect();
    let total = chunks.len();
    if n >= total {
        return (text.to_string(), 0);
    }
    let omitted = total - n;
    let kept: String = chunks.iter().skip(omitted).copied().collect();
    (kept, omitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_appends_sentinel_with_exit() {
        let s = Sentinel::new("xyz");
        let p = s.payload("ls");
        assert!(p.starts_with("ls; printf"), "got: {p}");
        assert!(p.contains("__KIRI_DONE_xyz__"), "got: {p}");
        assert!(p.ends_with('\n'), "got: {p}");
    }

    #[test]
    fn finds_exit_code() {
        let s = Sentinel::new("xyz");
        let result = s.find(b"hello\n__KIRI_DONE_xyz__0__\n");
        assert!(matches!(result, Some((0, _, _))));
    }

    #[test]
    fn finds_nonzero_exit() {
        let s = Sentinel::new("xyz");
        let result = s.find(b"oops\n__KIRI_DONE_xyz__127__\n");
        assert!(matches!(result, Some((127, _, _))));
    }

    #[test]
    fn returns_none_when_not_found() {
        let s = Sentinel::new("xyz");
        assert!(s.find(b"random output").is_none());
    }

    #[test]
    fn extract_output_drops_command_echo_and_sentinel() {
        let s = Sentinel::new("xyz");
        let data: &[u8] = b"ls\nfile1\nfile2\n__KIRI_DONE_xyz__0__\n";
        let (_, start, end) = s.find(data).expect("sentinel should be found");
        let out = extract_output(data, "ls", start, end);
        assert_eq!(out, "file1\nfile2");
    }

    #[test]
    fn extract_output_when_shell_echoes_payload_inline() {
        // Reproduces the v0.4.0 bug where the shell echoes the full
        // payload (cmd + injected printf) on a single line. The previous
        // implementation matched the marker substring inside the echo
        // and sliced off the actual output.
        let s = Sentinel::new("00000001");
        let data: &[u8] = b"echo hello; printf '\\n__KIRI_DONE_00000001__%d__\\n' \"$?\"\nhello\n__KIRI_DONE_00000001__0__\n";
        let (exit, start, end) = s.find(data).expect("sentinel should be found");
        assert_eq!(exit, 0);
        let out = extract_output(data, "echo hello", start, end);
        assert_eq!(out, "hello");
    }

    #[test]
    fn extract_output_when_echo_has_ansi_redraws() {
        // Reproduces the v0.4.0 case where bash inline-edit/redraw codes
        // break up the marker in the echo, so the literal substring
        // doesn't appear in the echo line itself — only on the real
        // sentinel line. Output must include the real command output and
        // exclude the ANSI-mangled echo.
        let s = Sentinel::new("00000000");
        let data: &[u8] = b"g\x08git status; printf '\\n__ \r\x1b[KK\rKIRI_DONE_00000000__%d__\\n' \"$?\"\r\nOn branch main\n__KIRI_DONE_00000000__0__\n";
        let (exit, start, end) = s.find(data).expect("sentinel should be found");
        assert_eq!(exit, 0);
        let out = extract_output(data, "git status", start, end);
        assert_eq!(out, "On branch main");
    }

    #[test]
    fn tail_lines_truncates() {
        let (out, omitted) = tail_lines("a\nb\nc\nd\n", 2);
        assert_eq!(out, "c\nd\n");
        assert_eq!(omitted, 2);
    }

    #[test]
    fn tail_lines_smaller_than_input_returns_all() {
        let (out, omitted) = tail_lines("a\nb\n", 5);
        assert_eq!(out, "a\nb\n");
        assert_eq!(omitted, 0);
    }
}
