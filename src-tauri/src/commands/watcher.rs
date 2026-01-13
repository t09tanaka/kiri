use notify_debouncer_mini::DebouncedEventKind;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct FsChangeEvent {
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitChangeEvent {
    pub repo_root: String,
}

/// Result of classifying a file path for event handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathClassification {
    /// The path is inside .git directory
    GitPath,
    /// The path is a regular file system path
    FsPath,
}

/// Classify a path to determine if it's a git-related path or a regular fs path
pub fn classify_path(path: &str) -> PathClassification {
    if path.contains("/.git/") || path.ends_with("/.git") {
        PathClassification::GitPath
    } else {
        PathClassification::FsPath
    }
}

/// Result of processing debounced events
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventClassificationResult {
    pub fs_changed: bool,
    pub git_changed: bool,
}

/// Process a list of debounced events and classify them
pub fn classify_events<'a, I>(events: I) -> EventClassificationResult
where
    I: IntoIterator<Item = &'a notify_debouncer_mini::DebouncedEvent>,
{
    let mut result = EventClassificationResult::default();

    for event in events {
        let path_str = event.path.to_string_lossy();

        match classify_path(&path_str) {
            PathClassification::GitPath => {
                // Only trigger git change on specific events
                if matches!(event.kind, DebouncedEventKind::Any) {
                    result.git_changed = true;
                }
            }
            PathClassification::FsPath => {
                result.fs_changed = true;
            }
        }
    }

    result
}

/// Check if a path exists
pub fn path_exists(path: &str) -> bool {
    PathBuf::from(path).exists()
}

/// Default debounce duration in milliseconds
pub const DEFAULT_DEBOUNCE_MS: u64 = 300;

pub struct WatcherInstance {
    #[allow(dead_code)]
    pub debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    #[allow(dead_code)]
    pub root_path: PathBuf,
}

pub struct WatcherManager {
    pub instances: HashMap<String, WatcherInstance>,
}

impl WatcherManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }

    /// Check if a path is being watched
    pub fn is_watching(&self, path: &str) -> bool {
        self.instances.contains_key(path)
    }
}

impl Default for WatcherManager {
    fn default() -> Self {
        Self::new()
    }
}

pub type WatcherState = Arc<Mutex<WatcherManager>>;

#[cfg(test)]
mod tests {
    use super::*;
    use notify_debouncer_mini::DebouncedEvent;

    #[test]
    fn test_watcher_manager_new() {
        let manager = WatcherManager::new();
        assert!(manager.instances.is_empty());
    }

    #[test]
    fn test_watcher_manager_default() {
        let manager = WatcherManager::default();
        assert!(manager.instances.is_empty());
    }

    #[test]
    fn test_watcher_manager_is_watching() {
        let manager = WatcherManager::new();
        assert!(!manager.is_watching("/some/path"));
    }

    #[test]
    fn test_fs_change_event_struct() {
        let event = FsChangeEvent {
            path: "/path/to/file".to_string(),
        };
        assert_eq!(event.path, "/path/to/file");
    }

    #[test]
    fn test_fs_change_event_clone() {
        let event = FsChangeEvent {
            path: "/path/to/file".to_string(),
        };
        let cloned = event.clone();
        assert_eq!(cloned.path, event.path);
    }

    #[test]
    fn test_git_change_event_struct() {
        let event = GitChangeEvent {
            repo_root: "/path/to/repo".to_string(),
        };
        assert_eq!(event.repo_root, "/path/to/repo");
    }

    #[test]
    fn test_git_change_event_clone() {
        let event = GitChangeEvent {
            repo_root: "/path/to/repo".to_string(),
        };
        let cloned = event.clone();
        assert_eq!(cloned.repo_root, event.repo_root);
    }

    #[test]
    fn test_classify_path_git_internal() {
        assert_eq!(
            classify_path("/repo/.git/objects/abc"),
            PathClassification::GitPath
        );
        assert_eq!(
            classify_path("/repo/.git/HEAD"),
            PathClassification::GitPath
        );
        assert_eq!(
            classify_path("/repo/.git/index"),
            PathClassification::GitPath
        );
    }

    #[test]
    fn test_classify_path_git_root() {
        assert_eq!(classify_path("/repo/.git"), PathClassification::GitPath);
    }

    #[test]
    fn test_classify_path_regular_file() {
        assert_eq!(
            classify_path("/repo/src/main.rs"),
            PathClassification::FsPath
        );
        assert_eq!(
            classify_path("/repo/README.md"),
            PathClassification::FsPath
        );
        assert_eq!(classify_path("/home/user/file.txt"), PathClassification::FsPath);
    }

    #[test]
    fn test_classify_path_gitignore() {
        // .gitignore is NOT inside .git directory, so it's a regular file
        assert_eq!(classify_path("/repo/.gitignore"), PathClassification::FsPath);
    }

    #[test]
    fn test_classify_events_empty() {
        let events: Vec<DebouncedEvent> = vec![];
        let result = classify_events(events.iter());
        assert!(!result.fs_changed);
        assert!(!result.git_changed);
    }

    #[test]
    fn test_classify_events_fs_only() {
        let events = vec![
            DebouncedEvent {
                path: PathBuf::from("/repo/src/main.rs"),
                kind: DebouncedEventKind::Any,
            },
            DebouncedEvent {
                path: PathBuf::from("/repo/README.md"),
                kind: DebouncedEventKind::Any,
            },
        ];
        let result = classify_events(events.iter());
        assert!(result.fs_changed);
        assert!(!result.git_changed);
    }

    #[test]
    fn test_classify_events_git_only() {
        let events = vec![
            DebouncedEvent {
                path: PathBuf::from("/repo/.git/index"),
                kind: DebouncedEventKind::Any,
            },
            DebouncedEvent {
                path: PathBuf::from("/repo/.git/HEAD"),
                kind: DebouncedEventKind::Any,
            },
        ];
        let result = classify_events(events.iter());
        assert!(!result.fs_changed);
        assert!(result.git_changed);
    }

    #[test]
    fn test_classify_events_mixed() {
        let events = vec![
            DebouncedEvent {
                path: PathBuf::from("/repo/src/main.rs"),
                kind: DebouncedEventKind::Any,
            },
            DebouncedEvent {
                path: PathBuf::from("/repo/.git/index"),
                kind: DebouncedEventKind::Any,
            },
        ];
        let result = classify_events(events.iter());
        assert!(result.fs_changed);
        assert!(result.git_changed);
    }

    #[test]
    fn test_classify_events_git_continuous_event() {
        // AnyContinuous events should not trigger git_changed
        let events = vec![DebouncedEvent {
            path: PathBuf::from("/repo/.git/index"),
            kind: DebouncedEventKind::AnyContinuous,
        }];
        let result = classify_events(events.iter());
        assert!(!result.fs_changed);
        assert!(!result.git_changed);
    }

    #[test]
    fn test_path_exists_valid() {
        assert!(path_exists("/tmp"));
        assert!(path_exists("/"));
    }

    #[test]
    fn test_path_exists_invalid() {
        assert!(!path_exists("/nonexistent/path/that/does/not/exist"));
    }

    #[test]
    fn test_default_debounce_ms() {
        assert_eq!(DEFAULT_DEBOUNCE_MS, 300);
    }

    #[test]
    fn test_event_classification_result_default() {
        let result = EventClassificationResult::default();
        assert!(!result.fs_changed);
        assert!(!result.git_changed);
    }
}
