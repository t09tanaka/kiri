use git2::{Oid, Repository, Sort};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    pub id: String,
    pub full_hash: String,
    pub message: String,
    pub message_body: String,
    pub author: String,
    pub author_email: String,
    pub date: i64,
    pub parent_ids: Vec<String>,
    pub is_pushed: bool,
    pub branch_type: String,
    pub graph_column: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitFileDiff {
    pub path: String,
    pub status: String,
    pub diff: String,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitDiffResult {
    pub commit: CommitInfo,
    pub files: Vec<CommitFileDiff>,
    pub total_additions: usize,
    pub total_deletions: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PushResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FetchResult {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BehindAheadCount {
    pub behind: usize,
    pub ahead: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PullResult {
    pub success: bool,
    pub message: String,
}

/// Build a CommitInfo from a git2::Commit
fn build_commit_info(
    commit: &git2::Commit,
    is_pushed: bool,
    branch_type: &str,
    graph_column: u32,
) -> CommitInfo {
    let id = format!("{:.7}", commit.id());
    let full_hash = commit.id().to_string();
    let message = commit
        .summary()
        .unwrap_or("")
        .to_string();
    let message_body = commit
        .message()
        .unwrap_or("")
        .to_string();
    let author = commit.author().name().unwrap_or("").to_string();
    let author_email = commit.author().email().unwrap_or("").to_string();
    let date = commit.time().seconds();
    let parent_ids = commit
        .parent_ids()
        .map(|oid| oid.to_string())
        .collect();

    CommitInfo {
        id,
        full_hash,
        message,
        message_body,
        author,
        author_email,
        date,
        parent_ids,
        is_pushed,
        branch_type: branch_type.to_string(),
        graph_column,
    }
}

/// Detect the default branch name for a repository.
/// Checks origin/HEAD first, then falls back to main or master.
fn detect_default_branch(repo: &Repository) -> Option<String> {
    // Try origin/HEAD
    if let Ok(reference) = repo.find_reference("refs/remotes/origin/HEAD") {
        if let Ok(resolved) = reference.resolve() {
            if let Some(name) = resolved.shorthand() {
                if let Some(branch) = name.strip_prefix("origin/") {
                    return Some(branch.to_string());
                }
            }
        }
    }

    // Fallback: check if main or master exists locally
    if repo
        .find_branch("main", git2::BranchType::Local)
        .is_ok()
    {
        return Some("main".to_string());
    }

    if repo
        .find_branch("master", git2::BranchType::Local)
        .is_ok()
    {
        return Some("master".to_string());
    }

    None
}

/// Get commit log for a repository
pub fn get_commit_log(
    repo_path: String,
    max_count: Option<usize>,
    skip: Option<usize>,
) -> Result<Vec<CommitInfo>, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;
    let max_count = max_count.unwrap_or(50);
    let skip = skip.unwrap_or(0);

    // Get HEAD reference
    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => return Ok(Vec::new()), // Empty repo
    };

    let head_oid = head
        .target()
        .ok_or_else(|| "HEAD has no target".to_string())?;

    // Build set of pushed OIDs by walking from the upstream tracking branch
    let mut pushed_oids = HashSet::new();

    // Try to find upstream tracking ref for the current branch,
    // falling back to origin's default branch (e.g., origin/main)
    let upstream_oid = head
        .shorthand()
        .and_then(|branch_name| {
            if branch_name == "HEAD" {
                return None;
            }
            let remote_ref = format!("refs/remotes/origin/{}", branch_name);
            repo.find_reference(&remote_ref)
                .ok()
                .and_then(|r| r.target())
        })
        .or_else(|| {
            // Fallback: use origin's default branch tracking ref
            detect_default_branch(&repo).and_then(|name| {
                let remote_ref = format!("refs/remotes/origin/{}", name);
                repo.find_reference(&remote_ref)
                    .ok()
                    .and_then(|r| r.target())
            })
        });

    if let Some(oid) = upstream_oid {
        if let Ok(mut revwalk) = repo.revwalk() {
            let _ = revwalk.push(oid);
            revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME).ok();
            for oid in revwalk.flatten() {
                pushed_oids.insert(oid);
            }
        }
    }

    // Detect default branch and its OID
    let default_branch_name = detect_default_branch(&repo);
    let default_branch_oid: Option<Oid> = default_branch_name.as_ref().and_then(|name| {
        repo.find_branch(name, git2::BranchType::Local)
            .ok()
            .and_then(|b| b.get().target())
    });

    // Compute merge-base between HEAD and default branch
    let merge_base_oid: Option<Oid> = default_branch_oid.and_then(|default_oid| {
        if default_oid == head_oid {
            return None;
        }
        repo.merge_base(head_oid, default_oid).ok()
    });

    // Build sets of commits exclusive to each branch (between tip and merge-base)
    let mut head_only_commits = HashSet::new();
    let mut default_only_commits = HashSet::new();

    if let Some(base_oid) = merge_base_oid {
        // Commits reachable from HEAD but not from merge-base
        if let Ok(mut walk) = repo.revwalk() {
            let _ = walk.push(head_oid);
            let _ = walk.hide(base_oid);
            walk.set_sorting(Sort::TOPOLOGICAL).ok();
            for oid in walk.flatten() {
                head_only_commits.insert(oid);
            }
        }

        // Commits reachable from default branch but not from merge-base
        if let Some(default_oid) = default_branch_oid {
            if let Ok(mut walk) = repo.revwalk() {
                let _ = walk.push(default_oid);
                let _ = walk.hide(base_oid);
                walk.set_sorting(Sort::TOPOLOGICAL).ok();
                for oid in walk.flatten() {
                    default_only_commits.insert(oid);
                }
            }
        }
    }

    // Revwalk from HEAD (and default branch if diverged)
    let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;
    revwalk.push(head_oid).map_err(|e| e.to_string())?;

    // Also include default branch commits in the walk
    if let (Some(_), Some(default_oid)) = (merge_base_oid, default_branch_oid) {
        if default_oid != head_oid {
            let _ = revwalk.push(default_oid);
        }
    }

    revwalk
        .set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
        .map_err(|e| e.to_string())?;

    let mut commits = Vec::new();
    for (count, oid_result) in revwalk.enumerate() {
        if count < skip {
            // Skip commits for pagination
            let _ = oid_result.map_err(|e| e.to_string())?;
            continue;
        }
        if count >= skip + max_count {
            break;
        }

        let oid = oid_result.map_err(|e| e.to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;

        let is_pushed = pushed_oids.contains(&oid);

        // Determine branch_type and graph_column using pre-computed sets
        let has_divergence = !head_only_commits.is_empty() || !default_only_commits.is_empty();
        let is_merge_base = merge_base_oid == Some(oid);
        let main_has_advanced = !default_only_commits.is_empty();
        let (branch_type, graph_column) = if is_merge_base && main_has_advanced {
            // True fork point: both branches have diverged
            ("both".to_string(), 0)
        } else if is_merge_base {
            // merge-base == main HEAD: main hasn't advanced, this is just a main commit
            ("shared".to_string(), 0)
        } else if head_only_commits.contains(&oid) {
            // Current branch exclusive commits branch off to the right
            ("current".to_string(), 1)
        } else if default_only_commits.contains(&oid) {
            ("base".to_string(), 1)
        } else if has_divergence {
            // Shared history before merge-base (on a diverged branch)
            ("shared".to_string(), 0)
        } else {
            // No divergence (e.g., on main) - all commits are current
            ("current".to_string(), 0)
        };

        commits.push(build_commit_info(
            &commit,
            is_pushed,
            &branch_type,
            graph_column,
        ));
    }

    Ok(commits)
}

/// Get diff details for a specific commit
pub fn get_commit_diff(
    repo_path: String,
    commit_hash: String,
) -> Result<CommitDiffResult, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;
    let oid = Oid::from_str(&commit_hash).map_err(|e| e.to_string())?;
    let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
    let commit_tree = commit.tree().map_err(|e| e.to_string())?;

    // Get parent tree (or empty tree for root commit)
    let parent_tree = if commit.parent_count() > 0 {
        let parent = commit.parent(0).map_err(|e| e.to_string())?;
        Some(parent.tree().map_err(|e| e.to_string())?)
    } else {
        None
    };

    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&commit_tree), None)
        .map_err(|e| e.to_string())?;

    let mut files: Vec<CommitFileDiff> = Vec::new();
    let mut total_additions: usize = 0;
    let mut total_deletions: usize = 0;

    // Iterate over each delta/patch
    let num_deltas = diff.deltas().len();
    for i in 0..num_deltas {
        if let Ok(Some(mut patch)) = git2::Patch::from_diff(&diff, i) {
            let delta = patch.delta();

            let path = delta
                .new_file()
                .path()
                .or_else(|| delta.old_file().path())
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();

            let status = match delta.status() {
                git2::Delta::Added => "Added",
                git2::Delta::Modified => "Modified",
                git2::Delta::Deleted => "Deleted",
                git2::Delta::Renamed => "Renamed",
                _ => "Modified",
            }
            .to_string();

            // Count additions and deletions
            let (_, adds, dels) = patch.line_stats().unwrap_or((0, 0, 0));

            // Get unified diff text
            let mut diff_text = String::new();
            let mut cb =
                |_delta: git2::DiffDelta<'_>,
                 _hunk: Option<git2::DiffHunk<'_>>,
                 line: git2::DiffLine<'_>|
                 -> bool {
                    let prefix = match line.origin() {
                        '+' => "+ ",
                        '-' => "- ",
                        ' ' => "  ",
                        'H' => "",
                        _ => "",
                    };
                    if let Ok(content) = std::str::from_utf8(line.content()) {
                        if line.origin() == 'H' {
                            diff_text.push_str(content);
                        } else if !prefix.is_empty() {
                            diff_text.push_str(prefix);
                            diff_text.push_str(content);
                        }
                    }
                    true
                };
            let _ = patch.print(&mut cb);

            total_additions += adds;
            total_deletions += dels;

            files.push(CommitFileDiff {
                path,
                status,
                diff: diff_text,
                additions: adds,
                deletions: dels,
            });
        }
    }

    let commit_info = build_commit_info(&commit, false, "current", 0);

    Ok(CommitDiffResult {
        commit: commit_info,
        files,
        total_additions,
        total_deletions,
    })
}

