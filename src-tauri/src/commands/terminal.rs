use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtyPair, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::str;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct TerminalOutput {
    pub id: u32,
    pub data: String,
}

pub struct PtyInstance {
    pub master: Box<dyn MasterPty + Send>,
    pub writer: Box<dyn Write + Send>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
    pub shell_pid: Option<u32>,
}

pub struct TerminalManager {
    pub instances: HashMap<u32, PtyInstance>,
    pub next_id: u32,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            next_id: 1,
        }
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

pub type TerminalState = Arc<Mutex<TerminalManager>>;

/// Resolve terminal size with defaults
/// Returns (cols, rows)
pub fn resolve_terminal_size(cols: Option<u16>, rows: Option<u16>) -> (u16, u16) {
    let default_cols = 120;
    let default_rows = 30;
    (cols.unwrap_or(default_cols), rows.unwrap_or(default_rows))
}

/// Create a PtySize struct from cols and rows
pub fn create_pty_size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

/// Get the shell path from environment or use default
pub fn get_shell_path() -> String {
    std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string())
}

/// Resolve the working directory for the terminal
/// Returns the cwd if provided, otherwise the home directory, or None
pub fn resolve_cwd(cwd: Option<String>) -> Option<String> {
    match cwd {
        Some(dir) => Some(dir),
        None => dirs::home_dir().map(|p| p.to_string_lossy().to_string()),
    }
}

/// Build a shell command with the given configuration
pub fn build_shell_command(shell: &str, cwd: Option<&str>) -> CommandBuilder {
    let mut cmd = CommandBuilder::new(shell);
    cmd.arg("-l"); // Login shell

    // Set TERM to enable proper ANSI escape sequence handling
    // This is critical for CLI tools like Claude Code that use cursor movement
    // and line clearing for progress indicators
    cmd.env("TERM", "xterm-256color");

    if let Some(dir) = cwd {
        cmd.cwd(dir);
    }

    cmd
}

/// Find the last valid UTF-8 boundary in a byte slice.
/// Returns the number of bytes that form valid UTF-8 from the start.
/// Any remaining bytes are incomplete multi-byte sequences that should be
/// preserved for the next read.
///
/// This is essential for handling PTY output correctly, as reads can split
/// multi-byte UTF-8 characters (like Japanese, emojis, etc.) across buffer
/// boundaries. Without proper boundary handling, these characters would be
/// corrupted by `String::from_utf8_lossy`.
pub fn find_utf8_boundary(buf: &[u8]) -> usize {
    let len = buf.len();
    if len == 0 {
        return 0;
    }

    // Check if the entire buffer is valid UTF-8
    if str::from_utf8(buf).is_ok() {
        return len;
    }

    // Find the last valid UTF-8 boundary by checking from the end
    // UTF-8 multi-byte sequences can be 1-4 bytes long
    // We need to find where an incomplete sequence starts
    for i in 1..=4.min(len) {
        let check_pos = len - i;
        if str::from_utf8(&buf[..check_pos]).is_ok() {
            return check_pos;
        }
    }

    // If we can't find a valid boundary, return 0
    // This handles severely corrupted data
    0
}

/// Result of opening a PTY with a spawned shell
pub struct PtyWithShell {
    pub pair: PtyPair,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

/// Open a PTY and spawn a shell command
/// This is the core PTY creation logic extracted for testability
pub fn open_pty_with_shell(
    cols: u16,
    rows: u16,
    cwd: Option<&str>,
) -> Result<PtyWithShell, String> {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(create_pty_size(cols, rows))
        .map_err(|e| format!("Failed to open PTY: {}", e))?;

    let shell = get_shell_path();
    let cmd = build_shell_command(&shell, cwd);

    let child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("Failed to spawn shell: {}", e))?;

    Ok(PtyWithShell { pair, child })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_manager_new() {
        let manager = TerminalManager::new();
        assert!(manager.instances.is_empty());
        assert_eq!(manager.next_id, 1);
    }

    #[test]
    fn test_terminal_manager_default() {
        let manager = TerminalManager::default();
        assert!(manager.instances.is_empty());
        assert_eq!(manager.next_id, 1);
    }

