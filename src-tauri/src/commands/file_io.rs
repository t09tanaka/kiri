// This file contains I/O operations with error handling that requires
// system-level failures to test. Covered via E2E tests.

use std::fs;
use std::path::Path;

/// Read file contents with error handling
pub fn read_file_contents(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
}

/// Write file contents with error handling
pub fn write_file_contents(path: &Path, content: &str) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("Failed to write file: {}", e))
}

/// Create parent directories if needed
pub fn create_parent_dirs(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create directories: {}", e))?;
        }
    }
    Ok(())
}
