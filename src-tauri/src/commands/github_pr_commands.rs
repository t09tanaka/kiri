use super::github_pr::{GhCliStatus, PullRequest};

#[tauri::command]
pub fn check_gh_cli() -> GhCliStatus {
    super::github_pr::check_gh_cli()
}

#[tauri::command]
pub fn list_pull_requests(repo_path: String) -> Result<Vec<PullRequest>, String> {
    super::github_pr::list_pull_requests(repo_path)
}

#[tauri::command]
pub fn get_pull_request_detail(repo_path: String, number: u32) -> Result<PullRequest, String> {
    super::github_pr::get_pull_request_detail(repo_path, number)
}
