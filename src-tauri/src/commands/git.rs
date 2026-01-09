use git2::{Repository, StatusOptions};
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
pub struct GitRepoInfo {
    pub root: String,
    pub branch: Option<String>,
    pub statuses: Vec<GitStatusEntry>,
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
        .include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts)).map_err(|e| e.to_string())?;

    let mut entries: Vec<GitStatusEntry> = Vec::new();

    for entry in statuses.iter() {
        let path = entry.path().unwrap_or("").to_string();
        let status = entry.status();

        let file_status = if status.is_index_new() {
            GitFileStatus::Added
        } else if status.is_wt_new() {
            GitFileStatus::Untracked
        } else if status.is_index_modified() || status.is_wt_modified() {
            GitFileStatus::Modified
        } else if status.is_index_deleted() || status.is_wt_deleted() {
            GitFileStatus::Deleted
        } else if status.is_index_renamed() || status.is_wt_renamed() {
            GitFileStatus::Renamed
        } else if status.is_conflicted() {
            GitFileStatus::Conflicted
        } else if status.is_ignored() {
            GitFileStatus::Ignored
        } else {
            continue; // Skip unchanged files
        };

        entries.push(GitStatusEntry {
            path,
            status: file_status,
        });
    }

    Ok(GitRepoInfo {
        root: repo_root,
        branch,
        statuses: entries,
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

    let file_status = if status.is_index_new() {
        Some(GitFileStatus::Added)
    } else if status.is_wt_new() {
        Some(GitFileStatus::Untracked)
    } else if status.is_index_modified() || status.is_wt_modified() {
        Some(GitFileStatus::Modified)
    } else if status.is_index_deleted() || status.is_wt_deleted() {
        Some(GitFileStatus::Deleted)
    } else if status.is_index_renamed() || status.is_wt_renamed() {
        Some(GitFileStatus::Renamed)
    } else if status.is_conflicted() {
        Some(GitFileStatus::Conflicted)
    } else {
        None
    };

    Ok(file_status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_repo_root() {
        // This test assumes we're running from within a git repo
        let current_dir = std::env::current_dir().unwrap();
        let result = find_repo_root(&current_dir);
        assert!(result.is_some());
    }
}
