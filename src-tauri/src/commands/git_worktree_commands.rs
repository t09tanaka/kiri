use super::git_worktree::{BranchInfo, CommandOutput, CopyResult, PackageManager, WorktreeContext, WorktreeInfo};

#[tauri::command]
pub fn list_worktrees(repo_path: String) -> Result<Vec<WorktreeInfo>, String> {
    super::git_worktree::list_worktrees(repo_path)
}

#[tauri::command]
pub fn create_worktree(
    repo_path: String,
    name: String,
    branch: Option<String>,
    new_branch: bool,
) -> Result<WorktreeInfo, String> {
    super::git_worktree::create_worktree(repo_path, name, branch, new_branch)
}

#[tauri::command]
pub fn remove_worktree(repo_path: String, name: String) -> Result<(), String> {
    super::git_worktree::remove_worktree(repo_path, name)
}

#[tauri::command]
pub fn get_worktree_context(repo_path: String) -> Result<WorktreeContext, String> {
    super::git_worktree::get_worktree_context(repo_path)
}

#[tauri::command]
pub fn list_branches(repo_path: String) -> Result<Vec<BranchInfo>, String> {
    super::git_worktree::list_branches(repo_path)
}

#[tauri::command]
pub fn copy_files_to_worktree(
    source_path: String,
    target_path: String,
    patterns: Vec<String>,
) -> Result<CopyResult, String> {
    super::git_worktree::copy_files_to_worktree(source_path, target_path, patterns)
}

#[tauri::command]
pub fn detect_package_manager(project_path: String) -> Result<Option<PackageManager>, String> {
    super::git_worktree::detect_package_manager(project_path)
}

#[tauri::command]
pub fn run_init_command(cwd: String, command: String) -> Result<CommandOutput, String> {
    super::git_worktree::run_init_command(cwd, command)
}
