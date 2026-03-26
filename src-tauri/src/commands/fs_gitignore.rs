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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_check_gitignore_file_ignored() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path().canonicalize().unwrap();
        let repo = Repository::init(&dir_path).unwrap();

        // Create .gitignore that ignores *.log files
        fs::write(dir_path.join(".gitignore"), "*.log\n").unwrap();

        let log_file = dir_path.join("debug.log");
        fs::write(&log_file, "log content").unwrap();

        assert!(check_gitignore(&repo, &log_file, false));
    }

    #[test]
    fn test_check_gitignore_file_not_ignored() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path().canonicalize().unwrap();
        let repo = Repository::init(&dir_path).unwrap();

        fs::write(dir_path.join(".gitignore"), "*.log\n").unwrap();

        let txt_file = dir_path.join("readme.txt");
        fs::write(&txt_file, "content").unwrap();

        assert!(!check_gitignore(&repo, &txt_file, false));
    }

    #[test]
    fn test_check_gitignore_directory_ignored() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path().canonicalize().unwrap();
        let repo = Repository::init(&dir_path).unwrap();

        fs::write(dir_path.join(".gitignore"), "build/\n").unwrap();

        let build_dir = dir_path.join("build");
        fs::create_dir(&build_dir).unwrap();

        assert!(check_gitignore(&repo, &build_dir, true));
    }

    #[test]
    fn test_check_gitignore_directory_not_ignored() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path().canonicalize().unwrap();
        let repo = Repository::init(&dir_path).unwrap();

        fs::write(dir_path.join(".gitignore"), "build/\n").unwrap();

        let src_dir = dir_path.join("src");
        fs::create_dir(&src_dir).unwrap();

        assert!(!check_gitignore(&repo, &src_dir, true));
    }

    #[test]
    fn test_check_gitignore_path_outside_repo() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path().canonicalize().unwrap();
        let repo = Repository::init(&dir_path).unwrap();

        // Path outside the repository
        let outside = Path::new("/tmp/outside_repo_file.txt");
        assert!(!check_gitignore(&repo, outside, false));
    }

    #[test]
    fn test_check_gitignore_bare_repo_no_workdir() {
        let dir = tempdir().unwrap();
        let dir_path = dir.path().canonicalize().unwrap();
        let repo = Repository::init_bare(&dir_path).unwrap();

        // Bare repo has no workdir, should return false
        let some_path = dir_path.join("file.txt");
        assert!(!check_gitignore(&repo, &some_path, false));
    }
}
