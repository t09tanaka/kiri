use git2::Repository;
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct WorktreeInfo {
    pub name: String,
    pub path: String,
    pub branch: Option<String>,
    pub is_locked: bool,
    pub is_main: bool,
    pub is_valid: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorktreeContext {
    pub is_worktree: bool,
    pub main_repo_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
}

/// Get the branch name for a worktree by opening the repo at that path
fn get_worktree_branch(wt_path: &Path) -> Option<String> {
    Repository::open(wt_path)
        .ok()?
        .head()
        .ok()?
        .shorthand()
        .map(|s| s.to_string())
}

/// List all worktrees for a repository, including the main working tree
pub fn list_worktrees(repo_path: String) -> Result<Vec<WorktreeInfo>, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a standard repository (bare repo?)")?;

    let mut result = Vec::new();

    // Add main working tree
    let main_branch = get_worktree_branch(repo_root);
    result.push(WorktreeInfo {
        name: repo_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "main".to_string()),
        path: repo_root.to_string_lossy().to_string(),
        branch: main_branch,
        is_locked: false,
        is_main: true,
        is_valid: true,
    });

    // List linked worktrees
    let worktrees = repo.worktrees().map_err(|e| e.to_string())?;
    for wt_name in worktrees.iter() {
        let wt_name = match wt_name {
            Some(name) => name.to_string(),
            None => continue,
        };

        let wt = match repo.find_worktree(&wt_name) {
            Ok(wt) => wt,
            Err(_) => continue,
        };

        let wt_path = wt.path();
        let is_valid = wt.validate().is_ok();
        let is_locked = !matches!(wt.is_locked(), Ok(git2::WorktreeLockStatus::Unlocked));
        let branch = if is_valid {
            get_worktree_branch(wt_path)
        } else {
            None
        };

        result.push(WorktreeInfo {
            name: wt_name,
            path: wt_path.to_string_lossy().to_string(),
            branch,
            is_locked,
            is_main: false,
            is_valid,
        });
    }

    Ok(result)
}

/// Create a new worktree
/// If `new_branch` is true, creates a new branch from HEAD with the given `branch` name.
/// If `new_branch` is false, uses an existing branch specified by `branch`.
/// The worktree is created at `../{repo_dir_name}-{name}/` relative to the repo.
pub fn create_worktree(
    repo_path: String,
    name: String,
    branch: Option<String>,
    new_branch: bool,
) -> Result<WorktreeInfo, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;
    let repo_root = repo
        .workdir()
        .ok_or("Not a standard repository (bare repo?)")?;

    // Calculate worktree path: parent_dir / {repo_name}-{worktree_name}
    let parent_dir = repo_root
        .parent()
        .ok_or("Cannot determine parent directory")?;
    let repo_dir_name = repo_root
        .file_name()
        .ok_or("Cannot determine repository directory name")?
        .to_string_lossy();
    let wt_path = parent_dir.join(format!("{}-{}", repo_dir_name, name));

    if wt_path.exists() {
        return Err(format!(
            "Directory already exists: {}",
            wt_path.display()
        ));
    }

    let branch_name = branch.unwrap_or_else(|| name.clone());

    if new_branch {
        // Check if branch already exists
        let branch_exists = repo
            .find_branch(&branch_name, git2::BranchType::Local)
            .is_ok();

        if !branch_exists {
            // Create new branch from HEAD
            let head = repo.head().map_err(|e| e.to_string())?;
            let head_commit = head.peel_to_commit().map_err(|e| e.to_string())?;

            // Create the branch
            repo.branch(&branch_name, &head_commit, false)
                .map_err(|e| e.to_string())?;
        }
        // If branch exists, just use it for the worktree
    }

    // Create the worktree
    // git2's worktree API requires a reference
    let reference = repo
        .find_branch(&branch_name, git2::BranchType::Local)
        .map_err(|e| format!("Branch '{}' not found: {}", branch_name, e))?;

    let ref_name = reference
        .get()
        .name()
        .ok_or("Invalid branch reference name")?;

    repo.worktree(&name, &wt_path, Some(&git2::WorktreeAddOptions::new().reference(
        Some(&repo.find_reference(ref_name).map_err(|e| e.to_string())?),
    )))
    .map_err(|e| e.to_string())?;

    let wt_branch = get_worktree_branch(&wt_path);

    Ok(WorktreeInfo {
        name,
        path: wt_path.to_string_lossy().to_string(),
        branch: wt_branch,
        is_locked: false,
        is_main: false,
        is_valid: true,
    })
}

