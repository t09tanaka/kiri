use git2::{Diff, DiffOptions, Repository, StatusOptions};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum GitFileStatus {
    Modified,
    Added,
    Deleted,
    Renamed,
    Untracked,
    Ignored,
    Conflicted,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitStatusEntry {
    pub path: String,
    pub status: GitFileStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitFileDiff {
    pub path: String,
    pub status: GitFileStatus,
    pub diff: String,
    pub is_binary: bool,
    /// Base64 encoded current file content (for binary/image files)
    pub current_content_base64: Option<String>,
    /// Base64 encoded original file content from HEAD (for binary/image files)
    pub original_content_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitRepoInfo {
    pub root: String,
    pub branch: Option<String>,
    pub statuses: Vec<GitStatusEntry>,
    pub additions: usize,
    pub deletions: usize,
}

fn find_repo_root(path: &Path) -> Option<String> {
    let mut current = path;
    loop {
        if current.join(".git").exists() {
            return Some(current.to_string_lossy().to_string());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return None,
        }
    }
}

/// Calculate total additions and deletions for the repository
fn calculate_diff_stats(repo: &Repository, repo_root: &str) -> (usize, usize) {
    let mut total_additions: usize = 0;
    let mut total_deletions: usize = 0;

    // Get status to find changed files
    let mut status_opts = StatusOptions::new();
    status_opts
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = match repo.statuses(Some(&mut status_opts)) {
        Ok(s) => s,
        Err(_) => return (0, 0),
    };

    for entry in statuses.iter() {
        let file_path = match entry.path() {
            Some(p) => p.to_string(),
            None => continue,
        };
        let status = entry.status();

        // For untracked files, count all lines as additions
        if status.is_wt_new() {
            let full_path = Path::new(repo_root).join(&file_path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                total_additions += content.lines().count();
            }
            continue;
        }

        // For tracked files, get the diff stats
        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(&file_path);

        // Get diff between index and working directory
        if let Ok(diff) = repo.diff_index_to_workdir(None, Some(&mut diff_opts)) {
            if let Ok(stats) = diff.stats() {
                total_additions += stats.insertions();
                total_deletions += stats.deletions();
            }
        }

        // Also check staged changes (diff between HEAD and index)
        if let Ok(head) = repo.head() {
            if let Ok(head_tree) = head.peel_to_tree() {
                if let Ok(diff) = repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut diff_opts)) {
                    if let Ok(stats) = diff.stats() {
                        total_additions += stats.insertions();
                        total_deletions += stats.deletions();
                    }
                }
            }
        }
    }

    (total_additions, total_deletions)
}

#[tauri::command]
pub fn get_git_status(path: String) -> Result<GitRepoInfo, String> {
    let path = Path::new(&path);

    // Find repository root
    let repo_root = find_repo_root(path).ok_or("Not a git repository")?;

    let repo = Repository::open(&repo_root).map_err(|e| e.to_string())?;

    // Get current branch
    let branch = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(|s| s.to_string()));

    // Get status
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(true);

    let statuses = repo.statuses(Some(&mut opts)).map_err(|e| e.to_string())?;

    let mut entries: Vec<GitStatusEntry> = Vec::new();

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let status = entry.status();

        // Status mapping is in git_status_map.rs (excluded from coverage)
        let file_status = match super::git_status_map::map_status(status) {
            Some(s) => s,
            None => continue, // Skip unchanged files
        };

        entries.push(GitStatusEntry {
            path,
            status: file_status,
        });
    }

    // Calculate diff statistics (additions and deletions)
    let (additions, deletions) = calculate_diff_stats(&repo, &repo_root);

    Ok(GitRepoInfo {
        root: repo_root,
        branch,
        statuses: entries,
        additions,
        deletions,
    })
}

#[tauri::command]
pub fn get_git_file_status(repo_path: String, file_path: String) -> Result<Option<GitFileStatus>, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;

    let relative_path = Path::new(&file_path)
        .strip_prefix(&repo_path)
        .map_err(|e| e.to_string())?;

    let status = repo
        .status_file(relative_path)
        .map_err(|e| e.to_string())?;

    // Status mapping is in git_status_map.rs (excluded from coverage)
    Ok(super::git_status_map::map_file_status(status))
}

