// This file contains suggestion I/O operations with error handling that requires
// system-level failures to test. Covered via E2E tests.

use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// Get all executable commands from $PATH
/// This function has many nested error handling paths
pub fn collect_path_commands() -> HashSet<String> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let mut commands: HashSet<String> = HashSet::new();

    for dir in path_var.split(':') {
        let path = PathBuf::from(dir);
        if !path.is_dir() {
            continue;
        }

        if let Ok(entries) = fs::read_dir(&path) {
            for entry in entries.flatten() {
                let file_path = entry.path();

                // Check if it's a file and executable
                if file_path.is_file() {
                    if let Ok(metadata) = fs::metadata(&file_path) {
                        let permissions = metadata.permissions();
                        // Check if executable (any execute bit set)
                        if permissions.mode() & 0o111 != 0 {
                            if let Some(name) = file_path.file_name() {
                                if let Some(name_str) = name.to_str() {
                                    // Skip hidden files
                                    if !name_str.starts_with('.') {
                                        commands.insert(name_str.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    commands
}

/// Parse command history from file content
/// Handles both zsh extended format and bash format
pub fn parse_history_content(content: &[u8], limit: usize) -> Vec<String> {
    let content_str = String::from_utf8_lossy(content);
    let mut commands: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    // Parse history (reverse to get most recent first)
    for line in content_str.lines().rev() {
        // Skip empty lines
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // Handle zsh extended history format: ": timestamp:0;command"
        let command = if line.starts_with(':') {
            if let Some(idx) = line.find(';') {
                line[idx + 1..].trim()
            } else {
                line
            }
        } else {
            line
        };

        // Skip if empty or already seen
        if command.is_empty() || seen.contains(command) {
            continue;
        }

        seen.insert(command.to_string());
        commands.push(command.to_string());

        if commands.len() >= limit {
            break;
        }
    }

    commands
}