/// Remove a worktree by name (prune it)
pub fn remove_worktree(repo_path: String, name: String) -> Result<(), String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;

    let wt = repo
        .find_worktree(&name)
        .map_err(|e| format!("Worktree '{}' not found: {}", name, e))?;

    let is_locked = !matches!(wt.is_locked(), Ok(git2::WorktreeLockStatus::Unlocked));
    if is_locked {
        return Err(format!("Worktree '{}' is locked", name));
    }

    // Get the worktree path before pruning
    let wt_path = wt.path().to_path_buf();

    // Prune the worktree (removes git metadata)
    wt.prune(Some(
        &mut git2::WorktreePruneOptions::new()
            .valid(true)
            .working_tree(true),
    ))
    .map_err(|e| e.to_string())?;

    // Remove the directory if it still exists
    if wt_path.exists() {
        std::fs::remove_dir_all(&wt_path)
            .map_err(|e| format!("Failed to remove worktree directory: {}", e))?;
    }

    Ok(())
}

/// Get worktree context for the current repository path.
/// Determines if the path is a worktree and provides the main repo path.
pub fn get_worktree_context(repo_path: String) -> Result<WorktreeContext, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;

    // Check if this repo is itself a worktree (not the main working tree)
    let is_worktree = repo.is_worktree();

    let main_repo_path = if is_worktree {
        // For a worktree, the path() returns the .git directory of the main repo
        // We need to go up from the worktree's gitdir to find the main repo workdir
        repo.path()
            .parent() // .git/worktrees/<name> -> .git/worktrees
            .and_then(|p| p.parent()) // .git/worktrees -> .git
            .and_then(|p| p.parent()) // .git -> repo root
            .map(|p: &Path| p.to_string_lossy().to_string())
    } else {
        repo.workdir()
            .map(|p: &Path| p.to_string_lossy().to_string())
    };

    Ok(WorktreeContext {
        is_worktree,
        main_repo_path,
    })
}

