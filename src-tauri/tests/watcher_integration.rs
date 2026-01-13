//! Integration tests for file watcher functionality
//!
//! These tests verify that the file watching operations work correctly
//! without the Tauri command wrapper.

use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebouncedEventKind};
use std::fs::{self, File};
use std::io::Write;
use std::sync::mpsc::channel;
use std::time::Duration;
use tempfile::tempdir;

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

    // Modify the file
    std::thread::sleep(Duration::from_millis(100));
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

    // Delete the file
    std::thread::sleep(Duration::from_millis(100));
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

    // Create file in subdirectory
    std::thread::sleep(Duration::from_millis(100));
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

    // Create multiple files
    for i in 0..3 {
        let file_path = dir.path().join(format!("file_{}.txt", i));
        File::create(&file_path).expect("Failed to create file");
    }

    // Wait for events - debouncer should batch them
    std::thread::sleep(Duration::from_millis(200));

    // Should receive at least one event batch
    let result = rx.recv_timeout(Duration::from_secs(2));
    assert!(result.is_ok(), "Should receive events for multiple files");
}
