// This file contains gitignore checking logic that is difficult to achieve
// full coverage due to coverage tool limitations with branch tracking.
// Tested via E2E tests.

use git2::Repository;
use std::path::Path;

/// Check if a path is gitignored
/// Returns true if the path matches gitignore rules
pub fn check_gitignore(repo: &Repository, entry_path: &Path, is_dir: bool) -> bool {
    let workdir = match repo.workdir() {
        Some(w) => w,
        None => return false,
    };

    let repo_path = match entry_path.strip_prefix(workdir) {
        Ok(p) => p,
        Err(_) => return false,
    };

    if is_dir {
        let dir_path = format!("{}/", repo_path.to_string_lossy());
        repo.is_path_ignored(&dir_path).unwrap_or(false)
    } else {
        repo.is_path_ignored(repo_path).unwrap_or(false)
    }
}
