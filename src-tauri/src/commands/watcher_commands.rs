//! Tauri command wrappers for file watcher functionality
//! These are thin wrappers that delegate to the core logic in watcher.rs

use super::watcher::{
    classify_events, path_exists, FsChangeEvent, FsFileChangeEvent, GitChangeEvent,
    WatcherInstance, WatcherState, DEFAULT_DEBOUNCE_MS,
};
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use std::path::PathBuf;
use std::time::Duration;
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub fn start_watching(
    app: AppHandle,
    state: tauri::State<'_, WatcherState>,
    path: String,
) -> Result<(), String> {
    let root_path = PathBuf::from(&path);

    if !path_exists(&path) {
        return Err(format!("Path does not exist: {}", path));
    }

    let mut manager = state.lock().map_err(|e| e.to_string())?;

    // Already watching this path
    if manager.is_watching(&path) {
        return Ok(());
    }

    let app_handle = app.clone();
    let watched_path = path.clone();

    // Create debounced watcher with default delay
    let mut debouncer = new_debouncer(
        Duration::from_millis(DEFAULT_DEBOUNCE_MS),
        move |result: DebounceEventResult| {
            if let Ok(events) = result {
                let classification = classify_events(events.iter());

                // Emit consolidated events
                if classification.fs_changed {
                    let _ = app_handle.emit(
                        "fs-changed",
                        FsChangeEvent {
                            path: watched_path.clone(),
                        },
                    );
                }

                // Emit file-level change event for editor auto-reload
                if !classification.changed_files.is_empty() {
                    let _ = app_handle.emit(
                        "fs-file-changed",
                        FsFileChangeEvent {
                            paths: classification.changed_files.clone(),
                        },
                    );
                }

                if classification.git_changed {
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
