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
    /// `(exit_code, end_byte_index_of_match)` if found.
    pub fn find(&self, data: &[u8]) -> Option<(i32, usize)> {
        let s = std::str::from_utf8(data).ok()?;
        let m = self.pattern.captures(s)?;
        let exit: i32 = m.get(1)?.as_str().parse().ok()?;
        let whole = m.get(0)?;
        Some((exit, whole.end()))
    }
}

/// Trim the command echo and the sentinel line(s) from `data`, returning
/// just the command's textual output.
///
/// 1. Slice off everything from the sentinel onward (incl. its line).
/// 2. If the very first line of what's left is an echo of `cmd` (or its
///    first physical line), drop that too.
pub fn extract_output(data: &[u8], cmd: &str, sentinel_end: usize) -> String {
    let upto = sentinel_end.min(data.len());
    let head = &data[..upto];
    let head_str = String::from_utf8_lossy(head);

    // Drop the sentinel line itself: walk back to the previous '\n'
    // before the sentinel and slice there.
    let mut text: &str = head_str.as_ref();
    if let Some(idx) = find_sentinel_line_start(text, cmd_done_marker_prefix(text)) {
        text = &text[..idx];
    }

    // Drop a leading echo of `cmd`'s first physical line, if present.
    let cmd_first_line = cmd.lines().next().unwrap_or("");
    if !cmd_first_line.is_empty() {
        if let Some(stripped) = strip_leading_command_echo(text, cmd_first_line) {
            text = stripped;
        }
    }

    // Trim a single trailing '\n' if present so the caller-visible
    // output doesn't end with a stray newline produced by the sentinel
    // printf prefix.
    let trimmed = text.strip_suffix('\n').unwrap_or(text);
    trimmed.to_string()
}

/// Return the byte index of the start of the line that contains the
/// sentinel marker in `text`. We pass in the marker prefix so callers
/// can pre-compute it for clarity.
fn find_sentinel_line_start(text: &str, marker_prefix: &str) -> Option<usize> {
    let pos = text.find(marker_prefix)?;
    // Walk back to start of that line.
    Some(text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0))
}

fn cmd_done_marker_prefix(_text: &str) -> &'static str {
    "__KIRI_DONE_"
}

/// If the first line of `text` matches `cmd_first_line` (after stripping
/// any common shell-emitted control chars like CR), strip that line and
/// return the rest. Otherwise return `None`.
fn strip_leading_command_echo<'a>(text: &'a str, cmd_first_line: &str) -> Option<&'a str> {
    let mut iter = text.splitn(2, '\n');
    let first = iter.next()?;
    let rest = iter.next().unwrap_or("");
    let first_clean = first.trim_end_matches('\r');
    if first_clean == cmd_first_line || first_clean.ends_with(cmd_first_line) {
        Some(rest)
    } else {
        None
    }
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
        assert!(matches!(result, Some((0, _))));
    }

    #[test]
    fn finds_nonzero_exit() {
        let s = Sentinel::new("xyz");
        let result = s.find(b"oops\n__KIRI_DONE_xyz__127__\n");
        assert!(matches!(result, Some((127, _))));
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
        let (_, end) = s.find(data).expect("sentinel should be found");
        let out = extract_output(data, "ls", end);
        assert_eq!(out, "file1\nfile2");
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
