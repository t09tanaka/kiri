// This file contains I/O operations with error handling that requires
// system-level failures to test. Covered via E2E tests.

use std::fs;
use std::path::Path;

/// Read file contents with error handling
pub fn read_file_contents(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("Failed to read file: {}", e))
}
