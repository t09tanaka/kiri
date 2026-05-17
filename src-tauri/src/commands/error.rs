//! Helpers for building user-facing error messages that do not leak
//! filesystem paths or other sensitive backend detail.
//!
//! The renderer (and remote clients reached via the remote-access server)
//! receive whatever string a `#[tauri::command]` returns in the `Err` arm.
//! Returning `format!("Path does not exist: {}", path.display())` therefore
//! exposes absolute filesystem paths — including the user's home directory
//! and project layout — to anyone who can talk to the IPC bridge.
//!
//! Use [`user_message`] (or [`user_io_error`]) to log the full detail at
//! the backend and return a redacted string to the caller.

use std::path::Path;

/// Log `detail` at `warn!` level and return `summary` for the caller.
///
/// `summary` should be a short, user-readable string that does NOT contain
/// the path, IO error chain, or any other backend detail. `detail` is
/// always logged on the backend so an operator inspecting `~/.kiri/logs`
/// can still diagnose what happened.
pub fn user_message(summary: &'static str, detail: impl std::fmt::Display) -> String {
    log::warn!("{}: {}", summary, detail);
    summary.to_string()
}

/// Convenience for the very common `"<summary>: <path.display()>"` pattern.
pub fn user_path_error(summary: &'static str, path: &Path) -> String {
    user_message(summary, format_args!("{}", path.display()))
}

/// Wrap a `std::io::Error` for return through the IPC boundary.
pub fn user_io_error(summary: &'static str, err: impl std::fmt::Display) -> String {
    user_message(summary, err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn user_message_returns_summary_only() {
        let msg = user_message("File not found", "/home/secret/path");
        assert_eq!(msg, "File not found");
        assert!(!msg.contains("secret"));
    }

    #[test]
    fn user_path_error_returns_summary_only() {
        let msg = user_path_error("File not found", Path::new("/home/secret/path"));
        assert_eq!(msg, "File not found");
        assert!(!msg.contains("/home"));
    }

    #[test]
    fn user_io_error_returns_summary_only() {
        let io_err = std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "owner=root mode=0600",
        );
        let msg = user_io_error("Failed to read file", io_err);
        assert_eq!(msg, "Failed to read file");
        assert!(!msg.contains("root"));
    }
}