/// Get the number of commits on the current branch ahead of the default branch (main/master).
/// Returns 0 if on the default branch itself or if no default branch is found.
pub fn get_branch_ahead_count(repo_path: String) -> Result<usize, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;

    let head = repo.head().map_err(|e| e.to_string())?;
    let head_oid = head
        .target()
        .ok_or_else(|| "HEAD has no target".to_string())?;

    let default_branch_name = match detect_default_branch(&repo) {
        Some(name) => name,
        None => return Ok(0),
    };

    // If currently on the default branch, return 0
    if let Some(branch_name) = head.shorthand() {
        if branch_name == default_branch_name {
            return Ok(0);
        }
    }

    let default_oid = repo
        .find_branch(&default_branch_name, git2::BranchType::Local)
        .map_err(|e| e.to_string())?
        .get()
        .target()
        .ok_or_else(|| "Default branch has no target".to_string())?;

    if head_oid == default_oid {
        return Ok(0);
    }

    let merge_base = repo
        .merge_base(head_oid, default_oid)
        .map_err(|e| e.to_string())?;

    // Count commits from merge-base to HEAD
    let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;
    revwalk.push(head_oid).map_err(|e| e.to_string())?;
    revwalk.hide(merge_base).map_err(|e| e.to_string())?;
    revwalk
        .set_sorting(Sort::TOPOLOGICAL)
        .map_err(|e| e.to_string())?;

    let count = revwalk.count();
    Ok(count)
}

/// Fetch from remote using git command
pub fn fetch_remote(repo_path: String, remote: Option<String>) -> Result<FetchResult, String> {
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());

    let output = std::process::Command::new("git")
        .args(["fetch", &remote_name])
        .current_dir(&repo_path)
        // Clear inherited GIT_DIR/GIT_WORK_TREE so git operates on the
        // target repo_path, not the parent worktree (e.g. during pre-commit hooks).
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .map_err(|e| format!("Failed to execute git fetch: {}", e))?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(FetchResult {
        success: output.status.success(),
        message: stderr,
    })
}

/// Get behind/ahead count relative to upstream tracking branch
pub fn get_behind_ahead_count(repo_path: String) -> Result<BehindAheadCount, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;

    let head = match repo.head() {
        Ok(h) => h,
        Err(_) => {
            return Ok(BehindAheadCount {
                behind: 0,
                ahead: 0,
            })
        }
    };

    let head_oid = match head.target() {
        Some(oid) => oid,
        None => {
            return Ok(BehindAheadCount {
                behind: 0,
                ahead: 0,
            })
        }
    };

    // Find upstream tracking ref for the current branch
    let upstream_oid = head.shorthand().and_then(|branch_name| {
        if branch_name == "HEAD" {
            return None;
        }
        let remote_ref = format!("refs/remotes/origin/{}", branch_name);
        repo.find_reference(&remote_ref)
            .ok()
            .and_then(|r| r.target())
    });

    match upstream_oid {
        Some(upstream) => {
            let (ahead, behind) = repo
                .graph_ahead_behind(head_oid, upstream)
                .map_err(|e| e.to_string())?;
            Ok(BehindAheadCount { behind, ahead })
        }
        None => Ok(BehindAheadCount {
            behind: 0,
            ahead: 0,
        }),
    }
}

/// Pull commits from remote using git command
pub fn pull_commits(
    repo_path: String,
    remote: Option<String>,
    branch: Option<String>,
) -> Result<PullResult, String> {
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());

    let branch_name = match branch {
        Some(b) => b,
        None => {
            let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;
            let head = repo.head().map_err(|e| e.to_string())?;
            head.shorthand().unwrap_or("HEAD").to_string()
        }
    };

    let output = std::process::Command::new("git")
        .args(["pull", &remote_name, &branch_name])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("Failed to execute git pull: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(PullResult {
            success: true,
            message: if stderr.is_empty() {
                stdout
            } else {
                format!("{}{}", stdout, stderr)
            },
        })
    } else {
        Ok(PullResult {
            success: false,
            message: if stderr.is_empty() {
                stdout
            } else {
                stderr
            },
        })
    }
}

