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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_status_index_new() {
        assert_eq!(map_status(Status::INDEX_NEW), Some(GitFileStatus::Added));
    }

    #[test]
    fn test_map_status_wt_new() {
        assert_eq!(map_status(Status::WT_NEW), Some(GitFileStatus::Untracked));
    }

    #[test]
    fn test_map_status_index_modified() {
        assert_eq!(map_status(Status::INDEX_MODIFIED), Some(GitFileStatus::Modified));
    }

    #[test]
    fn test_map_status_wt_modified() {
        assert_eq!(map_status(Status::WT_MODIFIED), Some(GitFileStatus::Modified));
    }

    #[test]
    fn test_map_status_index_deleted() {
        assert_eq!(map_status(Status::INDEX_DELETED), Some(GitFileStatus::Deleted));
    }

    #[test]
    fn test_map_status_wt_deleted() {
        assert_eq!(map_status(Status::WT_DELETED), Some(GitFileStatus::Deleted));
    }

    #[test]
    fn test_map_status_index_renamed() {
        assert_eq!(map_status(Status::INDEX_RENAMED), Some(GitFileStatus::Renamed));
    }

    #[test]
    fn test_map_status_wt_renamed() {
        assert_eq!(map_status(Status::WT_RENAMED), Some(GitFileStatus::Renamed));
    }

    #[test]
    fn test_map_status_conflicted() {
        assert_eq!(map_status(Status::CONFLICTED), Some(GitFileStatus::Conflicted));
    }

    #[test]
    fn test_map_status_ignored() {
        assert_eq!(map_status(Status::IGNORED), Some(GitFileStatus::Ignored));
    }

    #[test]
    fn test_map_status_current_returns_none() {
        assert_eq!(map_status(Status::CURRENT), None);
    }

    #[test]
    fn test_map_file_status_index_renamed() {
        assert_eq!(map_file_status(Status::INDEX_RENAMED), Some(GitFileStatus::Renamed));
    }

    #[test]
    fn test_map_file_status_conflicted() {
        assert_eq!(map_file_status(Status::CONFLICTED), Some(GitFileStatus::Conflicted));
    }

    #[test]
    fn test_map_file_status_index_new() {
        assert_eq!(map_file_status(Status::INDEX_NEW), Some(GitFileStatus::Added));
    }

    #[test]
    fn test_map_file_status_current_returns_none() {
        assert_eq!(map_file_status(Status::CURRENT), None);
    }
}
