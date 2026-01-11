use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

/// Get all executable commands from $PATH
#[tauri::command]
pub fn get_path_commands() -> Result<Vec<String>, String> {
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

    let mut result: Vec<String> = commands.into_iter().collect();
    result.sort();
    Ok(result)
}

/// Get command history from shell history file
#[tauri::command]
pub fn get_command_history(limit: Option<usize>) -> Result<Vec<String>, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let limit = limit.unwrap_or(500);

    // Try zsh history first, then bash
    let history_paths = vec![
        home.join(".zsh_history"),
        home.join(".bash_history"),
    ];

    for history_path in history_paths {
        if history_path.exists() {
            if let Ok(content) = fs::read(&history_path) {
                // Handle both UTF-8 and lossy conversion
                let content_str = String::from_utf8_lossy(&content);
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

                return Ok(commands);
            }
        }
    }

    Ok(Vec::new())
}

/// Get file suggestions for path completion
#[tauri::command]
pub fn get_file_suggestions(
    partial_path: String,
    cwd: Option<String>,
) -> Result<Vec<String>, String> {
    let base_path = if partial_path.starts_with('/') {
        PathBuf::from(&partial_path)
    } else if partial_path.starts_with('~') {
        let home = dirs::home_dir().ok_or("Could not find home directory")?;
        if partial_path == "~" {
            home
        } else {
            home.join(&partial_path[2..]) // Skip "~/"
        }
    } else {
        let cwd = cwd.map(PathBuf::from).unwrap_or_else(|| {
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/"))
        });
        cwd.join(&partial_path)
    };

    // Get the directory and prefix to match
    let (search_dir, prefix) = if base_path.is_dir() && partial_path.ends_with('/') {
        (base_path, String::new())
    } else {
        let parent = base_path.parent().unwrap_or(&base_path);
        let prefix = base_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        (parent.to_path_buf(), prefix)
    };

    let mut suggestions: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir(&search_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                // Skip hidden files unless prefix starts with '.'
                if name.starts_with('.') && !prefix.starts_with('.') {
                    continue;
                }

                if name.starts_with(&prefix) {
                    let mut suggestion = name.to_string();
                    // Add trailing slash for directories
                    if entry.path().is_dir() {
                        suggestion.push('/');
                    }
                    suggestions.push(suggestion);
                }
            }
        }
    }

    suggestions.sort();
    Ok(suggestions)
}