    #[test]
    fn test_terminal_output_struct() {
        let output = TerminalOutput {
            id: 1,
            data: "hello world".to_string(),
        };
        assert_eq!(output.id, 1);
        assert_eq!(output.data, "hello world");
    }

    #[test]
    fn test_terminal_output_clone() {
        let output = TerminalOutput {
            id: 42,
            data: "test data".to_string(),
        };
        let cloned = output.clone();
        assert_eq!(cloned.id, output.id);
        assert_eq!(cloned.data, output.data);
    }

    #[test]
    fn test_resolve_terminal_size_with_defaults() {
        let (cols, rows) = resolve_terminal_size(None, None);
        assert_eq!(cols, 120);
        assert_eq!(rows, 30);
    }

    #[test]
    fn test_resolve_terminal_size_with_custom_cols() {
        let (cols, rows) = resolve_terminal_size(Some(80), None);
        assert_eq!(cols, 80);
        assert_eq!(rows, 30);
    }

    #[test]
    fn test_resolve_terminal_size_with_custom_rows() {
        let (cols, rows) = resolve_terminal_size(None, Some(24));
        assert_eq!(cols, 120);
        assert_eq!(rows, 24);
    }

    #[test]
    fn test_resolve_terminal_size_with_custom_both() {
        let (cols, rows) = resolve_terminal_size(Some(100), Some(50));
        assert_eq!(cols, 100);
        assert_eq!(rows, 50);
    }

    #[test]
    fn test_create_pty_size() {
        let size = create_pty_size(80, 24);
        assert_eq!(size.cols, 80);
        assert_eq!(size.rows, 24);
        assert_eq!(size.pixel_width, 0);
        assert_eq!(size.pixel_height, 0);
    }

    #[test]
    fn test_get_shell_path() {
        let shell = get_shell_path();
        // Should return something (either from env or default)
        assert!(!shell.is_empty());
        // Should be a valid path format
        assert!(shell.starts_with('/') || shell.contains("sh"));
    }

    #[test]
    fn test_resolve_cwd_with_value() {
        let cwd = resolve_cwd(Some("/tmp".to_string()));
        assert_eq!(cwd, Some("/tmp".to_string()));
    }

    #[test]
    fn test_resolve_cwd_without_value() {
        let cwd = resolve_cwd(None);
        // Should return home directory
        assert!(cwd.is_some());
        let cwd_path = cwd.unwrap();
        assert!(!cwd_path.is_empty());
    }

    #[test]
    fn test_build_shell_command_basic() {
        let cmd = build_shell_command("/bin/bash", None);
        // CommandBuilder doesn't expose its internals, but we can verify it was created
        let _prog = cmd.get_argv(); // This should not panic
    }

    #[test]
    fn test_build_shell_command_with_cwd() {
        let cmd = build_shell_command("/bin/bash", Some("/tmp"));
        let _prog = cmd.get_argv(); // This should not panic
    }

    #[test]
    fn test_build_shell_command_with_different_shells() {
        for shell in ["/bin/bash", "/bin/zsh", "/bin/sh"] {
            let cmd = build_shell_command(shell, None);
            let argv = cmd.get_argv();
            assert!(!argv.is_empty());
        }
    }

    #[test]
    fn test_open_pty_with_shell_default_size() {
        let result = open_pty_with_shell(80, 24, None);
        assert!(result.is_ok());

        let pty = result.unwrap();
        // Verify we can get reader and writer
        let reader = pty.pair.master.try_clone_reader();
        assert!(reader.is_ok());
    }

    #[test]
    fn test_open_pty_with_shell_with_cwd() {
        let result = open_pty_with_shell(80, 24, Some("/tmp"));
        assert!(result.is_ok());
    }

    #[test]
    fn test_open_pty_with_shell_various_sizes() {
        for (cols, rows) in [(80, 24), (120, 30), (200, 50)] {
            let result = open_pty_with_shell(cols, rows, None);
            assert!(result.is_ok(), "Failed for size {}x{}", cols, rows);
        }
    }

