use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventKind};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct FsChangeEvent {
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitChangeEvent {
    pub repo_root: String,
}

struct WatcherInstance {
    #[allow(dead_code)]
    debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    root_path: PathBuf,
}

pub struct WatcherManager {
    instances: HashMap<String, WatcherInstance>,
}

impl WatcherManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
        }
    }
}

impl Default for WatcherManager {
    fn default() -> Self {
        Self::new()
    }
}

pub type WatcherState = Arc<Mutex<WatcherManager>>;

#[tauri::command]
pub fn start_watching(
    app: AppHandle,
    state: tauri::State<'_, WatcherState>,
    path: String,
) -> Result<(), String> {
    let root_path = PathBuf::from(&path);

    if !root_path.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    let mut manager = state.lock().map_err(|e| e.to_string())?;

    // Already watching this path
    if manager.instances.contains_key(&path) {
        return Ok(());
    }

    let app_handle = app.clone();
    let watched_path = path.clone();

    // Create debounced watcher with 300ms delay
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        move |result: DebounceEventResult| {
            if let Ok(events) = result {
                let mut fs_changed = false;
                let mut git_changed = false;

                for event in events {
                    let path_str = event.path.to_string_lossy().to_string();

                    // Check if this is a git-related change
                    if path_str.contains("/.git/") || path_str.ends_with("/.git") {
                        // Only trigger git change on specific events
                        if matches!(event.kind, DebouncedEventKind::Any) {
                            git_changed = true;
                        }
                    } else {
                        fs_changed = true;
                    }
                }

                // Emit consolidated events
                if fs_changed {
                    let _ = app_handle.emit(
                        "fs-changed",
                        FsChangeEvent {
                            path: watched_path.clone(),
                        },
                    );
                }

                if git_changed {
                    let _ = app_handle.emit(
                        "git-status-changed",
                        GitChangeEvent {
                            repo_root: watched_path.clone(),
                        },
                    );
                }
            }
        },
    )
    .map_err(|e| e.to_string())?;

    // Start watching the directory recursively
    debouncer
        .watcher()
        .watch(&root_path, RecursiveMode::Recursive)
        .map_err(|e| e.to_string())?;

    manager.instances.insert(
        path,
        WatcherInstance {
            debouncer,
            root_path,
        },
    );

    Ok(())
}

#[tauri::command]
pub fn stop_watching(state: tauri::State<'_, WatcherState>, path: String) -> Result<(), String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;

    if manager.instances.remove(&path).is_some() {
        log::info!("Stopped watching: {}", path);
    }

    Ok(())
}

#[tauri::command]
pub fn stop_all_watching(state: tauri::State<'_, WatcherState>) -> Result<(), String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;

    let count = manager.instances.len();
    manager.instances.clear();
    log::info!("Stopped all watchers ({})", count);

    Ok(())
}
