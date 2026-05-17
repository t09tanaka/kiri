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

/// Burst of file mutations must produce a bounded event stream
/// (debouncer collapses them) and every path that gets an event must
/// be one of the files we touched (or its parent dir) - the debouncer
/// must not invent paths.
#[test]
fn test_watcher_dedups_burst_of_mutations() {
    const N: usize = 100;
    let dir = tempdir().expect("Failed to create temp dir");

    let (tx, rx) = channel();

    let mut debouncer = new_debouncer(Duration::from_millis(75), move |res| {
        let _ = tx.send(res);
    })
    .expect("Failed to create debouncer");

    debouncer
        .watcher()
        .watch(dir.path(), RecursiveMode::Recursive)
        .expect("Failed to watch directory");

    // macOS reports paths via /private/var/... after canonicalisation,
    // so compare canonicalised paths on both sides.
    let canon_root = dir.path().canonicalize().expect("canonicalize root");
    let mut expected: std::collections::HashSet<std::path::PathBuf> =
        std::collections::HashSet::with_capacity(N);
    for i in 0..N {
        expected.insert(canon_root.join(format!("burst_{i}.txt")));
    }

    // Burst write, then a second mutation pass over each file. The
    // 75ms debounce window must collapse most of this.
    for i in 0..N {
        let p = dir.path().join(format!("burst_{i}.txt"));
        let mut f = File::create(&p).expect("create");
        writeln!(f, "v1").expect("write v1");
    }
    for i in 0..N {
        let p = dir.path().join(format!("burst_{i}.txt"));
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&p)
            .expect("reopen");
        writeln!(f, "v2").expect("write v2");
    }

    let mut batches = 0usize;
    let mut total_events = 0usize;
    let deadline = std::time::Instant::now() + Duration::from_millis(1500);
    while std::time::Instant::now() < deadline {
        match rx.recv_timeout(Duration::from_millis(200)) {
            Ok(Ok(events)) => {
                batches += 1;
                total_events += events.len();
                for ev in &events {
                    let is_known = ev.path == canon_root
                        || expected.contains(&ev.path)
                        || Some(canon_root.as_path()) == ev.path.parent();
                    assert!(is_known, "watcher surfaced unexpected path: {:?}", ev.path);
                }
            }
            Ok(Err(e)) => panic!("watcher error: {e:?}"),
            Err(_) => continue,
        }
    }

    assert!(batches >= 1, "expected at least one debounced batch");
    assert!(
        total_events <= N * 4,
        "debouncer failed to dedup: got {total_events} events for {N} files"
    );
}