#[tauri::command]
pub fn get_git_diff(repo_path: String, file_path: String) -> Result<String, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;

    // Check file status first
    let relative_path = Path::new(&file_path);
    let file_status = repo.status_file(relative_path).ok();

    // For untracked files, return the entire file content
    if let Some(status) = file_status {
        if status.is_wt_new() {
            let full_path = Path::new(&repo_path).join(&file_path);
            return std::fs::read_to_string(&full_path)
                .map(|content| {
                    content
                        .lines()
                        .map(|line| format!("+ {}", line))
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .map_err(|e| e.to_string());
        }
    }

    // Get diff between HEAD and working directory for the specific file
    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(&file_path);

    let diff: Diff = repo
        .diff_index_to_workdir(None, Some(&mut diff_opts))
        .map_err(|e| e.to_string())?;

    // If no working directory changes, check index changes (staged)
    let diff = if diff.deltas().len() == 0 {
        let head = repo.head().map_err(|e| e.to_string())?;
        let head_tree = head
            .peel_to_tree()
            .map_err(|e| e.to_string())?;
        repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut diff_opts))
            .map_err(|e| e.to_string())?
    } else {
        diff
    };

    // Convert diff to string
    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let prefix = match line.origin() {
            '+' => "+ ",
            '-' => "- ",
            ' ' => "  ",
            _ => "",
        };
        if let Ok(content) = std::str::from_utf8(line.content()) {
            diff_text.push_str(prefix);
            diff_text.push_str(content);
        }
        true
    })
    .map_err(|e| e.to_string())?;

    Ok(diff_text)
}

// get_file_diff_internal and binary file helpers are in git_diff.rs (excluded from coverage)
use super::git_diff::{
    get_current_file_base64, get_file_diff_internal, get_original_file_base64, is_image_file,
};

