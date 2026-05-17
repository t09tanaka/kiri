//! Integration tests for file watcher functionality
//!
//! These tests verify that the file watching operations work correctly
//! without the Tauri command wrapper.

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventKind};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;
use tempfile::tempdir;

/// Wait for the watcher to actually register the directory by writing
/// a sentinel file and blocking until its event arrives (or until
/// `deadline`). Drains any remaining events afterwards so the test's
/// own mutation starts from a clean queue.
///
/// Replaces the old `thread::sleep(100ms)` pattern, which was racy on
/// slow CI runners where the FSEvents kqueue subscription took longer
/// than 100ms to attach. The event-based wait still has a safety
/// timeout so a missing watcher fails fast instead of hanging.
fn wait_for_watcher_ready(rx: &Receiver<DebounceEventResult>, watched: &Path) {
    let sentinel = watched.join(".watcher-warmup");
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    let mut seen_warmup = false;

    while std::time::Instant::now() < deadline {
        // Touch the sentinel (idempotent if it already exists).
        let _ = File::create(&sentinel);
        match rx.recv_timeout(Duration::from_millis(150)) {
            Ok(Ok(events)) => {
                for ev in events {
                    if ev.path == sentinel || ev.path.file_name() == sentinel.file_name() {
                        seen_warmup = true;
                    }
                }
                if seen_warmup {
                    break;
                }
            }
            Ok(Err(_)) | Err(_) => continue,
        }
    }
    assert!(seen_warmup, "watcher never surfaced the warmup sentinel");

    // Clean up sentinel and drain anything still queued.
    let _ = fs::remove_file(&sentinel);
    while rx.recv_timeout(Duration::from_millis(150)).is_ok() {}
}

#[test]
fn test_watcher_creation() {
    let (tx, _rx) = channel();

    let result = new_debouncer(Duration::from_millis(100), move |res| {
        let _ = tx.send(res);
    });

    assert!(result.is_ok());
}

#[test]
fn test_watcher_watch_directory() {
    let dir = tempdir().expect("Failed to create temp dir");

    let (tx, _rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_millis(100), move |res| {
        let _ = tx.send(res);
    })
    .expect("Failed to create debouncer");

    let result = debouncer.watcher().watch(dir.path(), RecursiveMode::Recursive);

    assert!(result.is_ok());
}

#[test]
fn test_watcher_detects_file_creation() {
    let dir = tempdir().expect("Failed to create temp dir");

    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_millis(50), move |res| {
        let _ = tx.send(res);
    })
    .expect("Failed to create debouncer");

    debouncer
        .watcher()
        .watch(dir.path(), RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    // Create a new file
    let file_path = dir.path().join("test_file.txt");
    File::create(&file_path).expect("Failed to create file");

    // Wait for event with timeout
    let result = rx.recv_timeout(Duration::from_secs(2));

    assert!(result.is_ok(), "Should receive file creation event");

    let events = result.unwrap();
    assert!(events.is_ok(), "Events should be Ok");

    let events = events.unwrap();
    assert!(!events.is_empty(), "Should have at least one event");
}

#[test]
fn test_watcher_detects_file_modification() {
    let dir = tempdir().expect("Failed to create temp dir");

    // Create file before watching
    let file_path = dir.path().join("existing_file.txt");
    {
        let mut file = File::create(&file_path).expect("Failed to create file");
        file.write_all(b"initial content")
            .expect("Failed to write");
    }

    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_millis(50), move |res| {
        let _ = tx.send(res);
    })
    .expect("Failed to create debouncer");

    debouncer
        .watcher()
        .watch(dir.path(), RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    wait_for_watcher_ready(&rx, dir.path());

    // Modify the file
    {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&file_path)
            .expect("Failed to open file");
        file.write_all(b"modified content")
            .expect("Failed to write");
    }

    // Wait for event with timeout
    let result = rx.recv_timeout(Duration::from_secs(2));

    assert!(result.is_ok(), "Should receive file modification event");
}

#[test]
fn test_watcher_detects_file_deletion() {
    let dir = tempdir().expect("Failed to create temp dir");

    // Create file before watching
    let file_path = dir.path().join("to_delete.txt");
    File::create(&file_path).expect("Failed to create file");

    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_millis(50), move |res| {
        let _ = tx.send(res);
    })
    .expect("Failed to create debouncer");

    debouncer
        .watcher()
        .watch(dir.path(), RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    wait_for_watcher_ready(&rx, dir.path());

    // Delete the file
    fs::remove_file(&file_path).expect("Failed to delete file");

    // Wait for event with timeout
    let result = rx.recv_timeout(Duration::from_secs(2));

    assert!(result.is_ok(), "Should receive file deletion event");
}

#[test]
fn test_watcher_recursive_mode() {
    let dir = tempdir().expect("Failed to create temp dir");

    // Create subdirectory
    let sub_dir = dir.path().join("subdir");
    fs::create_dir(&sub_dir).expect("Failed to create subdir");

    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_millis(50), move |res| {
        let _ = tx.send(res);
    })
    .expect("Failed to create debouncer");

    debouncer
        .watcher()
        .watch(dir.path(), RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    wait_for_watcher_ready(&rx, dir.path());

    // Create file in subdirectory
    let file_path = sub_dir.join("nested_file.txt");
    File::create(&file_path).expect("Failed to create file");

    // Wait for event with timeout
    let result = rx.recv_timeout(Duration::from_secs(2));

    assert!(
        result.is_ok(),
        "Should receive event for file in subdirectory"
    );
}

#[test]
fn test_watcher_event_kind() {
    // Test that DebouncedEventKind::Any is the expected kind
    let kind = DebouncedEventKind::Any;
    assert!(matches!(kind, DebouncedEventKind::Any));

    let kind2 = DebouncedEventKind::AnyContinuous;
    assert!(matches!(kind2, DebouncedEventKind::AnyContinuous));
}

#[test]
fn test_watcher_multiple_files() {
    let dir = tempdir().expect("Failed to create temp dir");

    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_millis(50), move |res| {
        let _ = tx.send(res);
    })
    .expect("Failed to create debouncer");

    debouncer
        .watcher()
        .watch(dir.path(), RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    wait_for_watcher_ready(&rx, dir.path());

    // Create multiple files
    for i in 0..3 {
        let file_path = dir.path().join(format!("file_{}.txt", i));
        File::create(&file_path).expect("Failed to create file");
    }

    // The debouncer batches events; the first batch must contain at
    // least one of the files we created. Drain a few batches up to a
    // 2s deadline and assert we see a known path.
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    let mut saw_known = false;
    while std::time::Instant::now() < deadline && !saw_known {
        if let Ok(Ok(events)) = rx.recv_timeout(Duration::from_millis(200)) {
            for ev in events {
                if let Some(name) = ev.path.file_name().and_then(|n| n.to_str()) {
                    if name.starts_with("file_") {
                        saw_known = true;
                        break;
                    }
                }
            }
        }
    }
    assert!(saw_known, "Should receive events for the created files");
}
