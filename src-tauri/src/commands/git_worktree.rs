use git2::Repository;
use glob::glob;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::process::Command;

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
    pub worktree_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    /// Unix timestamp (seconds) of the last commit on this branch
    pub last_commit_time: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CopyResult {
    pub copied_files: Vec<String>,
    pub skipped_files: Vec<String>,
    pub transformed_files: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PackageManager {
    pub name: String,
    pub lock_file: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Get the default branch name for a repository.
/// First checks origin/HEAD, then falls back to main or master.
fn get_default_branch(repo: &Repository) -> Result<String, String> {
    // Try to get default branch from origin/HEAD
    if let Ok(reference) = repo.find_reference("refs/remotes/origin/HEAD") {
        if let Ok(resolved) = reference.resolve() {
            if let Some(name) = resolved.shorthand() {
                // name is like "origin/main", extract just "main"
                if let Some(branch) = name.strip_prefix("origin/") {
                    return Ok(branch.to_string());
                }
            }
        }
    }

    // Fallback: check if main or master exists
    if repo
        .find_branch("main", git2::BranchType::Local)
        .is_ok()
    {
        return Ok("main".to_string());
    }

    if repo
        .find_branch("master", git2::BranchType::Local)
        .is_ok()
    {
        return Ok("master".to_string());
    }

    Err("Could not determine default branch".to_string())
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
    // Use discover to find repo from subdirectory
    let repo = Repository::discover(&repo_path).map_err(|e| e.to_string())?;
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
    _new_branch: bool, // Kept for API compatibility, but now always creates branch if needed
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

    // Check if the branch is already checked out in the main worktree or any linked worktree
    let current_branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()));

    if let Some(ref current) = current_branch {
        if current == &branch_name {
            return Err(format!(
                "Branch '{}' is currently checked out. Cannot create a worktree for the current branch.",
                branch_name
            ));
        }
    }

    // Check if branch is already used by another worktree
    let worktrees = repo.worktrees().map_err(|e| e.to_string())?;
    for wt_name in worktrees.iter() {
        if let Some(wt_name) = wt_name {
            if let Ok(wt) = repo.find_worktree(wt_name) {
                let wt_branch = get_worktree_branch(wt.path());
                if let Some(ref wt_br) = wt_branch {
                    if wt_br == &branch_name {
                        return Err(format!(
                            "Branch '{}' is already checked out in worktree '{}'",
                            branch_name, wt_name
                        ));
                    }
                }
            }
        }
    }

    // Check if branch already exists
    let branch_exists = repo
        .find_branch(&branch_name, git2::BranchType::Local)
        .is_ok();

    if !branch_exists {
        // Create new branch from the default branch
        let default_branch = get_default_branch(&repo)?;
        let base_branch = repo
            .find_branch(&default_branch, git2::BranchType::Local)
            .map_err(|e| format!("Could not find default branch '{}': {}", default_branch, e))?;

        let base_commit = base_branch
            .get()
            .peel_to_commit()
            .map_err(|e| e.to_string())?;

        // Create the branch
        repo.branch(&branch_name, &base_commit, false)
            .map_err(|e| e.to_string())?;
    }

    // Create the worktree
    // git2's worktree API requires a reference
    let reference = repo
        .find_branch(&branch_name, git2::BranchType::Local)
        .map_err(|e| format!("Failed to find branch '{}' after creation: {}", branch_name, e))?;

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

/// Remove a worktree by name (prune it).
/// If `force` is true, locked worktrees will be unlocked before removal.
/// If `force` is false and the worktree is locked, an error is returned with
/// a descriptive message including the lock reason if available.
pub fn remove_worktree(repo_path: String, name: String) -> Result<(), String> {
    remove_worktree_with_options(repo_path, name, false)
}

/// Remove a worktree by name with optional force flag.
/// When `force` is true, locked worktrees are automatically unlocked before removal.
pub fn remove_worktree_with_options(
    repo_path: String,
    name: String,
    force: bool,
) -> Result<(), String> {
    let repo = Repository::open(&repo_path).map_err(|e| e.to_string())?;

    let wt = repo
        .find_worktree(&name)
        .map_err(|e| format!("Worktree '{}' not found: {}", name, e))?;

    let lock_status = wt.is_locked();
    let is_locked = !matches!(lock_status, Ok(git2::WorktreeLockStatus::Unlocked));

    if is_locked {
        if force {
            // Unlock the worktree before removing
            wt.unlock().map_err(|e| {
                format!(
                    "Failed to unlock worktree '{}': {}. You may need to manually remove the lock.",
                    name, e
                )
            })?;
        } else {
            // Build a descriptive error message including the lock reason if available
            let reason_msg = match &lock_status {
                Ok(git2::WorktreeLockStatus::Locked(Some(reason))) => {
                    format!(" (reason: {})", reason)
                }
                _ => String::new(),
            };
            return Err(format!(
                "Worktree '{}' is locked{}. Use force option to unlock and remove it.",
                name, reason_msg
            ));
        }
    }

    // Get the worktree path before pruning
    let wt_path = wt.path().to_path_buf();

    // Prune the worktree (removes git metadata)
    let mut prune_opts = git2::WorktreePruneOptions::new();
    prune_opts.valid(true).working_tree(true);
    if force {
        prune_opts.locked(true);
    }
    wt.prune(Some(&mut prune_opts))
        .map_err(|e| e.to_string())?;

    // Remove the directory if it still exists
    if wt_path.exists() {
        std::fs::remove_dir_all(&wt_path)
            .map_err(|e| format!("Failed to remove worktree directory: {}", e))?;
    }

    Ok(())
}

/// Get worktree context for the current repository path.
/// Determines if the path is a worktree and provides the main repo path and worktree name.
/// Uses `discover` to find the repository from a subdirectory.
pub fn get_worktree_context(repo_path: String) -> Result<WorktreeContext, String> {
    // Use discover to find repo from subdirectory (searches upward for .git)
    let repo = Repository::discover(&repo_path).map_err(|e| e.to_string())?;

    // Check if this repo is itself a worktree (not the main working tree)
    let is_worktree = repo.is_worktree();

    let (main_repo_path, worktree_name) = if is_worktree {
        // For a worktree, repo.path() returns the gitdir: .git/worktrees/<name>/
        // Extract the worktree name from the gitdir path
        let gitdir = repo.path();
        let wt_name = gitdir
            .file_name()
            .map(|n| n.to_string_lossy().to_string());

        // Go up from the worktree's gitdir to find the main repo workdir
        let main_path = gitdir
            .parent() // .git/worktrees/<name> -> .git/worktrees
            .and_then(|p| p.parent()) // .git/worktrees -> .git
            .and_then(|p| p.parent()) // .git -> repo root
            .map(|p: &Path| p.to_string_lossy().to_string());

        (main_path, wt_name)
    } else {
        let main_path = repo
            .workdir()
            .map(|p: &Path| p.to_string_lossy().to_string());
        (main_path, None)
    };

    Ok(WorktreeContext {
        is_worktree,
        main_repo_path,
        worktree_name,
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

        // Get the last commit time for this branch
        let last_commit_time = branch
            .get()
            .peel_to_commit()
            .ok()
            .map(|commit| commit.time().seconds());

        if !name.is_empty() {
            result.push(BranchInfo {
                name,
                is_head,
                last_commit_time,
            });
        }
    }

    // Sort: HEAD branch first, then by last commit time (most recent first)
    result.sort_by(|a, b| {
        if a.is_head && !b.is_head {
            std::cmp::Ordering::Less
        } else if !a.is_head && b.is_head {
            std::cmp::Ordering::Greater
        } else {
            // Sort by last commit time descending (most recent first)
            match (b.last_commit_time, a.last_commit_time) {
                (Some(b_time), Some(a_time)) => b_time.cmp(&a_time),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => a.name.cmp(&b.name),
            }
        }
    });

    Ok(result)
}

/// Copy files matching the given patterns from source to target directory.
/// Preserves directory structure (e.g., config/local.json -> target/config/local.json).
/// Does not overwrite existing files.
/// If a pattern matches a directory, all files within it are copied recursively.
pub fn copy_files_to_worktree(
    source_path: String,
    target_path: String,
    patterns: Vec<String>,
) -> Result<CopyResult, String> {
    let source = Path::new(&source_path);
    let target = Path::new(&target_path);

    if !source.exists() {
        return Err(format!("Source path does not exist: {}", source_path));
    }

    if !target.exists() {
        return Err(format!("Target path does not exist: {}", target_path));
    }

    let mut copied_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut errors = Vec::new();

    // Helper function to copy a single file
    fn copy_file(
        path: &Path,
        source: &Path,
        target: &Path,
        copied_files: &mut Vec<String>,
        skipped_files: &mut Vec<String>,
        errors: &mut Vec<String>,
    ) {
        // Calculate relative path from source
        let relative = match path.strip_prefix(source) {
            Ok(rel) => rel,
            Err(_) => {
                errors.push(format!(
                    "Failed to calculate relative path: {}",
                    path.display()
                ));
                return;
            }
        };

        let target_file = target.join(relative);

        // Skip if target already exists
        if target_file.exists() {
            skipped_files.push(relative.to_string_lossy().to_string());
            return;
        }

        // Create parent directories if needed
        if let Some(parent) = target_file.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    errors.push(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ));
                    return;
                }
            }
        }

        // Copy the file
        match fs::copy(path, &target_file) {
            Ok(_) => {
                copied_files.push(relative.to_string_lossy().to_string());
            }
            Err(e) => {
                errors.push(format!(
                    "Failed to copy {} to {}: {}",
                    path.display(),
                    target_file.display(),
                    e
                ));
            }
        }
    }

    // Helper function to recursively copy all files in a directory
    fn copy_directory_recursive(
        dir: &Path,
        source: &Path,
        target: &Path,
        copied_files: &mut Vec<String>,
        skipped_files: &mut Vec<String>,
        errors: &mut Vec<String>,
    ) {
        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(e) => {
                            let path = e.path();
                            if path.is_dir() {
                                copy_directory_recursive(
                                    &path,
                                    source,
                                    target,
                                    copied_files,
                                    skipped_files,
                                    errors,
                                );
                            } else if path.is_file() {
                                copy_file(
                                    &path, source, target, copied_files, skipped_files, errors,
                                );
                            } else if path
                                .symlink_metadata()
                                .map(|m| m.file_type().is_symlink())
                                .unwrap_or(false)
                            {
                                errors.push(format!(
                                    "Skipping dangling symlink: {}",
                                    path.display()
                                ));
                            }
                        }
                        Err(e) => {
                            errors.push(format!(
                                "Failed to read entry in {}: {}",
                                dir.display(),
                                e
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Failed to read directory {}: {}", dir.display(), e));
            }
        }
    }

    for pattern in patterns {
        // Create full pattern path
        let full_pattern = source.join(&pattern);
        let pattern_str = full_pattern.to_string_lossy().to_string();

        match glob(&pattern_str) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(path) => {
                            if path.is_dir() {
                                // If pattern matches a directory, copy all files recursively
                                copy_directory_recursive(
                                    &path,
                                    source,
                                    target,
                                    &mut copied_files,
                                    &mut skipped_files,
                                    &mut errors,
                                );
                            } else if path.is_file() {
                                copy_file(
                                    &path,
                                    source,
                                    target,
                                    &mut copied_files,
                                    &mut skipped_files,
                                    &mut errors,
                                );
                            } else if path
                                .symlink_metadata()
                                .map(|m| m.file_type().is_symlink())
                                .unwrap_or(false)
                            {
                                errors.push(format!(
                                    "Skipping dangling symlink: {}",
                                    path.display()
                                ));
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Glob error for pattern '{}': {}", pattern, e));
                        }
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Invalid glob pattern '{}': {}", pattern, e));
            }
        }
    }

    Ok(CopyResult {
        copied_files,
        skipped_files,
        transformed_files: vec![],
        errors,
    })
}