    #[test]
    fn test_pty_with_shell_can_resize() {
        let result = open_pty_with_shell(80, 24, None);
        assert!(result.is_ok());

        let pty = result.unwrap();
        // Verify resize works
        let resize_result = pty.pair.master.resize(create_pty_size(120, 40));
        assert!(resize_result.is_ok());
    }

    #[test]
    fn test_pty_with_shell_struct_fields() {
        let result = open_pty_with_shell(80, 24, None);
        assert!(result.is_ok());

        let pty = result.unwrap();
        // Verify struct has expected fields accessible
        let _ = &pty.pair;
        let _ = &pty.child;
    }

    #[test]
    fn test_find_utf8_boundary_empty() {
        assert_eq!(find_utf8_boundary(&[]), 0);
    }

    #[test]
    fn test_find_utf8_boundary_ascii() {
        let data = b"Hello, World!";
        assert_eq!(find_utf8_boundary(data), data.len());
    }

    #[test]
    fn test_find_utf8_boundary_complete_utf8() {
        // Japanese text: "こんにちは"
        let data = "こんにちは".as_bytes();
        assert_eq!(find_utf8_boundary(data), data.len());
    }

    #[test]
    fn test_find_utf8_boundary_incomplete_2byte() {
        // 2-byte UTF-8 sequence for 'é' (0xC3 0xA9)
        // Split after first byte
        let complete = "é".as_bytes();
        assert_eq!(complete.len(), 2);
        let incomplete = &[complete[0]]; // Just 0xC3
        assert_eq!(find_utf8_boundary(incomplete), 0);
    }

    #[test]
    fn test_find_utf8_boundary_incomplete_3byte() {
        // 3-byte UTF-8 sequence for 'あ' (0xE3 0x81 0x82)
        let complete = "あ".as_bytes();
        assert_eq!(complete.len(), 3);

        // Split after first byte
        let incomplete1 = &[complete[0]];
        assert_eq!(find_utf8_boundary(incomplete1), 0);

        // Split after second byte
        let incomplete2 = &[complete[0], complete[1]];
        assert_eq!(find_utf8_boundary(incomplete2), 0);
    }

    #[test]
    fn test_find_utf8_boundary_incomplete_4byte() {
        // 4-byte UTF-8 sequence for emoji '😀' (0xF0 0x9F 0x98 0x80)
        let complete = "😀".as_bytes();
        assert_eq!(complete.len(), 4);

        // Split after first byte
        let incomplete1 = &[complete[0]];
        assert_eq!(find_utf8_boundary(incomplete1), 0);

        // Split after second byte
        let incomplete2 = &[complete[0], complete[1]];
        assert_eq!(find_utf8_boundary(incomplete2), 0);

        // Split after third byte
        let incomplete3 = &[complete[0], complete[1], complete[2]];
        assert_eq!(find_utf8_boundary(incomplete3), 0);
    }

    #[test]
    fn test_find_utf8_boundary_mixed_with_incomplete() {
        // "Hello" followed by incomplete Japanese character
        let hello = b"Hello";
        let ja_char = "あ".as_bytes(); // 3 bytes
        let mut data = Vec::new();
        data.extend_from_slice(hello);
        data.push(ja_char[0]); // Only first byte of 'あ'

        // Should return 5 (just "Hello")
        assert_eq!(find_utf8_boundary(&data), 5);
    }

    #[test]
    fn test_find_utf8_boundary_mixed_complete() {
        // "Hello" followed by complete Japanese character
        let data = "Helloあ".as_bytes();
        assert_eq!(find_utf8_boundary(data), data.len());
    }

    #[test]
    fn test_find_utf8_boundary_realistic_scenario() {
        // Simulate a buffer that might occur in practice:
        // Some ASCII, a complete Japanese character, then an incomplete one
        let mut data = Vec::new();
        data.extend_from_slice(b"test ");
        data.extend_from_slice("日本".as_bytes()); // Complete Japanese
        data.push("語".as_bytes()[0]); // First byte of '語' (incomplete)

        let boundary = find_utf8_boundary(&data);
        // Should include "test " (5) + "日本" (6) = 11 bytes
        assert_eq!(boundary, 11);

        // Verify the valid portion
        let valid = std::str::from_utf8(&data[..boundary]).unwrap();
        assert_eq!(valid, "test 日本");
    }
}
