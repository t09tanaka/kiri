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

    // Try to find upstream tracking ref for the current branch
    let upstream_oid = head
        .shorthand()
        .and_then(|branch_name| {
            // Skip if HEAD is detached
            if branch_name == "HEAD" {
                return None;
            }
            // Try the upstream tracking branch
            let remote_ref = format!("refs/remotes/origin/{}", branch_name);
            repo.find_reference(&remote_ref)
                .ok()
                .and_then(|r| r.target())
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

    // Detect default branch and compute merge-base
    let default_branch = detect_default_branch(&repo);
    let merge_base_oid: Option<Oid> = default_branch.and_then(|default_name| {
        let default_ref = repo
            .find_branch(&default_name, git2::BranchType::Local)
            .ok()?;
        let default_oid = default_ref.get().target()?;
        // If HEAD is on the default branch, no merge-base needed
        if default_oid == head_oid {
            return None;
        }
        repo.merge_base(head_oid, default_oid).ok()
    });

    // Revwalk from HEAD
    let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;
    revwalk.push(head_oid).map_err(|e| e.to_string())?;
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

        // Determine branch_type and graph_column
        let (branch_type, graph_column) = match merge_base_oid {
            Some(base_oid) if oid == base_oid => ("both".to_string(), 0),
            Some(base_oid) => {
                if repo
                    .graph_descendant_of(oid, base_oid)
                    .unwrap_or(false)
                {
                    ("current".to_string(), 0)
                } else {
                    ("base".to_string(), 1)
                }
            }
            None => ("current".to_string(), 0),
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
}