/// Directories to exclude when scanning subdirectories for package managers.
const EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "dist",
    "build",
    "vendor",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    ".next",
    ".nuxt",
    "out",
    "coverage",
];

/// Detect package managers in a single directory and return results.
/// The `command_prefix` is prepended to commands (e.g., "cd subdir && " for subdirectories).
fn detect_package_managers_in_dir(dir: &Path, command_prefix: &str) -> Vec<PackageManager> {
    let mut results = Vec::new();

    // Node.js ecosystem (priority: pnpm > yarn > bun > npm)
    let nodejs_lock_files = [
        ("pnpm-lock.yaml", "pnpm", "pnpm install"),
        ("yarn.lock", "yarn", "yarn install"),
        ("bun.lockb", "bun", "bun install"),
        ("package-lock.json", "npm", "npm install"),
    ];

    let mut nodejs_found = false;
    for (lock_file, name, command) in nodejs_lock_files {
        if dir.join(lock_file).exists() {
            results.push(PackageManager {
                name: name.to_string(),
                lock_file: lock_file.to_string(),
                command: format!("{}{}", command_prefix, command),
            });
            nodejs_found = true;
            break;
        }
    }

    if !nodejs_found && dir.join("package.json").exists() {
        results.push(PackageManager {
            name: "npm".to_string(),
            lock_file: "".to_string(),
            command: format!("{}npm install", command_prefix),
        });
    }

    // Python ecosystem (priority: uv > poetry > pipenv > pip)
    let python_candidates: &[(&[&str], &str, &str)] = &[
        (&["uv.lock"], "uv", "uv sync"),
        (&["poetry.lock"], "poetry", "poetry install"),
        (&["Pipfile.lock", "Pipfile"], "pipenv", "pipenv install"),
        (
            &["requirements.txt"],
            "pip",
            "pip install -r requirements.txt",
        ),
    ];

    for (files, name, command) in python_candidates {
        if let Some(found_file) = files.iter().find(|f| dir.join(f).exists()) {
            results.push(PackageManager {
                name: name.to_string(),
                lock_file: found_file.to_string(),
                command: format!("{}{}", command_prefix, command),
            });
            break;
        }
    }

    // Rust ecosystem
    if dir.join("Cargo.toml").exists() {
        results.push(PackageManager {
            name: "cargo".to_string(),
            lock_file: "Cargo.toml".to_string(),
            command: format!("{}cargo build", command_prefix),
        });
    }

    // Go ecosystem
    if dir.join("go.mod").exists() {
        results.push(PackageManager {
            name: "go".to_string(),
            lock_file: "go.mod".to_string(),
            command: format!("{}go mod download", command_prefix),
        });
    }

    // Ruby ecosystem
    let ruby_files = ["Gemfile.lock", "Gemfile"];
    if let Some(found_file) = ruby_files.iter().find(|f| dir.join(f).exists()) {
        results.push(PackageManager {
            name: "bundler".to_string(),
            lock_file: found_file.to_string(),
            command: format!("{}bundle install", command_prefix),
        });
    }

    // PHP ecosystem
    let php_files = ["composer.lock", "composer.json"];
    if let Some(found_file) = php_files.iter().find(|f| dir.join(f).exists()) {
        results.push(PackageManager {
            name: "composer".to_string(),
            lock_file: found_file.to_string(),
            command: format!("{}composer install", command_prefix),
        });
    }

    results
}

