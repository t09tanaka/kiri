use super::git_history::{CommitDiffResult, CommitInfo, PushResult};

#[tauri::command]
pub fn get_commit_log(
    repo_path: String,
    max_count: Option<usize>,
) -> Result<Vec<CommitInfo>, String> {
    super::git_history::get_commit_log(repo_path, max_count)
}

#[tauri::command]
pub fn get_commit_diff(
    repo_path: String,
    commit_hash: String,
) -> Result<CommitDiffResult, String> {
    super::git_history::get_commit_diff(repo_path, commit_hash)
}

#[tauri::command]
pub fn push_commits(
    repo_path: String,
    remote: Option<String>,
    branch: Option<String>,
) -> Result<PushResult, String> {
    super::git_history::push_commits(repo_path, remote, branch)
}