/// List local branches for a repository
pub fn list_branches(repo_path: String) -> Result<Vec<BranchInfo>, String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;

    let branches = repo
        .branches(Some(git2::BranchType::Local))
        .map_err(|e| e.to_string())?;

    let mut result = Vec::new();
    for branch_result in branches {
        let (branch, _) = branch_result.map_err(|e| e.to_string())?;
        let name = branch
            .name()
            .map_err(|e| e.to_string())?
            .unwrap_or("")
            .to_string();
        let is_head = branch.is_head();

        if !name.is_empty() {
            result.push(BranchInfo { name, is_head });
        }
    }

    // Sort: HEAD branch first, then alphabetical
    result.sort_by(|a, b| {
        if a.is_head && !b.is_head {
            std::cmp::Ordering::Less
        } else if !a.is_head && b.is_head {
            std::cmp::Ordering::Greater
        } else {
            a.name.cmp(&b.name)
        }
    });

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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

    #[test]
    fn test_list_worktrees_basic() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        let result = list_worktrees(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let worktrees = result.unwrap();
        assert_eq!(worktrees.len(), 1);
        assert!(worktrees[0].is_main);
        assert!(worktrees[0].is_valid);
        assert!(!worktrees[0].is_locked);
        assert!(worktrees[0].branch.is_some());
    }

    #[test]
    fn test_list_worktrees_not_a_repo() {
        let dir = tempdir().unwrap();
        let result = list_worktrees(dir.path().to_string_lossy().to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_create_worktree_new_branch() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        let result = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "feature-test".to_string(),
            Some("feature-test".to_string()),
            true,
        );
        assert!(result.is_ok());

        let wt = result.unwrap();
        assert_eq!(wt.name, "feature-test");
        assert_eq!(wt.branch, Some("feature-test".to_string()));
        assert!(!wt.is_main);
        assert!(wt.is_valid);

        // Verify worktree path exists
        assert!(Path::new(&wt.path).exists());

        // Verify listed in worktrees
        let list = list_worktrees(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_create_worktree_existing_branch() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Create a branch first
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("existing-branch", &head, false).unwrap();

        let result = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "wt-existing".to_string(),
            Some("existing-branch".to_string()),
            false,
        );
        assert!(result.is_ok());

        let wt = result.unwrap();
        assert_eq!(wt.branch, Some("existing-branch".to_string()));
    }

    #[test]
    fn test_create_worktree_new_branch_already_exists() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Create a branch first
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("pre-existing", &head, false).unwrap();

        // Try to create worktree with new_branch=true for an existing branch
        // This should succeed by reusing the existing branch
        let result = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "wt-reuse".to_string(),
            Some("pre-existing".to_string()),
            true, // new_branch=true, but branch already exists
        );
        assert!(result.is_ok(), "Should succeed by reusing existing branch");

        let wt = result.unwrap();
        assert_eq!(wt.name, "wt-reuse");
        assert_eq!(wt.branch, Some("pre-existing".to_string()));
    }

    #[test]
    fn test_create_worktree_directory_exists() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        // Pre-create the target directory
        let parent = dir.path().parent().unwrap();
        let repo_name = dir.path().file_name().unwrap().to_string_lossy();
        let wt_path = parent.join(format!("{}-conflict", repo_name));
        fs::create_dir_all(&wt_path).unwrap();

        let result = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "conflict".to_string(),
            Some("conflict".to_string()),
            true,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_create_worktree_nonexistent_branch() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        let result = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "wt-none".to_string(),
            Some("nonexistent-branch".to_string()),
            false,
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_remove_worktree() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        // Create a worktree first
        let wt = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "to-remove".to_string(),
            Some("to-remove".to_string()),
            true,
        )
        .unwrap();

        let wt_path = wt.path.clone();
        assert!(Path::new(&wt_path).exists());

        // Remove it
        let result = remove_worktree(
            dir.path().to_string_lossy().to_string(),
            "to-remove".to_string(),
        );
        assert!(result.is_ok(), "remove_worktree failed: {:?}", result.err());

        // Verify directory is removed
        assert!(!Path::new(&wt_path).exists());

        // Verify not in list
        let list = list_worktrees(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn test_remove_worktree_not_found() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        let result = remove_worktree(
            dir.path().to_string_lossy().to_string(),
            "nonexistent".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_get_worktree_context_main_repo() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        let result = get_worktree_context(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let ctx = result.unwrap();
        assert!(!ctx.is_worktree);
        assert!(ctx.main_repo_path.is_some());
    }

    #[test]
    fn test_get_worktree_context_linked_worktree() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        let wt = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "linked".to_string(),
            Some("linked".to_string()),
            true,
        )
        .unwrap();

        let result = get_worktree_context(wt.path.clone());
        assert!(result.is_ok());

        let ctx = result.unwrap();
        assert!(ctx.is_worktree);
        assert!(ctx.main_repo_path.is_some());
    }

    #[test]
    fn test_get_worktree_context_not_a_repo() {
        let dir = tempdir().unwrap();
        let result = get_worktree_context(dir.path().to_string_lossy().to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_list_branches_basic() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Create additional branches
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature-a", &head, false).unwrap();
        repo.branch("feature-b", &head, false).unwrap();

        let result = list_branches(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let branches = result.unwrap();
        assert_eq!(branches.len(), 3); // master + feature-a + feature-b

        // HEAD branch should be first
        assert!(branches[0].is_head);

        // All names should be present
        let names: Vec<&str> = branches.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"feature-a"));
        assert!(names.contains(&"feature-b"));
    }

    #[test]
    fn test_list_branches_not_a_repo() {
        let dir = tempdir().unwrap();
        let result = list_branches(dir.path().to_string_lossy().to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_list_branches_empty_repo() {
        let dir = tempdir().unwrap();
        Repository::init(dir.path()).unwrap();

        // No commits = no branches
        let result = list_branches(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_get_worktree_branch_valid_path() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        let branch = get_worktree_branch(dir.path());
        assert!(branch.is_some());
    }

    #[test]
    fn test_get_worktree_branch_invalid_path() {
        let dir = tempdir().unwrap();
        let branch = get_worktree_branch(dir.path());
        assert!(branch.is_none());
    }

    #[test]
    fn test_worktree_info_serialization() {
        let info = WorktreeInfo {
            name: "test-wt".to_string(),
            path: "/tmp/test-wt".to_string(),
            branch: Some("feature".to_string()),
            is_locked: false,
            is_main: false,
            is_valid: true,
        };
        assert_eq!(info.name, "test-wt");
        assert_eq!(info.branch, Some("feature".to_string()));
        assert!(!info.is_main);
    }

    #[test]
    fn test_worktree_context_serialization() {
        let ctx = WorktreeContext {
            is_worktree: true,
            main_repo_path: Some("/tmp/repo".to_string()),
        };
        assert!(ctx.is_worktree);
        assert_eq!(ctx.main_repo_path, Some("/tmp/repo".to_string()));
    }

    #[test]
    fn test_branch_info_serialization() {
        let info = BranchInfo {
            name: "main".to_string(),
            is_head: true,
        };
        assert_eq!(info.name, "main");
        assert!(info.is_head);
    }
}