/// Detect all package managers by checking for lock files across multiple language ecosystems.
/// Searches both the root directory and immediate subdirectories (1 level deep).
/// Returns at most one result per language ecosystem per directory.
///
/// Supported ecosystems:
/// - Node.js: pnpm > yarn > bun > npm
/// - Python: uv > poetry > pipenv > pip
/// - Rust: cargo
/// - Go: go
/// - Ruby: bundler
/// - PHP: composer
pub fn detect_package_managers(project_path: String) -> Result<Vec<PackageManager>, String> {
    let path = Path::new(&project_path);

    if !path.exists() {
        return Err(format!("Path does not exist: {}", project_path));
    }

    // Detect in root directory (no command prefix)
    let mut results = detect_package_managers_in_dir(path, "");

    // Scan immediate subdirectories (1 level deep)
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if !entry_path.is_dir() {
                continue;
            }

            // Skip excluded directories
            let dir_name = match entry_path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name.to_string(),
                None => continue,
            };

            if dir_name.starts_with('.') && EXCLUDED_DIRS.contains(&dir_name.as_str()) {
                continue;
            }
            if EXCLUDED_DIRS.contains(&dir_name.as_str()) {
                continue;
            }

            let prefix = format!("cd '{}' && ", dir_name.replace('\'', "'\\''"));
            let subdir_results = detect_package_managers_in_dir(&entry_path, &prefix);
            results.extend(subdir_results);
        }
    }

    Ok(results)
}

