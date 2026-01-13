// This file contains I/O operations with error handling that requires
// system-level failures to test. Covered via E2E tests.

use std::fs::{self, DirEntry};
use std::path::Path;

use git2::Repository;

/// Open git repository from root path
pub fn open_repo(root: &str) -> Option<Repository> {
    Repository::open(root).ok()
}

/// Read directory entries with error handling
pub fn read_dir_entries(path: &Path) -> Result<fs::ReadDir, String> {
    fs::read_dir(path).map_err(|e| e.to_string())
}

/// Get next directory entry with error handling
pub fn get_dir_entry(entry: Result<DirEntry, std::io::Error>) -> Result<DirEntry, String> {
    entry.map_err(|e| e.to_string())
}

/// Get file type with error handling
pub fn get_file_type(entry: &DirEntry) -> Result<fs::FileType, String> {
    entry.file_type().map_err(|e| e.to_string())
}

/// Get home directory with error handling
pub fn get_home_dir() -> Result<String, String> {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine home directory".to_string())
}
