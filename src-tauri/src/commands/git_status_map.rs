// This file contains git status mapping logic that requires complex git operations
// (rename, conflict) to fully test. Covered via E2E tests.

use super::git::GitFileStatus;
use git2::Status;

/// Map git2 Status to GitFileStatus
/// Returns None for unchanged files that should be skipped
pub fn map_status(status: Status) -> Option<GitFileStatus> {
    if status.is_index_new() {
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
    } else if status.is_ignored() {
        Some(GitFileStatus::Ignored)
    } else {
        None // Skip unchanged files
    }
}

/// Map status for single file lookup (get_git_file_status)
pub fn map_file_status(status: Status) -> Option<GitFileStatus> {
    if status.is_index_new() {
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
    }
}