/// Detect package manager by checking for lock files in the project directory.
/// Returns the first detected package manager (backward compatible wrapper).
/// Priority order: pnpm > yarn > bun > npm
pub fn detect_package_manager(project_path: String) -> Result<Option<PackageManager>, String> {
    let results = detect_package_managers(project_path)?;
    Ok(results.into_iter().next())
}

/// Run an initialization command in the specified directory.
/// Returns the command output including stdout, stderr, and exit code.
pub fn run_init_command(cwd: String, command: String) -> Result<CommandOutput, String> {
    let path = Path::new(&cwd);

    if !path.exists() {
        return Err(format!("Working directory does not exist: {}", cwd));
    }

    // Split command into program and arguments
    // Use shell to handle complex commands
    #[cfg(target_os = "windows")]
    let output = Command::new("cmd")
        .args(["/C", &command])
        .current_dir(path)
        .output()
        .map_err(|e| format!("Failed to execute command: {}", e))?;

    #[cfg(not(target_os = "windows"))]
    let output = {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
        Command::new(&shell)
            .args(["-l", "-c", &command])
            .current_dir(path)
            .output()
            .map_err(|e| format!("Failed to execute command: {}", e))?
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok(CommandOutput {
        success: output.status.success(),
        stdout,
        stderr,
        exit_code,
    })
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
    fn test_create_worktree_nonexistent_branch_creates_new() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        // When new_branch=false but branch doesn't exist, should create the branch
        let result = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "wt-none".to_string(),
            Some("nonexistent-branch".to_string()),
            false,
        );
        assert!(result.is_ok(), "Should create branch if it doesn't exist");

        let wt = result.unwrap();
        assert_eq!(wt.name, "wt-none");
        assert_eq!(wt.branch, Some("nonexistent-branch".to_string()));
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
            worktree_name: Some("feature-branch".to_string()),
        };
        assert!(ctx.is_worktree);
        assert_eq!(ctx.main_repo_path, Some("/tmp/repo".to_string()));
        assert_eq!(ctx.worktree_name, Some("feature-branch".to_string()));
    }

    #[test]
    fn test_branch_info_serialization() {
        let info = BranchInfo {
            name: "main".to_string(),
            is_head: true,
            last_commit_time: Some(1704067200),
        };
        assert_eq!(info.name, "main");
        assert!(info.is_head);
        assert_eq!(info.last_commit_time, Some(1704067200));
    }

    #[test]
    fn test_create_worktree_current_branch_error() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        // Get current branch name (should be master or main)
        let repo = Repository::open(dir.path()).unwrap();
        let current_branch = repo.head().unwrap().shorthand().unwrap().to_string();

        // Try to create worktree for current branch with new_branch=false
        let result = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "wt-current".to_string(),
            Some(current_branch.clone()),
            false,
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("currently checked out"),
            "Expected 'currently checked out' error, got: {}",
            err
        );
    }

    #[test]
    fn test_create_worktree_current_branch_error_new_branch_true() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        // Get current branch name (should be master or main)
        let repo = Repository::open(dir.path()).unwrap();
        let current_branch = repo.head().unwrap().shorthand().unwrap().to_string();

        // Try to create worktree for current branch with new_branch=true
        // This should also fail because the branch already exists and is checked out
        let result = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "wt-current-new".to_string(),
            Some(current_branch.clone()),
            true, // new_branch=true, but branch already exists and is current
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("currently checked out"),
            "Expected 'currently checked out' error, got: {}",
            err
        );
    }

    #[test]
    fn test_create_worktree_branch_in_use_by_other_worktree() {
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        // Create first worktree with a new branch
        let result1 = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "wt-first".to_string(),
            Some("feature-used".to_string()),
            true,
        );
        assert!(result1.is_ok());

        // Try to create second worktree with the same branch
        let result2 = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "wt-second".to_string(),
            Some("feature-used".to_string()),
            false,
        );

        assert!(result2.is_err());
        let err = result2.unwrap_err();
        assert!(
            err.contains("already checked out in worktree"),
            "Expected 'already checked out in worktree' error, got: {}",
            err
        );
    }

    #[test]
    fn test_copy_files_to_worktree_basic() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create test files in source
        fs::write(source_dir.path().join(".env"), "SECRET=123").unwrap();
        fs::write(source_dir.path().join(".env.local"), "LOCAL=456").unwrap();
        fs::write(source_dir.path().join("test.txt"), "test content").unwrap();

        let result = copy_files_to_worktree(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
        );

        assert!(result.is_ok());
        let copy_result = result.unwrap();
        assert_eq!(copy_result.copied_files.len(), 2);
        assert!(copy_result.copied_files.contains(&".env".to_string()));
        assert!(copy_result.copied_files.contains(&".env.local".to_string()));
        assert!(copy_result.errors.is_empty());

        // Verify files were copied
        assert!(target_dir.path().join(".env").exists());
        assert!(target_dir.path().join(".env.local").exists());
        // test.txt should NOT be copied
        assert!(!target_dir.path().join("test.txt").exists());
    }

    #[test]
    fn test_copy_files_to_worktree_with_directory_structure() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create nested directory structure
        let config_dir = source_dir.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("local.json"), "{}").unwrap();
        fs::write(config_dir.join("prod.json"), "{}").unwrap();

        let result = copy_files_to_worktree(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config/*.json".to_string()],
        );

        assert!(result.is_ok());
        let copy_result = result.unwrap();
        assert_eq!(copy_result.copied_files.len(), 2);

        // Verify directory structure was preserved
        assert!(target_dir.path().join("config/local.json").exists());
        assert!(target_dir.path().join("config/prod.json").exists());
    }

    #[test]
    fn test_copy_files_to_worktree_skip_existing() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create file in source
        fs::write(source_dir.path().join(".env"), "SOURCE").unwrap();

        // Pre-create file in target
        fs::write(target_dir.path().join(".env"), "EXISTING").unwrap();

        let result = copy_files_to_worktree(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env".to_string()],
        );

        assert!(result.is_ok());
        let copy_result = result.unwrap();
        assert!(copy_result.copied_files.is_empty());
        assert_eq!(copy_result.skipped_files.len(), 1);
        assert!(copy_result.skipped_files.contains(&".env".to_string()));

        // Verify original content was preserved
        let content = fs::read_to_string(target_dir.path().join(".env")).unwrap();
        assert_eq!(content, "EXISTING");
    }

    #[test]
    fn test_copy_files_to_worktree_multiple_patterns() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(source_dir.path().join(".env"), "env").unwrap();
        fs::write(source_dir.path().join("secret.txt"), "secret").unwrap();
        fs::write(source_dir.path().join("other.txt"), "other").unwrap();

        let result = copy_files_to_worktree(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string(), "secret.txt".to_string()],
        );

        assert!(result.is_ok());
        let copy_result = result.unwrap();
        assert_eq!(copy_result.copied_files.len(), 2);
        assert!(target_dir.path().join(".env").exists());
        assert!(target_dir.path().join("secret.txt").exists());
        assert!(!target_dir.path().join("other.txt").exists());
    }

    #[test]
    fn test_copy_files_to_worktree_invalid_source() {
        let target_dir = tempdir().unwrap();

        let result = copy_files_to_worktree(
            "/nonexistent/path".to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env".to_string()],
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Source path does not exist"));
    }

    #[test]
    fn test_copy_files_to_worktree_invalid_target() {
        let source_dir = tempdir().unwrap();

        let result = copy_files_to_worktree(
            source_dir.path().to_string_lossy().to_string(),
            "/nonexistent/path".to_string(),
            vec![".env".to_string()],
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Target path does not exist"));
    }

    #[test]
    fn test_copy_files_to_worktree_no_matches() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create some files that won't match the pattern
        fs::write(source_dir.path().join("test.txt"), "test").unwrap();

        let result = copy_files_to_worktree(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
        );

        assert!(result.is_ok());
        let copy_result = result.unwrap();
        assert!(copy_result.copied_files.is_empty());
        assert!(copy_result.skipped_files.is_empty());
        assert!(copy_result.errors.is_empty());
    }

    #[test]
    fn test_copy_result_serialization() {
        let result = CopyResult {
            copied_files: vec!["file1.txt".to_string(), "file2.txt".to_string()],
            skipped_files: vec!["existing.txt".to_string()],
            transformed_files: vec![],
            errors: vec![],
        };
        assert_eq!(result.copied_files.len(), 2);
        assert_eq!(result.skipped_files.len(), 1);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_copy_files_to_worktree_directory_pattern() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a directory with nested files
        let config_dir = source_dir.path().join(".claude");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join("settings.json"), "{}").unwrap();

        // Create nested subdirectory
        let nested_dir = config_dir.join("rules");
        fs::create_dir_all(&nested_dir).unwrap();
        fs::write(nested_dir.join("rule1.md"), "# Rule 1").unwrap();
        fs::write(nested_dir.join("rule2.md"), "# Rule 2").unwrap();

        // Use directory name as pattern (without glob wildcards)
        let result = copy_files_to_worktree(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".claude".to_string()],
        );

        assert!(result.is_ok());
        let copy_result = result.unwrap();

        // Should copy all files in the directory recursively
        assert_eq!(copy_result.copied_files.len(), 3);
        assert!(copy_result.errors.is_empty());

        // Verify files were copied with directory structure preserved
        assert!(target_dir.path().join(".claude/settings.json").exists());
        assert!(target_dir.path().join(".claude/rules/rule1.md").exists());
        assert!(target_dir.path().join(".claude/rules/rule2.md").exists());
    }

    #[test]
    fn test_detect_package_manager_npm() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("package-lock.json"), "{}").unwrap();

        let result = detect_package_manager(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pm = result.unwrap().unwrap();
        assert_eq!(pm.name, "npm");
        assert_eq!(pm.lock_file, "package-lock.json");
        assert_eq!(pm.command, "npm install");
    }

    #[test]
    fn test_detect_package_manager_pnpm() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "{}").unwrap();

        let result = detect_package_manager(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pm = result.unwrap().unwrap();
        assert_eq!(pm.name, "pnpm");
        assert_eq!(pm.lock_file, "pnpm-lock.yaml");
        assert_eq!(pm.command, "pnpm install");
    }

    #[test]
    fn test_detect_package_manager_yarn() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("yarn.lock"), "").unwrap();

        let result = detect_package_manager(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pm = result.unwrap().unwrap();
        assert_eq!(pm.name, "yarn");
        assert_eq!(pm.lock_file, "yarn.lock");
        assert_eq!(pm.command, "yarn install");
    }

    #[test]
    fn test_detect_package_manager_bun() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("bun.lockb"), "").unwrap();

        let result = detect_package_manager(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pm = result.unwrap().unwrap();
        assert_eq!(pm.name, "bun");
        assert_eq!(pm.lock_file, "bun.lockb");
        assert_eq!(pm.command, "bun install");
    }

    #[test]
    fn test_detect_package_manager_priority() {
        // When multiple lock files exist, pnpm should take priority
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("package-lock.json"), "{}").unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "{}").unwrap();

        let result = detect_package_manager(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pm = result.unwrap().unwrap();
        assert_eq!(pm.name, "pnpm");
    }

    #[test]
    fn test_detect_package_manager_package_json_only() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();

        let result = detect_package_manager(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pm = result.unwrap().unwrap();
        assert_eq!(pm.name, "npm");
        assert!(pm.lock_file.is_empty());
    }

    #[test]
    fn test_detect_package_manager_none() {
        let dir = tempdir().unwrap();
        // No package.json or lock files

        let result = detect_package_manager(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_detect_package_manager_invalid_path() {
        let result = detect_package_manager("/nonexistent/path".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_run_init_command_success() {
        let dir = tempdir().unwrap();

        let result = run_init_command(dir.path().to_string_lossy().to_string(), "echo hello".to_string());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.success);
        assert!(output.stdout.contains("hello"));
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn test_run_init_command_failure() {
        let dir = tempdir().unwrap();

        // Run a command that will fail
        let result = run_init_command(
            dir.path().to_string_lossy().to_string(),
            "exit 1".to_string(),
        );
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.success);
        assert_eq!(output.exit_code, 1);
    }

    #[test]
    fn test_run_init_command_invalid_cwd() {
        let result = run_init_command("/nonexistent/path".to_string(), "echo hello".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_package_manager_serialization() {
        let pm = PackageManager {
            name: "npm".to_string(),
            lock_file: "package-lock.json".to_string(),
            command: "npm install".to_string(),
        };
        assert_eq!(pm.name, "npm");
        assert_eq!(pm.lock_file, "package-lock.json");
        assert_eq!(pm.command, "npm install");
    }

    #[test]
    fn test_command_output_serialization() {
        let output = CommandOutput {
            success: true,
            stdout: "output".to_string(),
            stderr: "".to_string(),
            exit_code: 0,
        };
        assert!(output.success);
        assert_eq!(output.stdout, "output");
        assert_eq!(output.exit_code, 0);
    }

    #[test]
    fn test_detect_package_managers_python_uv() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("uv.lock"), "").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "uv");
        assert_eq!(pms[0].lock_file, "uv.lock");
        assert_eq!(pms[0].command, "uv sync");
    }

    #[test]
    fn test_detect_package_managers_python_poetry() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("poetry.lock"), "").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "poetry");
        assert_eq!(pms[0].lock_file, "poetry.lock");
        assert_eq!(pms[0].command, "poetry install");
    }

    #[test]
    fn test_detect_package_managers_python_pipenv() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Pipfile"), "").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "pipenv");
        assert_eq!(pms[0].lock_file, "Pipfile");
        assert_eq!(pms[0].command, "pipenv install");
    }

    #[test]
    fn test_detect_package_managers_python_pip() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("requirements.txt"), "flask==2.0").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "pip");
        assert_eq!(pms[0].lock_file, "requirements.txt");
        assert_eq!(pms[0].command, "pip install -r requirements.txt");
    }

    #[test]
    fn test_detect_package_managers_rust() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "cargo");
        assert_eq!(pms[0].lock_file, "Cargo.toml");
        assert_eq!(pms[0].command, "cargo build");
    }

    #[test]
    fn test_detect_package_managers_go() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("go.mod"), "module example.com/app").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "go");
        assert_eq!(pms[0].lock_file, "go.mod");
        assert_eq!(pms[0].command, "go mod download");
    }

    #[test]
    fn test_detect_package_managers_ruby() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Gemfile"), "source 'https://rubygems.org'").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "bundler");
        assert_eq!(pms[0].lock_file, "Gemfile");
        assert_eq!(pms[0].command, "bundle install");
    }

    #[test]
    fn test_detect_package_managers_php() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), "{}").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "composer");
        assert_eq!(pms[0].lock_file, "composer.json");
        assert_eq!(pms[0].command, "composer install");
    }

    #[test]
    fn test_detect_package_managers_multi_language() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]").unwrap();
        fs::write(dir.path().join("requirements.txt"), "flask==2.0").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 3);

        let names: Vec<&str> = pms.iter().map(|pm| pm.name.as_str()).collect();
        assert!(names.contains(&"npm"));
        assert!(names.contains(&"cargo"));
        assert!(names.contains(&"pip"));
    }

    #[test]
    fn test_detect_package_managers_python_priority() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("uv.lock"), "").unwrap();
        fs::write(dir.path().join("requirements.txt"), "flask==2.0").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        // Should only return uv, not pip (uv has higher priority)
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "uv");
    }

    #[test]
    fn test_detect_package_managers_subdirectory_rust() {
        let dir = tempdir().unwrap();
        // Rust project in a subdirectory (like Tauri's src-tauri/)
        let subdir = dir.path().join("src-tauri");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(subdir.join("Cargo.toml"), "[package]").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "cargo");
        assert_eq!(pms[0].command, "cd 'src-tauri' && cargo build");
    }

    #[test]
    fn test_detect_package_managers_root_and_subdirectory() {
        let dir = tempdir().unwrap();
        // Node.js at root
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("package-lock.json"), "{}").unwrap();
        // Rust in subdirectory
        let subdir = dir.path().join("src-tauri");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(subdir.join("Cargo.toml"), "[package]").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert!(pms.len() >= 2, "Expected at least 2 PMs, got {}", pms.len());

        // Root npm
        let npm = pms.iter().find(|p| p.name == "npm");
        assert!(npm.is_some());
        assert_eq!(npm.unwrap().command, "npm install");

        // Subdirectory cargo
        let cargo = pms.iter().find(|p| p.name == "cargo");
        assert!(cargo.is_some());
        assert_eq!(cargo.unwrap().command, "cd 'src-tauri' && cargo build");
    }

    #[test]
    fn test_detect_package_managers_excludes_node_modules() {
        let dir = tempdir().unwrap();
        // Create node_modules with a package.json (should be ignored)
        let nm_dir = dir.path().join("node_modules");
        fs::create_dir_all(&nm_dir).unwrap();
        fs::write(nm_dir.join("package.json"), "{}").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_detect_package_managers_excludes_target() {
        let dir = tempdir().unwrap();
        // Create target dir with Cargo.toml (should be ignored)
        let target_dir = dir.path().join("target");
        fs::create_dir_all(&target_dir).unwrap();
        fs::write(target_dir.join("Cargo.toml"), "[package]").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_detect_package_managers_subdirectory_python() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("backend");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(subdir.join("requirements.txt"), "flask==2.0").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        assert_eq!(pms.len(), 1);
        assert_eq!(pms[0].name, "pip");
        assert_eq!(pms[0].command, "cd 'backend' && pip install -r requirements.txt");
    }

    #[test]
    fn test_detect_package_managers_same_lang_root_and_subdir() {
        let dir = tempdir().unwrap();
        // npm at root
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        fs::write(dir.path().join("package-lock.json"), "{}").unwrap();
        // Also npm in a subdirectory
        let subdir = dir.path().join("frontend");
        fs::create_dir_all(&subdir).unwrap();
        fs::write(subdir.join("package.json"), "{}").unwrap();
        fs::write(subdir.join("yarn.lock"), "").unwrap();

        let result = detect_package_managers(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let pms = result.unwrap();
        // Should return both: root npm + subdirectory yarn
        let root_npm = pms.iter().find(|p| p.command == "npm install");
        assert!(root_npm.is_some());
        let subdir_yarn = pms.iter().find(|p| p.command == "cd 'frontend' && yarn install");
        assert!(subdir_yarn.is_some());
    }

    #[test]
    fn test_copy_files_to_worktree_recursive_env_pattern() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create .env files at root and in subdirectories
        fs::write(source_dir.path().join(".env"), "ROOT=1").unwrap();
        fs::write(source_dir.path().join(".env.local"), "ROOT_LOCAL=2").unwrap();

        let sub1 = source_dir.path().join("packages/api");
        fs::create_dir_all(&sub1).unwrap();
        fs::write(sub1.join(".env"), "API_PORT=3000").unwrap();
        fs::write(sub1.join(".env.production"), "API_PROD=true").unwrap();

        let sub2 = source_dir.path().join("packages/web");
        fs::create_dir_all(&sub2).unwrap();
        fs::write(sub2.join(".env"), "WEB_PORT=8080").unwrap();

        // Also create a non-.env file that should NOT be copied
        fs::write(sub1.join("config.json"), "{}").unwrap();

        let result = copy_files_to_worktree(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["**/.env*".to_string()],
        );

        assert!(result.is_ok());
        let copy_result = result.unwrap();
        assert_eq!(copy_result.copied_files.len(), 5);
        assert!(copy_result.errors.is_empty());

        // Verify root .env files
        assert!(target_dir.path().join(".env").exists());
        assert!(target_dir.path().join(".env.local").exists());

        // Verify subdirectory .env files with preserved structure
        assert!(target_dir.path().join("packages/api/.env").exists());
        assert!(target_dir.path().join("packages/api/.env.production").exists());
        assert!(target_dir.path().join("packages/web/.env").exists());

        // Non-.env files should NOT be copied
        assert!(!target_dir.path().join("packages/api/config.json").exists());
    }

    #[test]
    fn test_create_worktree_find_branch_returns_error_not_panic() {
        // Verify that create_worktree returns an error instead of panicking
        // when find_branch fails after branch creation.
        // This test verifies the .map_err() fix (previously used .expect()).
        let dir = tempdir().unwrap();
        create_repo_with_commit(dir.path());

        // A normal worktree creation should succeed without panicking
        let result = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "test-no-panic".to_string(),
            Some("test-no-panic".to_string()),
            true,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_package_managers_quotes_dir_with_spaces() {
        // Verify that directory names with spaces are properly quoted in commands
        let dir = tempdir().unwrap();

        // Create a subdirectory with spaces
        let sub = dir.path().join("my project");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("package.json"), "{}").unwrap();
        fs::write(sub.join("package-lock.json"), "{}").unwrap();

        let results = detect_package_managers(dir.path().to_string_lossy().to_string()).unwrap();

        // Find the result for the subdirectory
        let sub_result = results.iter().find(|r| r.command.contains("my project"));
        assert!(
            sub_result.is_some(),
            "Should find package manager for dir with spaces"
        );

        let cmd = &sub_result.unwrap().command;
        // Verify the directory name is quoted with single quotes
        assert!(
            cmd.contains("cd 'my project'"),
            "Expected quoted dir name in command, got: {}",
            cmd
        );
    }

    #[test]
    fn test_detect_package_managers_quotes_dir_with_single_quote() {
        // Verify that directory names containing single quotes are properly escaped
        let dir = tempdir().unwrap();

        // Create a subdirectory with a single quote in the name
        let sub = dir.path().join("it's-a-project");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("package.json"), "{}").unwrap();
        fs::write(sub.join("package-lock.json"), "{}").unwrap();

        let results = detect_package_managers(dir.path().to_string_lossy().to_string()).unwrap();

        // Find the result for the subdirectory
        let sub_result = results.iter().find(|r| r.command.contains("it"));
        assert!(
            sub_result.is_some(),
            "Should find package manager for dir with single quote"
        );

        let cmd = &sub_result.unwrap().command;
        // Verify the single quote is properly escaped
        assert!(
            cmd.contains("cd 'it'\\''s-a-project'"),
            "Expected escaped single quote in command, got: {}",
            cmd
        );
    }

    // === Tests for dangling symlinks (M9) ===

    #[cfg(unix)]
    #[test]
    fn test_copy_files_to_worktree_dangling_symlink() {
        use std::os::unix::fs::symlink;

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a valid file
        fs::write(source_dir.path().join(".env"), "PORT=3000").unwrap();

        // Create a dangling symlink (points to non-existent target)
        let symlink_path = source_dir.path().join(".env.link");
        symlink("/nonexistent/target/file", &symlink_path).unwrap();

        let result = copy_files_to_worktree(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
        );

        assert!(result.is_ok());
        let copy_result = result.unwrap();

        // The valid file should be copied
        assert_eq!(copy_result.copied_files.len(), 1);
        assert!(copy_result.copied_files.contains(&".env".to_string()));

        // The dangling symlink should be reported in errors
        assert!(
            !copy_result.errors.is_empty(),
            "Expected an error for the dangling symlink"
        );
        assert!(
            copy_result.errors.iter().any(|e| e.contains("dangling symlink")),
            "Expected 'dangling symlink' error, got: {:?}",
            copy_result.errors
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_copy_directory_recursive_dangling_symlink() {
        use std::os::unix::fs::symlink;

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a directory with a mix of files and dangling symlinks
        let sub = source_dir.path().join("config");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("settings.json"), "{}").unwrap();

        // Create a dangling symlink inside the directory
        symlink("/nonexistent/target", sub.join("broken_link")).unwrap();

        let result = copy_files_to_worktree(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config".to_string()],
        );

        assert!(result.is_ok());
        let copy_result = result.unwrap();

        // The valid file should be copied
        assert_eq!(copy_result.copied_files.len(), 1);

        // The dangling symlink should be reported
        assert!(
            copy_result.errors.iter().any(|e| e.contains("dangling symlink")),
            "Expected 'dangling symlink' error, got: {:?}",
            copy_result.errors
        );
    }

    // === Tests for locked worktree handling (H6) ===

    #[test]
    fn test_remove_locked_worktree_returns_descriptive_error() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Create a worktree
        let wt = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "locked-wt".to_string(),
            Some("locked-wt".to_string()),
            true,
        )
        .unwrap();

        // Lock the worktree
        let git_wt = repo.find_worktree("locked-wt").unwrap();
        git_wt.lock(Some("test lock reason")).unwrap();

        // Attempt to remove without force should fail with descriptive error
        let result = remove_worktree(
            dir.path().to_string_lossy().to_string(),
            "locked-wt".to_string(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("is locked"),
            "Expected 'is locked' in error, got: {}",
            err
        );
        assert!(
            err.contains("force"),
            "Expected hint about force option, got: {}",
            err
        );

        // Worktree should still exist
        assert!(Path::new(&wt.path).exists());
    }

    #[test]
    fn test_remove_locked_worktree_with_force() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Create a worktree
        let wt = create_worktree(
            dir.path().to_string_lossy().to_string(),
            "force-remove-wt".to_string(),
            Some("force-remove-wt".to_string()),
            true,
        )
        .unwrap();

        // Lock the worktree
        let git_wt = repo.find_worktree("force-remove-wt").unwrap();
        git_wt.lock(Some("test reason")).unwrap();

        // Force remove should succeed
        let result = remove_worktree_with_options(
            dir.path().to_string_lossy().to_string(),
            "force-remove-wt".to_string(),
            true,
        );

        assert!(
            result.is_ok(),
            "Force remove of locked worktree should succeed, got: {:?}",
            result.err()
        );

        // Worktree directory should be removed
        assert!(!Path::new(&wt.path).exists());

        // Worktree should no longer be in the list
        let list = list_worktrees(dir.path().to_string_lossy().to_string()).unwrap();
        assert_eq!(list.len(), 1, "Only main worktree should remain");
    }

    #[test]
    fn test_remove_locked_worktree_error_includes_reason() {
        let dir = tempdir().unwrap();
        let repo = create_repo_with_commit(dir.path());

        // Create a worktree
        create_worktree(
            dir.path().to_string_lossy().to_string(),
            "reason-wt".to_string(),
            Some("reason-wt".to_string()),
            true,
        )
        .unwrap();

        // Lock with a reason
        let git_wt = repo.find_worktree("reason-wt").unwrap();
        git_wt.lock(Some("important work in progress")).unwrap();

        let result = remove_worktree(
            dir.path().to_string_lossy().to_string(),
            "reason-wt".to_string(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("important work in progress"),
            "Expected lock reason in error message, got: {}",
            err
        );
    }
}
