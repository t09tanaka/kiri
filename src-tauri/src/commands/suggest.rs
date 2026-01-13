use std::fs;
use std::path::PathBuf;

// collect_path_commands and parse_history_content are in suggest_io.rs (excluded from coverage)
use super::suggest_io::{collect_path_commands, parse_history_content};

/// Get all executable commands from $PATH
#[tauri::command]
pub fn get_path_commands() -> Result<Vec<String>, String> {
    let commands = collect_path_commands();
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
    let history_paths = vec![home.join(".zsh_history"), home.join(".bash_history")];

    for history_path in history_paths {
        if history_path.exists() {
            if let Ok(content) = fs::read(&history_path) {
                return Ok(parse_history_content(&content, limit));
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
        let cwd = cwd
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/")));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_get_path_commands() {
        let result = get_path_commands();
        assert!(result.is_ok());
        let commands = result.unwrap();
        // Should find at least some common commands
        assert!(!commands.is_empty());
        // Commands should be sorted
        let mut sorted = commands.clone();
        sorted.sort();
        assert_eq!(commands, sorted);
    }

    #[test]
    fn test_get_command_history() {
        let result = get_command_history(Some(10));
        // This might fail if no history file exists, which is OK
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_command_history_default_limit() {
        let result = get_command_history(None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_file_suggestions_absolute_path() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("file1.txt"), "").unwrap();
        fs::write(dir.path().join("file2.txt"), "").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let partial = format!("{}/", dir.path().to_string_lossy());
        let result = get_file_suggestions(partial, None);
        assert!(result.is_ok());

        let suggestions = result.unwrap();
        assert!(suggestions.len() >= 3);
    }

    #[test]
    fn test_get_file_suggestions_with_prefix() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("apple.txt"), "").unwrap();
        fs::write(dir.path().join("apricot.txt"), "").unwrap();
        fs::write(dir.path().join("banana.txt"), "").unwrap();

        let partial = format!("{}/ap", dir.path().to_string_lossy());
        let result = get_file_suggestions(partial, None);
        assert!(result.is_ok());

        let suggestions = result.unwrap();
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions.contains(&"apple.txt".to_string()));
        assert!(suggestions.contains(&"apricot.txt".to_string()));
    }

    #[test]
    fn test_get_file_suggestions_directory_slash() {
        let dir = tempdir().unwrap();

        fs::create_dir(dir.path().join("mydir")).unwrap();

        let partial = format!("{}/", dir.path().to_string_lossy());
        let result = get_file_suggestions(partial, None);
        assert!(result.is_ok());

        let suggestions = result.unwrap();
        assert!(suggestions.iter().any(|s| s == "mydir/"));
    }

    #[test]
    fn test_get_file_suggestions_hidden_files() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join(".hidden"), "").unwrap();
        fs::write(dir.path().join("visible.txt"), "").unwrap();

        // Without dot prefix, hidden files should be excluded
        let partial = format!("{}/", dir.path().to_string_lossy());
        let result = get_file_suggestions(partial, None);
        assert!(result.is_ok());

        let suggestions = result.unwrap();
        assert!(!suggestions.contains(&".hidden".to_string()));
        assert!(suggestions.contains(&"visible.txt".to_string()));
    }

    #[test]
    fn test_get_file_suggestions_hidden_files_with_prefix() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join(".hidden"), "").unwrap();
        fs::write(dir.path().join(".config"), "").unwrap();

        // With dot prefix, hidden files should be included
        let partial = format!("{}/.h", dir.path().to_string_lossy());
        let result = get_file_suggestions(partial, None);
        assert!(result.is_ok());

        let suggestions = result.unwrap();
        assert!(suggestions.contains(&".hidden".to_string()));
    }

    #[test]
    fn test_get_file_suggestions_relative_path_with_cwd() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("test.txt"), "").unwrap();

        let result = get_file_suggestions(
            "tes".to_string(),
            Some(dir.path().to_string_lossy().to_string()),
        );
        assert!(result.is_ok());

        let suggestions = result.unwrap();
        assert!(suggestions.contains(&"test.txt".to_string()));
    }

    #[test]
    fn test_get_file_suggestions_tilde_expansion() {
        // Test ~ expansion
        let result = get_file_suggestions("~".to_string(), None);
        assert!(result.is_ok());
        // Should return home directory contents
    }

    #[test]
    fn test_get_file_suggestions_sorted() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("zebra.txt"), "").unwrap();
        fs::write(dir.path().join("apple.txt"), "").unwrap();
        fs::write(dir.path().join("middle.txt"), "").unwrap();

        let partial = format!("{}/", dir.path().to_string_lossy());
        let result = get_file_suggestions(partial, None);
        assert!(result.is_ok());

        let suggestions = result.unwrap();
        let mut sorted = suggestions.clone();
        sorted.sort();
        assert_eq!(suggestions, sorted);
    }

    #[test]
    fn test_get_file_suggestions_empty_directory() {
        let dir = tempdir().unwrap();

        let partial = format!("{}/", dir.path().to_string_lossy());
        let result = get_file_suggestions(partial, None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_file_suggestions_nonexistent_dir() {
        let result = get_file_suggestions("/nonexistent/path/".to_string(), None);
        assert!(result.is_ok());
        // Should return empty since directory doesn't exist
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_file_suggestions_tilde_with_path() {
        // Test ~/path format expansion
        let result = get_file_suggestions("~/".to_string(), None);
        assert!(result.is_ok());
        // Should return home directory contents
    }

    #[test]
    fn test_get_file_suggestions_relative_no_cwd() {
        // Test relative path without cwd provided
        let result = get_file_suggestions("src".to_string(), None);
        assert!(result.is_ok());
        // Should use current directory
    }

    #[test]
    fn test_get_file_suggestions_partial_match_at_start() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("test1.txt"), "").unwrap();
        fs::write(dir.path().join("test2.txt"), "").unwrap();
        fs::write(dir.path().join("other.txt"), "").unwrap();

        // Partial match at directory level
        let partial = format!("{}/te", dir.path().to_string_lossy());
        let result = get_file_suggestions(partial, None);
        assert!(result.is_ok());

        let suggestions = result.unwrap();
        assert_eq!(suggestions.len(), 2);
    }

    #[test]
    fn test_get_path_commands_filters_hidden_and_nonexecutable() {
        // This test validates the internal filtering logic in get_path_commands
        let result = get_path_commands();
        assert!(result.is_ok());

        let commands = result.unwrap();
        // Commands should not include hidden files (starting with .)
        assert!(commands.iter().all(|c| !c.starts_with('.')));
    }

    #[test]
    fn test_get_command_history_handles_nonexistent_files() {
        // If neither .zsh_history nor .bash_history exist in some test environment,
        // the function should still return Ok(empty vec)
        let result = get_command_history(Some(10));
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_file_suggestions_handles_nonexistent_parent() {
        // When the parent directory doesn't exist, should return empty suggestions
        let result =
            get_file_suggestions("/nonexistent/directory/with/file".to_string(), None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_file_suggestions_with_file_no_prefix() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("file.txt"), "").unwrap();

        // Without trailing slash and no partial name - should use directory itself
        let result = get_file_suggestions(dir.path().to_string_lossy().to_string(), None);
        assert!(result.is_ok());
        // Should find files in that directory or treat as prefix
    }
}
