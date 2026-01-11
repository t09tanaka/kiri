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
        .include_ignored(true);

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

fn get_file_diff_internal(repo: &Repository, repo_path: &str, file_path: &str) -> String {
    let relative_path = Path::new(file_path);
    let file_status = repo.status_file(relative_path).ok();

    // For untracked files, return the entire file content
    if let Some(status) = file_status {
        if status.is_wt_new() {
            let full_path = Path::new(repo_path).join(file_path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                return content
                    .lines()
                    .map(|line| format!("+ {}", line))
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            return String::new();
        }
    }

    // Get diff between HEAD and working directory for the specific file
    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(file_path);

    let diff = match repo.diff_index_to_workdir(None, Some(&mut diff_opts)) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };

    // If no working directory changes, check index changes (staged)
    let diff = if diff.deltas().len() == 0 {
        let head = match repo.head() {
            Ok(h) => h,
            Err(_) => return String::new(),
        };
        let head_tree = match head.peel_to_tree() {
            Ok(t) => t,
            Err(_) => return String::new(),
        };
        match repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut diff_opts)) {
            Ok(d) => d,
            Err(_) => return String::new(),
        }
    } else {
        diff
    };

    // Convert diff to string
    let mut diff_text = String::new();
    let _ = diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
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
    });

    diff_text
}

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

        let diff = get_file_diff_internal(&repo, &repo_path, &path);

        diffs.push(GitFileDiff {
            path,
            status: file_status,
            diff,
        });
    }

    // Sort by path alphabetically
    diffs.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(diffs)
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