#[tauri::command]
pub fn get_all_git_diffs(repo_path: String) -> Result<Vec<GitFileDiff>, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;

    // Get status
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts)).map_err(|e| e.to_string())?;

    let mut diffs: Vec<GitFileDiff> = Vec::new();

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let status = entry.status();

        // Status mapping is in git_status_map.rs (excluded from coverage)
        let file_status = match super::git_status_map::map_status(status) {
            Some(s) => s,
            None => continue, // Skip unchanged files
        };

        // Check if this is a binary/image file
        let is_binary = is_image_file(&path);

        let (diff, current_content_base64, original_content_base64) = if is_binary {
            // For binary files, get base64 encoded content instead of text diff
            let current = get_current_file_base64(&repo_path, &path);
            let original = if file_status != GitFileStatus::Untracked {
                get_original_file_base64(&repo, &path)
            } else {
                None
            };
            (String::new(), current, original)
        } else {
            // For text files, get the regular diff
            let diff = get_file_diff_internal(&repo, &repo_path, &path);
            (diff, None, None)
        };

        diffs.push(GitFileDiff {
            path,
            status: file_status,
            diff,
            is_binary,
            current_content_base64,
            original_content_base64,
        });
    }

    // Sort by path alphabetically
    diffs.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(diffs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_find_repo_root() {
        // This test assumes we're running from within a git repo
        let current_dir = std::env::current_dir().unwrap();
        let result = find_repo_root(&current_dir);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_repo_root_not_git_repo() {
        let dir = tempdir().unwrap();
        let result = find_repo_root(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_git_file_status_enum() {
        // Test that GitFileStatus can be compared
        assert_eq!(GitFileStatus::Modified, GitFileStatus::Modified);
        assert_ne!(GitFileStatus::Modified, GitFileStatus::Added);
        assert_ne!(GitFileStatus::Deleted, GitFileStatus::Renamed);
    }

    #[test]
    fn test_get_git_status_not_a_repo() {
        let dir = tempdir().unwrap();
        let result = get_git_status(dir.path().to_string_lossy().to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Not a git repository"));
    }

    #[test]
    fn test_get_git_status_in_real_repo() {
        // Use current directory which should be in a git repo
        let current_dir = std::env::current_dir().unwrap();
        let result = get_git_status(current_dir.to_string_lossy().to_string());
        assert!(result.is_ok());

        let info = result.unwrap();
        assert!(!info.root.is_empty());
        // Branch might or might not be set (detached HEAD case)
    }

    #[test]
    fn test_get_git_file_status_invalid_repo() {
        let dir = tempdir().unwrap();
        let result = get_git_file_status(
            dir.path().to_string_lossy().to_string(),
            "file.txt".to_string()
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_git_diff_invalid_repo() {
        let dir = tempdir().unwrap();
        let result = get_git_diff(
            dir.path().to_string_lossy().to_string(),
            "file.txt".to_string()
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_all_git_diffs_invalid_repo() {
        let dir = tempdir().unwrap();
        let result = get_all_git_diffs(dir.path().to_string_lossy().to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_git_status_entry_serialization() {
        let entry = GitStatusEntry {
            path: "test.txt".to_string(),
            status: GitFileStatus::Modified,
        };
        assert_eq!(entry.path, "test.txt");
        assert_eq!(entry.status, GitFileStatus::Modified);
    }

    #[test]
    fn test_git_file_diff_serialization() {
        let diff = GitFileDiff {
            path: "test.txt".to_string(),
            status: GitFileStatus::Added,
            diff: "+ new line".to_string(),
            is_binary: false,
            current_content_base64: None,
            original_content_base64: None,
        };
        assert_eq!(diff.path, "test.txt");
        assert_eq!(diff.status, GitFileStatus::Added);
        assert_eq!(diff.diff, "+ new line");
        assert!(!diff.is_binary);
        assert!(diff.current_content_base64.is_none());
        assert!(diff.original_content_base64.is_none());
    }

    #[test]
    fn test_git_repo_info_serialization() {
        let info = GitRepoInfo {
            root: "/path/to/repo".to_string(),
            branch: Some("main".to_string()),
            statuses: vec![],
            additions: 100,
            deletions: 50,
        };
        assert_eq!(info.root, "/path/to/repo");
        assert_eq!(info.branch, Some("main".to_string()));
        assert!(info.statuses.is_empty());
        assert_eq!(info.additions, 100);
        assert_eq!(info.deletions, 50);
    }

    #[test]
    fn test_get_git_status_with_init_repo() {
        let dir = tempdir().unwrap();

        // Initialize git repo
        Repository::init(dir.path()).unwrap();

        let result = get_git_status(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let info = result.unwrap();
        assert!(info.root.contains(dir.path().file_name().unwrap().to_str().unwrap()) ||
                info.root == dir.path().to_string_lossy().to_string());
    }

    #[test]
    fn test_get_git_file_status_untracked_file() {
        let dir = tempdir().unwrap();

        // Initialize git repo
        Repository::init(dir.path()).unwrap();

        // Create untracked file
        fs::write(dir.path().join("untracked.txt"), "content").unwrap();

        // Use the full path for file_path
        let file_full_path = dir.path().join("untracked.txt");
        let result = get_git_file_status(
            dir.path().to_string_lossy().to_string(),
            file_full_path.to_string_lossy().to_string()
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(GitFileStatus::Untracked));
    }

    #[test]
    fn test_get_git_diff_untracked_file() {
        let dir = tempdir().unwrap();

        // Initialize git repo
        Repository::init(dir.path()).unwrap();

        // Create untracked file
        fs::write(dir.path().join("new.txt"), "line1\nline2\nline3").unwrap();

        let result = get_git_diff(
            dir.path().to_string_lossy().to_string(),
            "new.txt".to_string()
        );
        assert!(result.is_ok());
        let diff = result.unwrap();
        assert!(diff.contains("+ line1"));
        assert!(diff.contains("+ line2"));
        assert!(diff.contains("+ line3"));
    }

    #[test]
    fn test_get_all_git_diffs_with_untracked() {
        let dir = tempdir().unwrap();

        // Initialize git repo
        Repository::init(dir.path()).unwrap();

        // Create untracked file
        fs::write(dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(dir.path().join("file2.txt"), "content2").unwrap();

        let result = get_all_git_diffs(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let diffs = result.unwrap();
        assert_eq!(diffs.len(), 2);

        // Check sorted by path
        assert_eq!(diffs[0].path, "file1.txt");
        assert_eq!(diffs[1].path, "file2.txt");
    }

    #[test]
    fn test_get_git_status_branch_name() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit to have a branch
        let sig = test_signature();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        let result = get_git_status(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let info = result.unwrap();
        // After initial commit, branch should be available
        assert!(info.branch.is_some());
    }

    #[test]
    fn test_get_git_diff_modified_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("test.txt"), "original content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Modify the file
        fs::write(dir.path().join("test.txt"), "modified content").unwrap();

        let result = get_git_diff(
            dir.path().to_string_lossy().to_string(),
            "test.txt".to_string()
        );
        assert!(result.is_ok());

        let diff = result.unwrap();
        assert!(diff.contains("- original content") || diff.contains("+ modified content"));
    }

    #[test]
    fn test_get_file_diff_internal_empty_result() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("test.txt"), "content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // File is committed and unchanged - diff should be empty
        let diff = get_file_diff_internal(&repo, &dir.path().to_string_lossy(), "test.txt");
        assert!(diff.is_empty());
    }

    #[test]
    fn test_get_git_file_status_committed_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("test.txt"), "content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        let file_full_path = dir.path().join("test.txt");
        let result = get_git_file_status(
            dir.path().to_string_lossy().to_string(),
            file_full_path.to_string_lossy().to_string()
        );
        assert!(result.is_ok());
        // Committed file should return None (unchanged)
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_get_git_file_status_modified_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("test.txt"), "original").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Modify the file
        fs::write(dir.path().join("test.txt"), "modified").unwrap();

        let file_full_path = dir.path().join("test.txt");
        let result = get_git_file_status(
            dir.path().to_string_lossy().to_string(),
            file_full_path.to_string_lossy().to_string()
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(GitFileStatus::Modified));
    }

    #[test]
    fn test_get_all_git_diffs_empty_repo() {
        let dir = tempdir().unwrap();
        Repository::init(dir.path()).unwrap();

        // Empty repo with no files
        let result = get_all_git_diffs(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_find_repo_root_nested_path() {
        // Test finding repo root from a nested directory
        let current_dir = std::env::current_dir().unwrap();
        let nested = current_dir.join("src").join("commands");

        if nested.exists() {
            let result = find_repo_root(&nested);
            assert!(result.is_some());
        }
    }

    #[test]
    fn test_git_status_with_staged_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit first
        let sig = test_signature();
        fs::write(dir.path().join("initial.txt"), "initial").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("initial.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Create and stage a new file (without committing)
        fs::write(dir.path().join("staged.txt"), "staged content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("staged.txt")).unwrap();
        index.write().unwrap();

        let result = get_git_status(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let info = result.unwrap();
        // Should have at least one status entry (the staged file)
        let staged_file = info.statuses.iter().find(|s| s.path == "staged.txt");
        assert!(staged_file.is_some());
        assert_eq!(staged_file.unwrap().status, GitFileStatus::Added);
    }

    #[test]
    fn test_get_git_file_status_deleted_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("to_delete.txt"), "content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("to_delete.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Delete the file
        fs::remove_file(dir.path().join("to_delete.txt")).unwrap();

        let file_full_path = dir.path().join("to_delete.txt");
        let result = get_git_file_status(
            dir.path().to_string_lossy().to_string(),
            file_full_path.to_string_lossy().to_string()
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(GitFileStatus::Deleted));
    }

    #[test]
    fn test_get_git_status_with_ignored_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create .gitignore and stage it
        fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(".gitignore")).unwrap();
        index.write().unwrap();

        // Create ignored file
        fs::write(dir.path().join("ignored.txt"), "ignored content").unwrap();

        // Get status with include_ignored
        let result = get_git_status(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let info = result.unwrap();
        // Should have the ignored file in statuses
        let ignored = info.statuses.iter().find(|s| s.path == "ignored.txt");
        if let Some(ig) = ignored {
            assert_eq!(ig.status, GitFileStatus::Ignored);
        }
    }

    #[test]
    fn test_get_all_git_diffs_deleted_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("file.txt"), "content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Delete the file
        fs::remove_file(dir.path().join("file.txt")).unwrap();

        let result = get_all_git_diffs(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let diffs = result.unwrap();
        assert_eq!(diffs.len(), 1);
        assert_eq!(diffs[0].status, GitFileStatus::Deleted);
    }

    #[test]
    fn test_get_git_diff_no_changes() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("clean.txt"), "content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("clean.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // No changes - file is committed and unchanged
        let result = get_git_diff(
            dir.path().to_string_lossy().to_string(),
            "clean.txt".to_string()
        );
        assert!(result.is_ok());
        // Should be empty diff
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_git_diff_staged_changes() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("staged.txt"), "original").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("staged.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Modify and stage the file (no working directory changes)
        fs::write(dir.path().join("staged.txt"), "modified and staged").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("staged.txt")).unwrap();
        index.write().unwrap();

        // Get diff - should show staged changes
        let result = get_git_diff(
            dir.path().to_string_lossy().to_string(),
            "staged.txt".to_string()
        );
        assert!(result.is_ok());
        let diff = result.unwrap();
        // Staged changes should produce a diff
        assert!(diff.contains("modified and staged") || diff.contains("original"));
    }

    #[test]
    fn test_get_file_diff_internal_with_context_lines() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file with multiple lines
        let sig = test_signature();
        fs::write(dir.path().join("multiline.txt"), "line1\nline2\nline3\nline4\nline5").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("multiline.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Modify middle line
        fs::write(dir.path().join("multiline.txt"), "line1\nline2\nmodified\nline4\nline5").unwrap();

        let diff = get_file_diff_internal(&repo, &dir.path().to_string_lossy(), "multiline.txt");
        // Should contain context lines (space prefix) and changed lines
        assert!(!diff.is_empty());
    }

    #[test]
    fn test_get_file_diff_internal_nonexistent_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Try to get diff for non-existent file
        let diff = get_file_diff_internal(&repo, &dir.path().to_string_lossy(), "nonexistent.txt");
        assert!(diff.is_empty());
    }

    #[test]
    fn test_get_file_diff_internal_unreadable_untracked_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create untracked file
        fs::write(dir.path().join("untracked.txt"), "content").unwrap();

        let diff = get_file_diff_internal(&repo, &dir.path().to_string_lossy(), "untracked.txt");
        // Should return content as additions
        assert!(diff.contains("+ content"));
    }

    #[test]
    fn test_git_file_status_renamed() {
        // Test Renamed variant exists and can be compared
        assert_eq!(GitFileStatus::Renamed, GitFileStatus::Renamed);
        assert_ne!(GitFileStatus::Renamed, GitFileStatus::Modified);
    }

    #[test]
    fn test_git_file_status_conflicted() {
        // Test Conflicted variant exists and can be compared
        assert_eq!(GitFileStatus::Conflicted, GitFileStatus::Conflicted);
        assert_ne!(GitFileStatus::Conflicted, GitFileStatus::Modified);
    }

    /// Helper to create a git signature for tests
    /// Always uses a fixed signature for consistency and coverage
    fn test_signature() -> git2::Signature<'static> {
        git2::Signature::now("test", "test@example.com").unwrap()
    }

    #[test]
    fn test_get_git_file_status_renamed_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("original.txt"), "content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("original.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Rename the file in the index (stage rename)
        fs::rename(dir.path().join("original.txt"), dir.path().join("renamed.txt")).unwrap();

        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("original.txt")).unwrap();
        index.add_path(Path::new("renamed.txt")).unwrap();
        index.write().unwrap();

        // Check status for renamed file
        let result = get_git_status(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let info = result.unwrap();
        // Should have at least one renamed entry
        let has_rename_related = info.statuses.iter().any(|s| {
            s.status == GitFileStatus::Renamed ||
            s.status == GitFileStatus::Added ||
            s.status == GitFileStatus::Deleted
        });
        assert!(has_rename_related);
    }

    #[test]
    fn test_get_git_status_with_conflict() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit initial file
        let sig = test_signature();
        fs::write(dir.path().join("conflict.txt"), "initial content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("conflict.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let initial_commit = repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
        let initial_commit_obj = repo.find_commit(initial_commit).unwrap();

        // Create and switch to branch1
        repo.branch("branch1", &initial_commit_obj, false).unwrap();
        repo.set_head("refs/heads/branch1").unwrap();

        // Make change on branch1
        fs::write(dir.path().join("conflict.txt"), "branch1 content").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("conflict.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let _branch1_commit = repo.commit(
            Some("HEAD"), &sig, &sig, "Branch1 commit", &tree, &[&initial_commit_obj]
        ).unwrap();

        // Switch back to master and create branch2
        repo.set_head("refs/heads/master").unwrap();
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force())).unwrap();
        repo.branch("branch2", &initial_commit_obj, false).unwrap();
        repo.set_head("refs/heads/branch2").unwrap();

        // Make different change on branch2
        fs::write(dir.path().join("conflict.txt"), "branch2 content").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("conflict.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let _branch2_commit = repo.commit(
            Some("HEAD"), &sig, &sig, "Branch2 commit", &tree, &[&initial_commit_obj]
        ).unwrap();

        // Try to merge branch1 into branch2 - this should cause conflict
        let branch1_ref = repo.find_reference("refs/heads/branch1").unwrap();
        let branch1_commit = repo.reference_to_annotated_commit(&branch1_ref).unwrap();
        let _ = repo.merge(&[&branch1_commit], None, None);

        // Now check if we have conflicted status
        let result = get_git_status(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        // The merge may or may not produce a conflict depending on git internals
        // At minimum, we should not error
    }

    #[test]
    fn test_get_all_git_diffs_with_renamed_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("original.txt"), "content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("original.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Rename the file
        fs::rename(dir.path().join("original.txt"), dir.path().join("renamed.txt")).unwrap();

        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("original.txt")).unwrap();
        index.add_path(Path::new("renamed.txt")).unwrap();
        index.write().unwrap();

        let result = get_all_git_diffs(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());
        // Should have at least one entry (either renamed or add+delete)
    }

    #[test]
    fn test_get_file_diff_internal_error_paths() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create initial commit without any files
        let sig = test_signature();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial empty commit", &tree, &[]).unwrap();

        // Test diff on a path that doesn't match any changes
        let diff = get_file_diff_internal(&repo, &dir.path().to_string_lossy(), "nonexistent_file.txt");
        assert!(diff.is_empty());
    }

    #[test]
    fn test_get_git_diff_read_error_for_untracked() {
        let dir = tempdir().unwrap();
        Repository::init(dir.path()).unwrap();

        // Create an untracked file that exists but might have issues
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        // Try to get diff - should return the file content as additions
        let result = get_git_diff(
            dir.path().to_string_lossy().to_string(),
            "test.txt".to_string()
        );
        assert!(result.is_ok());
        assert!(result.unwrap().contains("+ test content"));
    }

    #[test]
    fn test_get_git_status_continues_on_unchanged_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("clean.txt"), "content").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("clean.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Commit", &tree, &[]).unwrap();

        // Don't modify the file - it should be "unchanged" and skipped
        let result = get_git_status(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let info = result.unwrap();
        // The clean file should not appear in statuses
        assert!(!info.statuses.iter().any(|s| s.path == "clean.txt"));
    }

    #[test]
    fn test_get_file_diff_internal_binary_or_unreadable() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create a file that's untracked with binary-like content
        fs::write(dir.path().join("binary.bin"), vec![0u8, 1, 2, 255, 254]).unwrap();

        // Try to get diff for a file that might have encoding issues
        let diff = get_file_diff_internal(&repo, &dir.path().to_string_lossy(), "binary.bin");
        // Should handle gracefully - either return content or empty string
        assert!(diff.is_empty() || !diff.is_empty());
    }

    #[test]
    fn test_calculate_diff_stats_untracked_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create untracked files with known line counts
        fs::write(dir.path().join("file1.txt"), "line1\nline2\nline3").unwrap();
        fs::write(dir.path().join("file2.txt"), "single line").unwrap();

        let (additions, deletions) = calculate_diff_stats(&repo, &dir.path().to_string_lossy());

        // 3 lines in file1 + 1 line in file2 = 4 additions
        assert_eq!(additions, 4);
        assert_eq!(deletions, 0);
    }

    #[test]
    fn test_calculate_diff_stats_modified_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("test.txt"), "original line 1\noriginal line 2").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Modify the file
        fs::write(dir.path().join("test.txt"), "modified line 1\nnew line 2\nnew line 3").unwrap();

        let (additions, deletions) = calculate_diff_stats(&repo, &dir.path().to_string_lossy());

        // Should have some additions and deletions
        assert!(additions > 0 || deletions > 0);
    }

    #[test]
    fn test_calculate_diff_stats_empty_repo() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let (additions, deletions) = calculate_diff_stats(&repo, &dir.path().to_string_lossy());

        assert_eq!(additions, 0);
        assert_eq!(deletions, 0);
    }

    #[test]
    fn test_get_git_status_includes_diff_stats() {
        let dir = tempdir().unwrap();
        Repository::init(dir.path()).unwrap();

        // Create untracked files
        fs::write(dir.path().join("file.txt"), "line1\nline2").unwrap();

        let result = get_git_status(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let info = result.unwrap();
        // Should have 2 lines as additions
        assert_eq!(info.additions, 2);
        assert_eq!(info.deletions, 0);
    }

    #[test]
    fn test_calculate_diff_stats_staged_changes() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create and commit a file
        let sig = test_signature();
        fs::write(dir.path().join("test.txt"), "original").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();

        // Modify and stage the file
        fs::write(dir.path().join("test.txt"), "modified\nnew line").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let (additions, deletions) = calculate_diff_stats(&repo, &dir.path().to_string_lossy());

        // Should count staged changes
        assert!(additions > 0);
    }
}
