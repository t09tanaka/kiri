use super::git_history::{
    BehindAheadCount, CommitDiffResult, CommitInfo, FetchResult, PullResult, PushResult,
};

#[tauri::command]
pub fn get_commit_log(
    repo_path: String,
    max_count: Option<usize>,
    skip: Option<usize>,
) -> Result<Vec<CommitInfo>, String> {
    super::git_history::get_commit_log(repo_path, max_count, skip)
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

#[tauri::command]
pub fn fetch_remote(
    repo_path: String,
    remote: Option<String>,
) -> Result<FetchResult, String> {
    super::git_history::fetch_remote(repo_path, remote)
}

#[tauri::command]
pub fn get_behind_ahead_count(repo_path: String) -> Result<BehindAheadCount, String> {
    super::git_history::get_behind_ahead_count(repo_path)
}

#[tauri::command]
pub fn get_branch_ahead_count(repo_path: String) -> Result<usize, String> {
    super::git_history::get_branch_ahead_count(repo_path)
}

#[tauri::command]
pub fn pull_commits(
    repo_path: String,
    remote: Option<String>,
    branch: Option<String>,
) -> Result<PullResult, String> {
    super::git_history::pull_commits(repo_path, remote, branch)
}