/// Push commits to remote using git command
pub fn push_commits(
    repo_path: String,
    remote: Option<String>,
    branch: Option<String>,
) -> Result<PushResult, String> {
    let remote_name = remote.unwrap_or_else(|| "origin".to_string());

    let branch_name = match branch {
        Some(b) => b,
        None => {
            let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;
            let head = repo.head().map_err(|e| e.to_string())?;
            head.shorthand()
                .unwrap_or("HEAD")
                .to_string()
        }
    };

    let output = std::process::Command::new("git")
        .args(["push", &remote_name, &branch_name])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("Failed to execute git push: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(PushResult {
            success: true,
            message: if stderr.is_empty() {
                stdout
            } else {
                // git push often writes progress to stderr even on success
                format!("{}{}", stdout, stderr)
            },
        })
    } else {
        Ok(PushResult {
            success: false,
            message: if stderr.is_empty() {
                stdout
            } else {
                stderr
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::tempdir;

    fn test_signature() -> git2::Signature<'static> {
        git2::Signature::now("test", "test@example.com").unwrap()
    }

    fn create_repo_with_commit(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        let sig = test_signature();

        fs::write(dir.join("README.md"), "# Test").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        repo
    }

    fn add_commit(repo: &Repository, dir: &Path, filename: &str, content: &str, message: &str) -> Oid {
        fs::write(dir.join(filename), content).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(filename)).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let sig = test_signature();

        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
            .unwrap()
    }

    #[test]
    fn test_get_commit_log_basic() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Add a second commit
        add_commit(&repo, dir.path(), "file1.txt", "hello", "Add file1");

        let result = get_commit_log(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());

        let commits = result.unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "Add file1");
        assert_eq!(commits[1].message, "Initial commit");
    }

    #[test]
    fn test_get_commit_log_empty_repo() {
        let dir = tempdir().unwrap();
        Repository::init(dir.path()).unwrap();

        let result = get_commit_log(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_commit_log_max_count() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Add several commits
        for i in 0..5 {
            add_commit(
                &repo,
                dir.path(),
                &format!("file{}.txt", i),
                &format!("content {}", i),
                &format!("Commit {}", i),
            );
        }

        let result = get_commit_log(dir.path().to_string_lossy().to_string(), Some(3), None);
        assert!(result.is_ok());

        let commits = result.unwrap();
        assert_eq!(commits.len(), 3);
    }

    #[test]
    fn test_get_commit_log_skip() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Add several commits
        for i in 0..5 {
            add_commit(
                &repo,
                dir.path(),
                &format!("file{}.txt", i),
                &format!("content {}", i),
                &format!("Commit {}", i),
            );
        }

        // Total 6 commits (Initial + 5). Skip first 2, take 3
        let result = get_commit_log(dir.path().to_string_lossy().to_string(), Some(3), Some(2));
        assert!(result.is_ok());

        let commits = result.unwrap();
        assert_eq!(commits.len(), 3);
        // Commits are in reverse chronological order, so skip 2 means skip "Commit 4" and "Commit 3"
        assert_eq!(commits[0].message, "Commit 2");
        assert_eq!(commits[1].message, "Commit 1");
        assert_eq!(commits[2].message, "Commit 0");
    }

    #[test]
    fn test_get_commit_log_skip_beyond_end() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        // Only 1 commit, skip 5
        let result = get_commit_log(dir.path().to_string_lossy().to_string(), Some(10), Some(5));
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_commit_diff_added_file() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let oid = add_commit(&repo, dir.path(), "new_file.txt", "new content\n", "Add new file");

        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            oid.to_string(),
        );
        assert!(result.is_ok());

        let diff_result = result.unwrap();
        assert_eq!(diff_result.commit.message, "Add new file");
        assert!(!diff_result.files.is_empty());

        let file_diff = diff_result.files.iter().find(|f| f.path == "new_file.txt").unwrap();
        assert_eq!(file_diff.status, "Added");
        assert!(file_diff.additions > 0);
    }

    #[test]
    fn test_get_commit_diff_modified_file() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Modify existing file
        let oid = add_commit(&repo, dir.path(), "README.md", "# Updated Test\n", "Update README");

        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            oid.to_string(),
        );
        assert!(result.is_ok());

        let diff_result = result.unwrap();
        let file_diff = diff_result.files.iter().find(|f| f.path == "README.md").unwrap();
        assert_eq!(file_diff.status, "Modified");
    }

    #[test]
    fn test_get_commit_diff_deleted_file() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Add a file first
        add_commit(&repo, dir.path(), "to_delete.txt", "delete me\n", "Add file to delete");

        // Delete the file
        fs::remove_file(dir.path().join("to_delete.txt")).unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("to_delete.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let sig = test_signature();
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Delete file", &tree, &[&head])
            .unwrap();

        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            oid.to_string(),
        );
        assert!(result.is_ok());

        let diff_result = result.unwrap();
        let file_diff = diff_result.files.iter().find(|f| f.path == "to_delete.txt").unwrap();
        assert_eq!(file_diff.status, "Deleted");
        assert!(file_diff.deletions > 0);
    }

    #[test]
    fn test_get_commit_diff_root_commit() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Get the root commit hash
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let oid = head.id();

        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            oid.to_string(),
        );
        assert!(result.is_ok());

        let diff_result = result.unwrap();
        assert_eq!(diff_result.commit.message, "Initial commit");
        assert!(!diff_result.files.is_empty());

        let file_diff = diff_result.files.iter().find(|f| f.path == "README.md").unwrap();
        assert_eq!(file_diff.status, "Added");
    }

    #[test]
    fn test_push_result_serialization() {
        let result = PushResult {
            success: true,
            message: "Everything up-to-date".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("Everything up-to-date"));
    }

    #[test]
    fn test_commit_info_serialization() {
        let info = CommitInfo {
            id: "abc1234".to_string(),
            full_hash: "abc1234567890abcdef1234567890abcdef123456".to_string(),
            message: "Test commit".to_string(),
            message_body: "Test commit\n\nDetailed description".to_string(),
            author: "Test Author".to_string(),
            author_email: "test@example.com".to_string(),
            date: 1704067200,
            parent_ids: vec!["def5678".to_string()],
            is_pushed: true,
            branch_type: "current".to_string(),
            graph_column: 0,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"id\":\"abc1234\""));
        assert!(json.contains("\"is_pushed\":true"));
        assert!(json.contains("\"branch_type\":\"current\""));
    }

    #[test]
    fn test_commit_file_diff_serialization() {
        let diff = CommitFileDiff {
            path: "src/main.rs".to_string(),
            status: "Modified".to_string(),
            diff: "+ new line\n- old line\n".to_string(),
            additions: 1,
            deletions: 1,
        };
        let json = serde_json::to_string(&diff).unwrap();
        assert!(json.contains("\"status\":\"Modified\""));
        assert!(json.contains("\"additions\":1"));
    }

    #[test]
    fn test_merge_commit_with_multiple_parents() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let sig = test_signature();

        // Create a branch
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature", &head, false).unwrap();

        // Add a commit on main
        let main_oid = add_commit(&repo, dir.path(), "main_file.txt", "main\n", "Main commit");

        // Checkout feature and add commit
        let feature_branch = repo.find_branch("feature", git2::BranchType::Local).unwrap();
        let feature_commit = feature_branch.get().peel_to_commit().unwrap();

        // Reset index to feature branch tree
        repo.checkout_tree(
            feature_commit.tree().unwrap().as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        )
        .unwrap();
        repo.set_head("refs/heads/feature").unwrap();

        fs::write(dir.path().join("feature_file.txt"), "feature\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("feature_file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let feature_head = repo.head().unwrap().peel_to_commit().unwrap();
        let feature_oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Feature commit", &tree, &[&feature_head])
            .unwrap();

        // Go back to main and merge
        repo.checkout_tree(
            repo.find_commit(main_oid).unwrap().tree().unwrap().as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        )
        .unwrap();
        repo.set_head("refs/heads/master").unwrap();

        // Create merge commit
        let main_commit = repo.find_commit(main_oid).unwrap();
        let feature_commit = repo.find_commit(feature_oid).unwrap();
        let mut merge_index = repo
            .merge_commits(&main_commit, &feature_commit, None)
            .unwrap();
        let merge_tree_id = merge_index.write_tree_to(&repo).unwrap();
        let merge_tree = repo.find_tree(merge_tree_id).unwrap();

        let merge_oid = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Merge feature",
                &merge_tree,
                &[&main_commit, &feature_commit],
            )
            .unwrap();

        // Verify merge commit has two parents in log
        let result = get_commit_log(dir.path().to_string_lossy().to_string(), Some(1), None);
        assert!(result.is_ok());

        let commits = result.unwrap();
        assert_eq!(commits[0].message, "Merge feature");
        assert_eq!(commits[0].parent_ids.len(), 2);

        // Also verify diff for merge commit
        let diff_result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            merge_oid.to_string(),
        );
        assert!(diff_result.is_ok());
    }

    #[test]
    fn test_get_commit_log_branch_divergence() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let sig = test_signature();

        // Add commits on master (default branch)
        add_commit(&repo, dir.path(), "file1.txt", "content1", "Master commit 1");
        let base_oid = add_commit(&repo, dir.path(), "file2.txt", "content2", "Master commit 2");

        // Create feature branch from current HEAD (Master commit 2)
        let base_commit = repo.find_commit(base_oid).unwrap();
        repo.branch("feature", &base_commit, false).unwrap();

        // Add another commit on master
        add_commit(&repo, dir.path(), "file3.txt", "content3", "Master commit 3");

        // Switch to feature branch
        let feature_branch = repo.find_branch("feature", git2::BranchType::Local).unwrap();
        let feature_commit = feature_branch.get().peel_to_commit().unwrap();
        repo.checkout_tree(
            feature_commit.tree().unwrap().as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        ).unwrap();
        repo.set_head("refs/heads/feature").unwrap();

        // Add commits on feature branch
        fs::write(dir.path().join("feature1.txt"), "feature content 1\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("feature1.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Feature commit 1", &tree, &[&head]).unwrap();

        fs::write(dir.path().join("feature2.txt"), "feature content 2\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("feature2.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Feature commit 2", &tree, &[&head]).unwrap();

        // Get commit log from feature branch perspective
        let result = get_commit_log(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());

        let commits = result.unwrap();

        // Feature commits should be column 1, "current" (branched off to the right)
        let fc2 = commits.iter().find(|c| c.message == "Feature commit 2").unwrap();
        assert_eq!(fc2.graph_column, 1);
        assert_eq!(fc2.branch_type, "current");

        let fc1 = commits.iter().find(|c| c.message == "Feature commit 1").unwrap();
        assert_eq!(fc1.graph_column, 1);
        assert_eq!(fc1.branch_type, "current");

        // Master-only commit should be column 1, "base"
        let mc3 = commits.iter().find(|c| c.message == "Master commit 3").unwrap();
        assert_eq!(mc3.graph_column, 1);
        assert_eq!(mc3.branch_type, "base");

        // Merge-base commit should be column 0, "both"
        let mc2 = commits.iter().find(|c| c.message == "Master commit 2").unwrap();
        assert_eq!(mc2.graph_column, 0);
        assert_eq!(mc2.branch_type, "both");

        // Shared history should be column 0, "shared"
        let mc1 = commits.iter().find(|c| c.message == "Master commit 1").unwrap();
        assert_eq!(mc1.graph_column, 0);
        assert_eq!(mc1.branch_type, "shared");

        let ic = commits.iter().find(|c| c.message == "Initial commit").unwrap();
        assert_eq!(ic.graph_column, 0);
        assert_eq!(ic.branch_type, "shared");
    }

    #[test]
    fn test_get_commit_log_branch_no_divergence_on_main() {
        // When feature branch is created from main and main hasn't advanced,
        // the merge-base (= main HEAD) should be "shared", not "both"
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let sig = test_signature();

        // Add commits on master (default branch)
        add_commit(&repo, dir.path(), "file1.txt", "content1", "Master commit 1");
        let base_oid = add_commit(&repo, dir.path(), "file2.txt", "content2", "Master commit 2");

        // Create feature branch from current HEAD (Master commit 2)
        let base_commit = repo.find_commit(base_oid).unwrap();
        repo.branch("feature", &base_commit, false).unwrap();

        // Do NOT add more commits on master (main hasn't advanced)

        // Switch to feature branch
        let feature_branch = repo.find_branch("feature", git2::BranchType::Local).unwrap();
        let feature_commit = feature_branch.get().peel_to_commit().unwrap();
        repo.checkout_tree(
            feature_commit.tree().unwrap().as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        ).unwrap();
        repo.set_head("refs/heads/feature").unwrap();

        // Add commits on feature branch
        fs::write(dir.path().join("feature1.txt"), "feature content 1\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("feature1.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Feature commit 1", &tree, &[&head]).unwrap();

        let result = get_commit_log(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());

        let commits = result.unwrap();

        // Feature commit should be "current"
        let fc1 = commits.iter().find(|c| c.message == "Feature commit 1").unwrap();
        assert_eq!(fc1.branch_type, "current");

        // Merge-base (= main HEAD) should be "shared" since main hasn't advanced
        let mc2 = commits.iter().find(|c| c.message == "Master commit 2").unwrap();
        assert_eq!(mc2.branch_type, "shared");

        // Earlier commits should also be "shared"
        let mc1 = commits.iter().find(|c| c.message == "Master commit 1").unwrap();
        assert_eq!(mc1.branch_type, "shared");
    }

    #[test]
    fn test_get_branch_ahead_count_on_feature() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let sig = test_signature();

        add_commit(&repo, dir.path(), "file1.txt", "c1", "Master commit 1");
        let base_oid = add_commit(&repo, dir.path(), "file2.txt", "c2", "Master commit 2");

        // Create and switch to feature branch
        let base_commit = repo.find_commit(base_oid).unwrap();
        repo.branch("feature", &base_commit, false).unwrap();
        let fb = repo.find_branch("feature", git2::BranchType::Local).unwrap();
        let fc = fb.get().peel_to_commit().unwrap();
        repo.checkout_tree(fc.tree().unwrap().as_object(), Some(git2::build::CheckoutBuilder::new().force())).unwrap();
        repo.set_head("refs/heads/feature").unwrap();

        // Add 3 commits on feature
        for i in 1..=3 {
            fs::write(dir.path().join(format!("f{}.txt", i)), format!("fc{}", i)).unwrap();
            let mut idx = repo.index().unwrap();
            idx.add_path(Path::new(&format!("f{}.txt", i))).unwrap();
            idx.write().unwrap();
            let tid = idx.write_tree().unwrap();
            let tree = repo.find_tree(tid).unwrap();
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, &format!("Feature {}", i), &tree, &[&head]).unwrap();
        }

        let count = get_branch_ahead_count(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_get_branch_ahead_count_on_main() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        // On master (default branch), ahead count should be 0
        let count = get_branch_ahead_count(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_get_commit_log_not_a_repo() {
        let dir = tempdir().unwrap();
        let result = get_commit_log(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_commit_diff_invalid_hash() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            "invalid_hash".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_info_short_hash() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        let result = get_commit_log(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());

        let commits = result.unwrap();
        assert_eq!(commits[0].id.len(), 7);
        assert_eq!(commits[0].full_hash.len(), 40);
    }

    #[test]
    fn test_commit_info_fields() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        add_commit(&repo, dir.path(), "test.txt", "hello\n", "Test message");

        let result = get_commit_log(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());

        let commits = result.unwrap();
        let latest = &commits[0];
        assert_eq!(latest.message, "Test message");
        assert_eq!(latest.author, "test");
        assert_eq!(latest.author_email, "test@example.com");
        assert!(latest.date > 0);
        assert_eq!(latest.parent_ids.len(), 1);
    }

    #[test]
    fn test_get_commit_diff_total_counts() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let oid = add_commit(
            &repo,
            dir.path(),
            "multi.txt",
            "line1\nline2\nline3\n",
            "Add multi-line file",
        );

        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            oid.to_string(),
        );
        assert!(result.is_ok());

        let diff_result = result.unwrap();
        assert_eq!(diff_result.total_additions, 3);
        assert_eq!(diff_result.total_deletions, 0);
    }

    #[test]
    fn test_get_behind_ahead_count_no_remote() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        let result = get_behind_ahead_count(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let counts = result.unwrap();
        // No remote configured, should return 0/0
        assert_eq!(counts.behind, 0);
        assert_eq!(counts.ahead, 0);
    }

    #[test]
    fn test_get_behind_ahead_count_empty_repo() {
        let dir = tempdir().unwrap();
        Repository::init(dir.path()).unwrap();

        let result = get_behind_ahead_count(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let counts = result.unwrap();
        assert_eq!(counts.behind, 0);
        assert_eq!(counts.ahead, 0);
    }

    #[test]
    fn test_get_behind_ahead_count_not_a_repo() {
        let dir = tempdir().unwrap();
        let result = get_behind_ahead_count(dir.path().to_string_lossy().to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_fetch_result_serialization() {
        let result = FetchResult {
            success: true,
            message: "".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn test_behind_ahead_count_serialization() {
        let counts = BehindAheadCount {
            behind: 3,
            ahead: 2,
        };
        let json = serde_json::to_string(&counts).unwrap();
        assert!(json.contains("\"behind\":3"));
        assert!(json.contains("\"ahead\":2"));
    }

    #[test]
    fn test_pull_result_serialization() {
        let result = PullResult {
            success: true,
            message: "Already up to date.".to_string(),
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("Already up to date."));
    }

    #[test]
    fn test_fetch_remote_not_a_repo() {
        let dir = tempdir().unwrap();
        // fetch_remote uses git CLI, so it will fail on a non-repo directory
        let result = fetch_remote(dir.path().to_string_lossy().to_string(), None);
        assert!(result.is_ok());
        // git fetch will fail but the function returns FetchResult with success: false
        assert!(!result.unwrap().success);
    }

    #[test]
    fn test_fetch_remote_no_remote() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        let result = fetch_remote(dir.path().to_string_lossy().to_string(), None);
        assert!(result.is_ok());
        // No remote configured, git fetch will fail
        assert!(!result.unwrap().success);
    }

    #[test]
    fn test_get_behind_ahead_count_with_remote() {
        // Create a "remote" repo with an initial commit so it has a branch
        let remote_dir = tempdir().unwrap();
        let remote_repo = Repository::init(remote_dir.path()).unwrap();
        let sig = test_signature();

        // Create initial commit in remote
        fs::write(remote_dir.path().join("README.md"), "# Remote").unwrap();
        let mut index = remote_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = remote_repo.find_tree(tree_id).unwrap();
        remote_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Clone it
        let local_dir = tempdir().unwrap();
        let local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // Add a commit locally
        add_commit(
            &local_repo,
            local_dir.path(),
            "file.txt",
            "content",
            "Local commit",
        );

        let result = get_behind_ahead_count(local_dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let counts = result.unwrap();
        assert_eq!(counts.ahead, 1);
        assert_eq!(counts.behind, 0);
    }

    #[test]
    fn test_detect_default_branch_with_main() {
        // git init creates "master" by default, so we rename it to "main"
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = test_signature();

        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Rename the current branch to "main"
        let mut branch = repo
            .find_branch("master", git2::BranchType::Local)
            .or_else(|_| repo.find_branch("main", git2::BranchType::Local))
            .unwrap();
        // If already "main", just verify detect works
        let current_name = branch.name().unwrap().unwrap().to_string();
        if current_name != "main" {
            branch.rename("main", false).unwrap();
            repo.set_head("refs/heads/main").unwrap();
        }

        // detect_default_branch is private, test indirectly via get_branch_ahead_count
        // On the default branch itself, it returns 0
        let count = get_branch_ahead_count(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(count, 0);

        // Also verify via get_commit_log that it finds commits (uses detect_default_branch internally)
        let commits = get_commit_log(dir.path().to_string_lossy().to_string(), None, None).unwrap();
        assert_eq!(commits.len(), 1);
        // On main branch with no divergence, branch_type should be "current"
        assert_eq!(commits[0].branch_type, "current");
    }

    #[test]
    fn test_detect_default_branch_with_master() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // create_repo_with_commit uses git init which creates "master" by default
        // Verify master branch exists
        let branch = repo.find_branch("master", git2::BranchType::Local);
        if branch.is_err() {
            // On systems where default is "main", skip this test
            return;
        }

        let count = get_branch_ahead_count(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(count, 0);

        let commits = get_commit_log(dir.path().to_string_lossy().to_string(), None, None).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].branch_type, "current");
    }

    #[test]
    fn test_detect_default_branch_no_default() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = test_signature();

        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Rename the default branch to something that is neither "main" nor "master"
        let current_branch_name = repo
            .head()
            .unwrap()
            .shorthand()
            .unwrap()
            .to_string();
        let mut branch = repo
            .find_branch(&current_branch_name, git2::BranchType::Local)
            .unwrap();
        branch.rename("develop", false).unwrap();
        repo.set_head("refs/heads/develop").unwrap();

        // detect_default_branch should return None, so get_branch_ahead_count returns 0
        let count = get_branch_ahead_count(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_detect_default_branch_prefers_origin_head() {
        // Create a "remote" repo
        let remote_dir = tempdir().unwrap();
        let remote_repo = Repository::init(remote_dir.path()).unwrap();
        let sig = test_signature();

        fs::write(remote_dir.path().join("README.md"), "# Remote").unwrap();
        let mut index = remote_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = remote_repo.find_tree(tree_id).unwrap();
        remote_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Clone it so we get origin/HEAD set up
        let local_dir = tempdir().unwrap();
        let local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // Verify origin/HEAD exists by checking commit log works
        let commits = get_commit_log(local_dir.path().to_string_lossy().to_string(), None, None).unwrap();
        assert!(!commits.is_empty());

        // On the default branch, ahead count is 0
        let count = get_branch_ahead_count(local_dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(count, 0);

        // Add a commit on a feature branch and verify ahead count uses origin/HEAD detection
        let head_commit = local_repo.head().unwrap().peel_to_commit().unwrap();
        local_repo.branch("feature", &head_commit, false).unwrap();
        let fb = local_repo.find_branch("feature", git2::BranchType::Local).unwrap();
        let fc = fb.get().peel_to_commit().unwrap();
        local_repo
            .checkout_tree(
                fc.tree().unwrap().as_object(),
                Some(git2::build::CheckoutBuilder::new().force()),
            )
            .unwrap();
        local_repo.set_head("refs/heads/feature").unwrap();

        add_commit(&local_repo, local_dir.path(), "feature.txt", "feature\n", "Feature commit");

        let count = get_branch_ahead_count(local_dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_push_commits_not_a_repo() {
        let dir = tempdir().unwrap();
        // push_commits tries to open the repo first, so it should return an error
        let result = push_commits(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_push_commits_no_remote() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        // push_commits uses git CLI; no remote configured means push will fail
        let result = push_commits(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());
        let push_result = result.unwrap();
        assert!(!push_result.success);
    }

    #[test]
    fn test_pull_commits_not_a_repo() {
        let dir = tempdir().unwrap();
        // pull_commits tries to open the repo to detect the branch, so it should error
        let result = pull_commits(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_pull_commits_no_remote() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        // pull_commits uses git CLI; no remote configured means pull will fail
        let result = pull_commits(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());
        let pull_result = result.unwrap();
        assert!(!pull_result.success);
    }

    #[test]
    fn test_fetch_remote_custom_remote() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        // Use a custom remote name that doesn't exist
        let result = fetch_remote(
            dir.path().to_string_lossy().to_string(),
            Some("upstream".to_string()),
        );
        assert!(result.is_ok());
        // Should fail because "upstream" remote does not exist
        assert!(!result.unwrap().success);
    }

    #[test]
    fn test_get_branch_ahead_count_not_a_repo() {
        let dir = tempdir().unwrap();
        let result = get_branch_ahead_count(dir.path().to_string_lossy().to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_behind_ahead_count_detached_head() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let oid = add_commit(&repo, dir.path(), "file.txt", "content\n", "Second commit");

        // Detach HEAD by pointing it directly to the commit OID
        repo.set_head_detached(oid).unwrap();

        let result = get_behind_ahead_count(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let counts = result.unwrap();
        // Detached HEAD means shorthand() returns "HEAD", so upstream lookup is skipped
        assert_eq!(counts.behind, 0);
        assert_eq!(counts.ahead, 0);
    }

    #[test]
    fn test_pull_commits_with_local_clone() {
        // Create a remote repo
        let remote_dir = tempdir().unwrap();
        let remote_repo = Repository::init(remote_dir.path()).unwrap();
        let sig = test_signature();

        fs::write(remote_dir.path().join("README.md"), "# Remote").unwrap();
        let mut index = remote_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = remote_repo.find_tree(tree_id).unwrap();
        remote_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Clone it
        let local_dir = tempdir().unwrap();
        let _local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // Add a commit in the remote
        add_commit(&remote_repo, remote_dir.path(), "new_file.txt", "new content\n", "Remote commit");

        // Pull from local clone
        let result = pull_commits(local_dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());
        let pull_result = result.unwrap();
        assert!(pull_result.success, "Pull failed: {}", pull_result.message);
    }

    #[test]
    fn test_push_commits_with_local_clone() {
        // Create a bare remote repo (bare repos accept pushes)
        let remote_dir = tempdir().unwrap();
        let _bare_repo = Repository::init_bare(remote_dir.path()).unwrap();

        // We need to set up the bare repo with an initial commit so the clone has a branch
        // First, create a temporary non-bare repo, commit, then push to bare
        let setup_dir = tempdir().unwrap();
        let setup_repo = Repository::init(setup_dir.path()).unwrap();
        let sig = test_signature();

        fs::write(setup_dir.path().join("README.md"), "# Setup").unwrap();
        let mut index = setup_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = setup_repo.find_tree(tree_id).unwrap();
        setup_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Add the bare repo as remote and push
        setup_repo
            .remote("origin", remote_dir.path().to_str().unwrap())
            .unwrap();

        let branch_name = setup_repo
            .head()
            .unwrap()
            .shorthand()
            .unwrap()
            .to_string();

        // Use git CLI to push setup to bare
        let push_setup = std::process::Command::new("git")
            .args(["push", "origin", &branch_name])
            .current_dir(setup_dir.path())
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .output()
            .unwrap();
        assert!(push_setup.status.success(), "Setup push failed: {}", String::from_utf8_lossy(&push_setup.stderr));

        // Clone the bare repo
        let local_dir = tempdir().unwrap();
        let local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // Configure git user for the local clone
        let mut config = local_repo.config().unwrap();
        config.set_str("user.name", "test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Add a commit locally
        add_commit(&local_repo, local_dir.path(), "new_file.txt", "content\n", "Local commit");

        // Push
        let result = push_commits(local_dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());
        let push_result = result.unwrap();
        assert!(push_result.success, "Push failed: {}", push_result.message);
    }

    #[test]
    fn test_get_commit_log_with_message_body() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = test_signature();

        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        // Create a commit with a multi-line message
        let multi_line_msg = "Add initial README\n\nThis is a detailed description\nwith multiple lines.";
        repo.commit(Some("HEAD"), &sig, &sig, multi_line_msg, &tree, &[])
            .unwrap();

        let commits = get_commit_log(dir.path().to_string_lossy().to_string(), None, None).unwrap();
        assert_eq!(commits.len(), 1);

        // message should be just the summary (first line)
        assert_eq!(commits[0].message, "Add initial README");

        // message_body should contain the full message including body
        assert!(commits[0].message_body.contains("Add initial README"));
        assert!(commits[0].message_body.contains("This is a detailed description"));
        assert!(commits[0].message_body.contains("with multiple lines."));
    }

    #[test]
    fn test_build_commit_info_fields() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = test_signature();

        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let commit_msg = "Test build_commit_info\n\nDetailed body text here.";
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, commit_msg, &tree, &[])
            .unwrap();

        // Add a second commit so we can verify parent_ids
        add_commit(&repo, dir.path(), "file.txt", "content\n", "Second commit");

        let commits = get_commit_log(dir.path().to_string_lossy().to_string(), None, None).unwrap();
        assert_eq!(commits.len(), 2);

        // Verify second commit (latest)
        let latest = &commits[0];
        assert_eq!(latest.message, "Second commit");
        assert_eq!(latest.author, "test");
        assert_eq!(latest.author_email, "test@example.com");
        assert!(latest.date > 0);
        assert_eq!(latest.id.len(), 7);
        assert_eq!(latest.full_hash.len(), 40);
        assert_eq!(latest.parent_ids.len(), 1);
        // parent should be the first commit
        assert_eq!(latest.parent_ids[0], oid.to_string());

        // Verify first commit (root commit)
        let root = &commits[1];
        assert_eq!(root.message, "Test build_commit_info");
        assert!(root.message_body.contains("Detailed body text here."));
        assert!(root.parent_ids.is_empty());
        assert_eq!(root.branch_type, "current");
        assert_eq!(root.graph_column, 0);
    }

    #[test]
    fn test_build_commit_info_directly() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = test_signature();

        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Summary line\n\nBody paragraph.", &tree, &[])
            .unwrap();

        let commit = repo.find_commit(oid).unwrap();

        // Test with is_pushed = true
        let info = build_commit_info(&commit, true, "base", 2);
        assert_eq!(info.id.len(), 7);
        assert_eq!(info.full_hash, oid.to_string());
        assert_eq!(info.message, "Summary line");
        assert!(info.message_body.contains("Body paragraph."));
        assert_eq!(info.author, "test");
        assert_eq!(info.author_email, "test@example.com");
        assert!(info.date > 0);
        assert!(info.parent_ids.is_empty());
        assert!(info.is_pushed);
        assert_eq!(info.branch_type, "base");
        assert_eq!(info.graph_column, 2);

        // Test with is_pushed = false and different branch_type
        let info2 = build_commit_info(&commit, false, "shared", 0);
        assert!(!info2.is_pushed);
        assert_eq!(info2.branch_type, "shared");
        assert_eq!(info2.graph_column, 0);
    }

    #[test]
    fn test_build_commit_info_with_multiple_parents() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let sig = test_signature();

        // Create a branch
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature", &head, false).unwrap();

        // Add a commit on master
        let main_oid = add_commit(&repo, dir.path(), "main_file.txt", "main\n", "Main commit");

        // Checkout feature and add commit
        let feature_branch = repo.find_branch("feature", git2::BranchType::Local).unwrap();
        let feature_commit = feature_branch.get().peel_to_commit().unwrap();
        repo.checkout_tree(
            feature_commit.tree().unwrap().as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        )
        .unwrap();
        repo.set_head("refs/heads/feature").unwrap();

        fs::write(dir.path().join("feature_file.txt"), "feature\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("feature_file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let feature_head = repo.head().unwrap().peel_to_commit().unwrap();
        let feature_oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Feature commit", &tree, &[&feature_head])
            .unwrap();

        // Go back to main and create merge commit
        repo.checkout_tree(
            repo.find_commit(main_oid).unwrap().tree().unwrap().as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        )
        .unwrap();
        repo.set_head("refs/heads/master").unwrap();

        let main_commit = repo.find_commit(main_oid).unwrap();
        let feature_commit = repo.find_commit(feature_oid).unwrap();
        let mut merge_index = repo
            .merge_commits(&main_commit, &feature_commit, None)
            .unwrap();
        let merge_tree_id = merge_index.write_tree_to(&repo).unwrap();
        let merge_tree = repo.find_tree(merge_tree_id).unwrap();

        let merge_oid = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Merge feature",
                &merge_tree,
                &[&main_commit, &feature_commit],
            )
            .unwrap();

        // Test build_commit_info with a merge commit that has 2 parents
        let merge_commit = repo.find_commit(merge_oid).unwrap();
        let info = build_commit_info(&merge_commit, false, "current", 0);
        assert_eq!(info.parent_ids.len(), 2);
        assert_eq!(info.parent_ids[0], main_oid.to_string());
        assert_eq!(info.parent_ids[1], feature_oid.to_string());
    }

    #[test]
    fn test_detect_default_branch_directly_main() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = test_signature();

        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Rename to "main" if not already
        let current = repo.head().unwrap().shorthand().unwrap().to_string();
        if current != "main" {
            let mut branch = repo.find_branch(&current, git2::BranchType::Local).unwrap();
            branch.rename("main", false).unwrap();
            repo.set_head("refs/heads/main").unwrap();
        }

        let result = detect_default_branch(&repo);
        assert_eq!(result, Some("main".to_string()));
    }

    #[test]
    fn test_detect_default_branch_directly_master() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = test_signature();

        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Rename to "master" if not already
        let current = repo.head().unwrap().shorthand().unwrap().to_string();
        if current != "master" {
            let mut branch = repo.find_branch(&current, git2::BranchType::Local).unwrap();
            branch.rename("master", false).unwrap();
            repo.set_head("refs/heads/master").unwrap();
        }

        let result = detect_default_branch(&repo);
        assert_eq!(result, Some("master".to_string()));
    }

    #[test]
    fn test_detect_default_branch_directly_none() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = test_signature();

        fs::write(dir.path().join("README.md"), "# Test").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Rename to something neither main nor master
        let current = repo.head().unwrap().shorthand().unwrap().to_string();
        let mut branch = repo.find_branch(&current, git2::BranchType::Local).unwrap();
        branch.rename("develop", false).unwrap();
        repo.set_head("refs/heads/develop").unwrap();

        let result = detect_default_branch(&repo);
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_default_branch_with_origin_head() {
        // Create a "remote" repo
        let remote_dir = tempdir().unwrap();
        let remote_repo = Repository::init(remote_dir.path()).unwrap();
        let sig = test_signature();

        fs::write(remote_dir.path().join("README.md"), "# Remote").unwrap();
        let mut index = remote_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = remote_repo.find_tree(tree_id).unwrap();
        remote_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Clone it (sets up origin/HEAD)
        let local_dir = tempdir().unwrap();
        let local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // detect_default_branch should resolve via origin/HEAD
        let result = detect_default_branch(&local_repo);
        assert!(result.is_some());
        // The default branch from git init is usually "master" or "main"
        let name = result.unwrap();
        assert!(name == "main" || name == "master");
    }

    #[test]
    fn test_get_commit_log_detached_head() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let oid = add_commit(&repo, dir.path(), "file.txt", "content\n", "Second commit");

        // Detach HEAD
        repo.set_head_detached(oid).unwrap();

        let result = get_commit_log(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());

        let commits = result.unwrap();
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].message, "Second commit");
        // In detached HEAD, shorthand returns "HEAD", so upstream lookup is skipped
        // All commits should be not pushed
        assert!(!commits[0].is_pushed);
    }

    #[test]
    fn test_get_commit_diff_renamed_file() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let sig = test_signature();

        // Add a file with enough content for rename detection
        let content = "This is a file with enough content for git to detect it as a rename.\n\
                       Line 2: some more content to help with rename detection.\n\
                       Line 3: even more content here.\n\
                       Line 4: and yet another line of content.\n\
                       Line 5: final line of our test content.\n";
        add_commit(&repo, dir.path(), "original.txt", content, "Add original file");

        // Rename the file using git mv style: remove old, add new with same content
        fs::remove_file(dir.path().join("original.txt")).unwrap();
        fs::write(dir.path().join("renamed.txt"), content).unwrap();

        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("original.txt")).unwrap();
        index.add_path(Path::new("renamed.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();

        // Use diff options with rename detection for the commit
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Rename file", &tree, &[&head])
            .unwrap();

        // get_commit_diff uses default diff which may or may not detect renames
        // depending on git2 settings, but at least exercise the code path
        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            oid.to_string(),
        );
        assert!(result.is_ok());

        let diff_result = result.unwrap();
        assert!(!diff_result.files.is_empty());
    }

    #[test]
    fn test_get_branch_ahead_count_same_oid_as_default() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Create a feature branch at the same commit as master
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature", &head, false).unwrap();

        // Checkout feature branch (HEAD same OID as master)
        let feature_branch = repo.find_branch("feature", git2::BranchType::Local).unwrap();
        let feature_commit = feature_branch.get().peel_to_commit().unwrap();
        repo.checkout_tree(
            feature_commit.tree().unwrap().as_object(),
            Some(git2::build::CheckoutBuilder::new().force()),
        )
        .unwrap();
        repo.set_head("refs/heads/feature").unwrap();

        // head_oid == default_oid should return 0
        let count = get_branch_ahead_count(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_pull_commits_with_explicit_branch() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        // Pull with explicit branch name (exercises the Some(b) path)
        let result = pull_commits(
            dir.path().to_string_lossy().to_string(),
            Some("origin".to_string()),
            Some("master".to_string()),
        );
        assert!(result.is_ok());
        // No remote configured, so it should fail
        let pull_result = result.unwrap();
        assert!(!pull_result.success);
    }

    #[test]
    fn test_push_commits_with_explicit_branch() {
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        // Push with explicit branch name (exercises the Some(b) path)
        let result = push_commits(
            dir.path().to_string_lossy().to_string(),
            Some("origin".to_string()),
            Some("master".to_string()),
        );
        assert!(result.is_ok());
        // No remote configured, so it should fail
        let push_result = result.unwrap();
        assert!(!push_result.success);
    }

    #[test]
    fn test_get_commit_log_with_remote_tracking() {
        // Create a remote repo with a commit
        let remote_dir = tempdir().unwrap();
        let remote_repo = Repository::init(remote_dir.path()).unwrap();
        let sig = test_signature();

        fs::write(remote_dir.path().join("README.md"), "# Remote").unwrap();
        let mut index = remote_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = remote_repo.find_tree(tree_id).unwrap();
        remote_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Clone it
        let local_dir = tempdir().unwrap();
        let local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // Add local commits
        add_commit(&local_repo, local_dir.path(), "local.txt", "local\n", "Local commit");

        // Commits on the tracked branch should have is_pushed set correctly
        let result = get_commit_log(local_dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());

        let commits = result.unwrap();
        assert_eq!(commits.len(), 2);

        // The local commit should NOT be pushed
        let local = commits.iter().find(|c| c.message == "Local commit").unwrap();
        assert!(!local.is_pushed);

        // The initial commit should be pushed (it exists on origin)
        let initial = commits.iter().find(|c| c.message == "Initial commit").unwrap();
        assert!(initial.is_pushed);
    }

    #[test]
    fn test_get_commit_log_fallback_upstream_detection() {
        // Test the fallback path where current branch has no upstream
        // but detect_default_branch finds origin's default branch
        let remote_dir = tempdir().unwrap();
        let remote_repo = Repository::init(remote_dir.path()).unwrap();
        let sig = test_signature();

        fs::write(remote_dir.path().join("README.md"), "# Remote").unwrap();
        let mut index = remote_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = remote_repo.find_tree(tree_id).unwrap();
        remote_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Clone it
        let local_dir = tempdir().unwrap();
        let local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // Create a feature branch that does NOT track a remote
        let head_commit = local_repo.head().unwrap().peel_to_commit().unwrap();
        local_repo.branch("feature-no-remote", &head_commit, false).unwrap();

        let fb = local_repo.find_branch("feature-no-remote", git2::BranchType::Local).unwrap();
        let fc = fb.get().peel_to_commit().unwrap();
        local_repo
            .checkout_tree(
                fc.tree().unwrap().as_object(),
                Some(git2::build::CheckoutBuilder::new().force()),
            )
            .unwrap();
        local_repo.set_head("refs/heads/feature-no-remote").unwrap();

        // Add a commit on the feature branch
        add_commit(&local_repo, local_dir.path(), "feat.txt", "feat\n", "Feature commit");

        // get_commit_log should still work - fallback to origin/main or origin/master
        let result = get_commit_log(local_dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());

        let commits = result.unwrap();
        assert!(!commits.is_empty());

        // The initial commit should be marked as pushed via the fallback upstream
        let initial = commits.iter().find(|c| c.message == "Initial commit").unwrap();
        assert!(initial.is_pushed);
    }

    #[test]
    fn test_get_commit_diff_multiple_files() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let sig = test_signature();

        // Add multiple files in a single commit
        fs::write(dir.path().join("file_a.txt"), "content A\n").unwrap();
        fs::write(dir.path().join("file_b.txt"), "content B\nline 2\n").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file_a.txt")).unwrap();
        index.add_path(Path::new("file_b.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Add multiple files", &tree, &[&head])
            .unwrap();

        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            oid.to_string(),
        );
        assert!(result.is_ok());

        let diff_result = result.unwrap();
        assert_eq!(diff_result.files.len(), 2);
        assert_eq!(diff_result.total_additions, 3); // 1 + 2

        let file_a = diff_result.files.iter().find(|f| f.path == "file_a.txt").unwrap();
        assert_eq!(file_a.additions, 1);
        assert_eq!(file_a.deletions, 0);

        let file_b = diff_result.files.iter().find(|f| f.path == "file_b.txt").unwrap();
        assert_eq!(file_b.additions, 2);
        assert_eq!(file_b.deletions, 0);
    }

    #[test]
    fn test_get_commit_diff_mixed_changes() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let sig = test_signature();

        // Add a file
        add_commit(&repo, dir.path(), "modify_me.txt", "line1\nline2\nline3\n", "Add file");

        // Modify the file (change line2, keep others)
        fs::write(dir.path().join("modify_me.txt"), "line1\nmodified_line2\nline3\n").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("modify_me.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, "Modify file", &tree, &[&head])
            .unwrap();

        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            oid.to_string(),
        );
        assert!(result.is_ok());

        let diff_result = result.unwrap();
        assert_eq!(diff_result.files.len(), 1);
        let f = &diff_result.files[0];
        assert_eq!(f.status, "Modified");
        assert!(f.additions > 0);
        assert!(f.deletions > 0);
        assert_eq!(diff_result.total_additions, f.additions);
        assert_eq!(diff_result.total_deletions, f.deletions);
        // Diff text should contain the changes
        assert!(f.diff.contains("modified_line2"));
    }

    #[test]
    fn test_get_commit_diff_not_a_repo() {
        let dir = tempdir().unwrap();
        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            "0000000000000000000000000000000000000000".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_get_branch_ahead_count_empty_repo() {
        let dir = tempdir().unwrap();
        Repository::init(dir.path()).unwrap();

        // Empty repo has no HEAD, should error
        let result = get_branch_ahead_count(dir.path().to_string_lossy().to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_diff_result_serialization() {
        let result = CommitDiffResult {
            commit: CommitInfo {
                id: "abc1234".to_string(),
                full_hash: "abc1234567890abcdef1234567890abcdef123456".to_string(),
                message: "Test".to_string(),
                message_body: "Test\n\nBody".to_string(),
                author: "Test".to_string(),
                author_email: "test@example.com".to_string(),
                date: 1704067200,
                parent_ids: vec![],
                is_pushed: false,
                branch_type: "current".to_string(),
                graph_column: 0,
            },
            files: vec![CommitFileDiff {
                path: "test.rs".to_string(),
                status: "Added".to_string(),
                diff: "+ line\n".to_string(),
                additions: 1,
                deletions: 0,
            }],
            total_additions: 1,
            total_deletions: 0,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"total_additions\":1"));
        assert!(json.contains("\"total_deletions\":0"));
        assert!(json.contains("\"test.rs\""));
    }

    #[test]
    fn test_get_behind_ahead_count_with_ahead_and_behind() {
        // Create a "remote" repo
        let remote_dir = tempdir().unwrap();
        let remote_repo = Repository::init(remote_dir.path()).unwrap();
        let sig = test_signature();

        fs::write(remote_dir.path().join("README.md"), "# Remote").unwrap();
        let mut index = remote_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = remote_repo.find_tree(tree_id).unwrap();
        remote_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Clone it
        let local_dir = tempdir().unwrap();
        let local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // Add a commit locally (ahead by 1)
        add_commit(&local_repo, local_dir.path(), "local.txt", "local\n", "Local commit");

        // Add a commit on the remote (local will be behind by 1 after fetch)
        add_commit(&remote_repo, remote_dir.path(), "remote.txt", "remote\n", "Remote commit");

        // Fetch to update remote tracking branches
        let fetch_result = fetch_remote(local_dir.path().to_string_lossy().to_string(), None);
        assert!(fetch_result.is_ok());
        assert!(fetch_result.unwrap().success);

        let result = get_behind_ahead_count(local_dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let counts = result.unwrap();
        assert_eq!(counts.ahead, 1);
        assert_eq!(counts.behind, 1);
    }

    #[test]
    fn test_get_commit_log_on_main_no_divergence() {
        // When on main branch with no other branches, all commits are "current"
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        add_commit(&repo, dir.path(), "f1.txt", "c1", "Commit 1");
        add_commit(&repo, dir.path(), "f2.txt", "c2", "Commit 2");

        let result = get_commit_log(dir.path().to_string_lossy().to_string(), None, None);
        assert!(result.is_ok());

        let commits = result.unwrap();
        assert_eq!(commits.len(), 3);
        for c in &commits {
            assert_eq!(c.branch_type, "current");
            assert_eq!(c.graph_column, 0);
        }
    }

    #[test]
    fn test_fetch_remote_with_valid_remote() {
        // Create remote and clone
        let remote_dir = tempdir().unwrap();
        let remote_repo = Repository::init(remote_dir.path()).unwrap();
        let sig = test_signature();

        fs::write(remote_dir.path().join("README.md"), "# Remote").unwrap();
        let mut index = remote_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = remote_repo.find_tree(tree_id).unwrap();
        remote_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        let local_dir = tempdir().unwrap();
        let _local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // Fetch should succeed
        let result = fetch_remote(local_dir.path().to_string_lossy().to_string(), None);
        assert!(result.is_ok());
        assert!(result.unwrap().success);
    }

    #[test]
    fn test_get_behind_ahead_count_empty_repo_no_head() {
        // An empty repo (no commits) should return (0, 0) from the head() Err path
        let dir = tempdir().unwrap();
        Repository::init(dir.path()).unwrap();

        let result = get_behind_ahead_count(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());
        let counts = result.unwrap();
        assert_eq!(counts.behind, 0);
        assert_eq!(counts.ahead, 0);
    }

    #[test]
    fn test_get_behind_ahead_count_no_upstream() {
        // A repo with commits but no remote should return (0, 0) from the None upstream path
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        let result = get_behind_ahead_count(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());
        let counts = result.unwrap();
        assert_eq!(counts.behind, 0);
        assert_eq!(counts.ahead, 0);
    }

    #[test]
    fn test_pull_commits_success_with_up_to_date() {
        // Clone a repo and pull when already up to date - exercises the success path
        let remote_dir = tempdir().unwrap();
        let _remote_repo = Repository::init_bare(remote_dir.path()).unwrap();
        let sig = test_signature();

        // Create a non-bare repo, add a commit, push to bare remote
        let source_dir = tempdir().unwrap();
        let source_repo = Repository::init(source_dir.path()).unwrap();
        fs::write(source_dir.path().join("README.md"), "# Test").unwrap();
        let mut index = source_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = source_repo.find_tree(tree_id).unwrap();
        source_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Clone from the bare remote
        // First push source to bare remote
        source_repo
            .remote("origin", remote_dir.path().to_str().unwrap())
            .unwrap();
        let current_branch = source_repo
            .head()
            .unwrap()
            .shorthand()
            .unwrap()
            .to_string();
        let mut remote = source_repo.find_remote("origin").unwrap();
        remote
            .push(
                &[&format!("refs/heads/{0}:refs/heads/{0}", current_branch)],
                None,
            )
            .unwrap();
        drop(remote);

        // Clone from bare remote
        let local_dir = tempdir().unwrap();
        let _local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // Pull when already up to date - should succeed
        let result = pull_commits(
            local_dir.path().to_string_lossy().to_string(),
            None,
            None,
        );
        assert!(result.is_ok());
        let pull_result = result.unwrap();
        assert!(pull_result.success);
        // Message should contain something (stdout or stderr)
        assert!(!pull_result.message.is_empty());
    }

    #[test]
    fn test_pull_commits_no_remote_fails() {
        // Pull from a repo with no remote configured - exercises failure path
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        let result = pull_commits(
            dir.path().to_string_lossy().to_string(),
            None,
            None,
        );
        assert!(result.is_ok());
        let pull_result = result.unwrap();
        // No remote, so git pull fails
        assert!(!pull_result.success);
        // stderr should be non-empty with the error message
        assert!(!pull_result.message.is_empty());
    }

    #[test]
    fn test_push_commits_no_remote_fails() {
        // Push from a repo with no remote configured - exercises failure path
        let dir = tempdir().unwrap();
        let _repo = create_repo_with_commit(dir.path());

        let result = push_commits(
            dir.path().to_string_lossy().to_string(),
            None,
            None,
        );
        assert!(result.is_ok());
        let push_result = result.unwrap();
        // No remote, so git push fails
        assert!(!push_result.success);
        // stderr should be non-empty with the error message
        assert!(!push_result.message.is_empty());
    }

    #[test]
    fn test_push_commits_success_to_bare_remote() {
        // Push to a bare remote - exercises the success path with stderr output
        let remote_dir = tempdir().unwrap();
        let _remote_repo = Repository::init_bare(remote_dir.path()).unwrap();
        let sig = test_signature();

        // Create local repo with a commit
        let local_dir = tempdir().unwrap();
        let local_repo = Repository::init(local_dir.path()).unwrap();
        fs::write(local_dir.path().join("README.md"), "# Test").unwrap();
        let mut index = local_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = local_repo.find_tree(tree_id).unwrap();
        local_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Add the bare repo as remote
        local_repo
            .remote("origin", remote_dir.path().to_str().unwrap())
            .unwrap();

        let branch_name = local_repo
            .head()
            .unwrap()
            .shorthand()
            .unwrap()
            .to_string();

        // Push should succeed
        let result = push_commits(
            local_dir.path().to_string_lossy().to_string(),
            Some("origin".to_string()),
            Some(branch_name),
        );
        assert!(result.is_ok());
        let push_result = result.unwrap();
        assert!(push_result.success);
    }

    #[test]
    fn test_get_commit_diff_with_rename_detection() {
        // Test that rename detection works by creating a commit with a renamed file
        // and verifying the diff output includes it (either as Renamed or Delete+Add)
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());
        let sig = test_signature();

        // Add a file with substantial content for rename detection
        let content = "Line 1: This is test content for rename detection.\n\
                       Line 2: Adding more lines helps git detect renames.\n\
                       Line 3: The similarity index needs enough content.\n\
                       Line 4: So we add several lines of unique text.\n\
                       Line 5: This should be enough for detection.\n\
                       Line 6: More content to ensure high similarity.\n\
                       Line 7: Even more content for good measure.\n\
                       Line 8: Almost done with the content.\n\
                       Line 9: Just one more line.\n\
                       Line 10: Final line of content.\n";
        add_commit(
            &repo,
            dir.path(),
            "before_rename.txt",
            content,
            "Add file before rename",
        );

        // Rename the file: remove old, add new with identical content
        fs::remove_file(dir.path().join("before_rename.txt")).unwrap();
        fs::write(dir.path().join("after_rename.txt"), content).unwrap();

        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("before_rename.txt")).unwrap();
        index.add_path(Path::new("after_rename.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();

        let oid = repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "Rename before_rename.txt to after_rename.txt",
                &tree,
                &[&head],
            )
            .unwrap();

        // get_commit_diff doesn't use find_similar(), so this will appear as
        // Delete + Add rather than Renamed. This test exercises the code path
        // and verifies the diff output handles file removal and addition correctly.
        let result = get_commit_diff(
            dir.path().to_string_lossy().to_string(),
            oid.to_string(),
        );
        assert!(result.is_ok());

        let diff_result = result.unwrap();
        assert!(!diff_result.files.is_empty());

        // Without find_similar(), the rename appears as Delete + Add
        let statuses: Vec<&str> = diff_result.files.iter().map(|f| f.status.as_str()).collect();
        // Should have either a Renamed status (if git2 detects it) or Delete+Add pair
        let has_rename = statuses.contains(&"Renamed");
        let has_delete_add = statuses.contains(&"Deleted") && statuses.contains(&"Added");
        assert!(
            has_rename || has_delete_add,
            "Expected Renamed or Deleted+Added, got: {:?}",
            statuses
        );
    }

    #[test]
    fn test_pull_commits_with_new_remote_commits() {
        // Create bare remote, clone, add commits to remote, pull
        // This exercises the success path where actual changes are pulled
        let remote_dir = tempdir().unwrap();
        let _bare_repo = Repository::init_bare(remote_dir.path()).unwrap();
        let sig = test_signature();

        // Create source repo and push initial commit to bare remote
        let source_dir = tempdir().unwrap();
        let source_repo = Repository::init(source_dir.path()).unwrap();
        fs::write(source_dir.path().join("README.md"), "# Test").unwrap();
        let mut index = source_repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = source_repo.find_tree(tree_id).unwrap();
        source_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        source_repo
            .remote("origin", remote_dir.path().to_str().unwrap())
            .unwrap();
        let branch_name = source_repo
            .head()
            .unwrap()
            .shorthand()
            .unwrap()
            .to_string();
        let mut remote = source_repo.find_remote("origin").unwrap();
        remote
            .push(
                &[&format!("refs/heads/{0}:refs/heads/{0}", branch_name)],
                None,
            )
            .unwrap();
        drop(remote);

        // Clone from bare remote
        let local_dir = tempdir().unwrap();
        let _local_repo = Repository::clone(
            remote_dir.path().to_str().unwrap(),
            local_dir.path(),
        )
        .unwrap();

        // Add a new commit to source and push to bare remote
        add_commit(
            &source_repo,
            source_dir.path(),
            "new_file.txt",
            "new content\n",
            "Add new file",
        );
        let mut remote = source_repo.find_remote("origin").unwrap();
        remote
            .push(
                &[&format!("refs/heads/{0}:refs/heads/{0}", branch_name)],
                None,
            )
            .unwrap();
        drop(remote);

        // Pull should succeed and fetch the new commit
        let result = pull_commits(
            local_dir.path().to_string_lossy().to_string(),
            Some("origin".to_string()),
            Some(branch_name),
        );
        assert!(result.is_ok());
        let pull_result = result.unwrap();
        assert!(pull_result.success);

        // Verify the new file was pulled
        assert!(local_dir.path().join("new_file.txt").exists());
    }

}
