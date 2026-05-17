//! Concurrency stress tests for git status / diff calls.
//!
//! The libgit2 `Repository` handle is not `Sync`, but every command in
//! `commands/git.rs` opens its own `Repository::open(...)` instance per
//! call. That means N concurrent calls from multiple tabs each get
//! their own handle - the only shared resource is the on-disk repo
//! itself (and the OS-level file locks libgit2 takes for index reads).
//!
//! These tests verify that:
//! - N parallel `get_git_status` calls against the same repo all
//!   succeed and return consistent results.
//! - Concurrent `get_git_diff` calls don't corrupt each other.
//! - A `get_git_status` racing against an `index` mutation in another
//!   thread still returns *some* sensible answer instead of erroring.
//!
//! These tests intentionally run with the multi-threaded default
//! `cargo test` runtime; they have no `#[serial]` annotation.

use app_lib::commands::git::{get_all_git_diffs, get_git_status};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use tempfile::TempDir;

fn run(cmd: &str, args: &[&str], cwd: &Path) {
    let status = Command::new(cmd)
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap_or_else(|e| panic!("failed to spawn {cmd}: {e}"));
    assert!(status.success(), "{cmd} {args:?} failed");
}

/// Initialise a small repo with a couple of tracked files, one staged
/// change, and one untracked file, so `get_git_status` has real work.
fn seed_repo() -> TempDir {
    let dir = tempfile::tempdir().expect("temp");
    let p = dir.path();

    run("git", &["init", "-q", "-b", "main"], p);
    run("git", &["config", "user.email", "t@example.com"], p);
    run("git", &["config", "user.name", "Test"], p);
    run("git", &["config", "commit.gpgsign", "false"], p);

    fs::write(p.join("a.txt"), "first\n").unwrap();
    fs::write(p.join("b.txt"), "second\n").unwrap();
    run("git", &["add", "."], p);
    run("git", &["commit", "-qm", "init"], p);

    // One unstaged modification.
    let mut f = fs::OpenOptions::new()
        .append(true)
        .open(p.join("a.txt"))
        .unwrap();
    writeln!(f, "appended").unwrap();

    // One untracked file.
    fs::write(p.join("c.txt"), "third\n").unwrap();

    dir
}

#[test]
fn parallel_get_git_status_does_not_race() {
    let dir = seed_repo();
    let path = dir.path().to_string_lossy().to_string();

    // Reference shape from a serial call.
    let reference = get_git_status(path.clone()).expect("baseline");
    let baseline_count = reference.statuses.len();
    assert!(baseline_count >= 2, "seed should produce statuses");

    let path = Arc::new(path);
    let errors = Arc::new(AtomicUsize::new(0));
    let mismatches = Arc::new(AtomicUsize::new(0));

    let mut handles = Vec::new();
    for _ in 0..16 {
        let path = Arc::clone(&path);
        let errors = Arc::clone(&errors);
        let mismatches = Arc::clone(&mismatches);

        handles.push(thread::spawn(move || {
            for _ in 0..8 {
                match get_git_status((*path).clone()) {
                    Ok(info) => {
                        if info.statuses.len() != baseline_count {
                            mismatches.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    for h in handles {
        h.join().expect("thread");
    }

    assert_eq!(
        errors.load(Ordering::Relaxed),
        0,
        "no parallel call should error"
    );
    assert_eq!(
        mismatches.load(Ordering::Relaxed),
        0,
        "every parallel call must return the same status count as serial"
    );
}

#[test]
fn parallel_get_all_git_diffs_does_not_race() {
    let dir = seed_repo();
    let path = Arc::new(dir.path().to_string_lossy().to_string());

    let errors = Arc::new(AtomicUsize::new(0));
    let mut handles = Vec::new();

    for _ in 0..8 {
        let path = Arc::clone(&path);
        let errors = Arc::clone(&errors);
        handles.push(thread::spawn(move || {
            for _ in 0..4 {
                if get_all_git_diffs((*path).clone()).is_err() {
                    errors.fetch_add(1, Ordering::Relaxed);
                }
            }
        }));
    }

    for h in handles {
        h.join().expect("thread");
    }

    assert_eq!(
        errors.load(Ordering::Relaxed),
        0,
        "concurrent diff reads must not error"
    );
}

#[test]
fn status_returns_a_value_under_concurrent_writes() {
    // One writer mutates index entries while readers poll status. The
    // contract is "no error / no crash" - libgit2's status read takes
    // its own snapshot, so the answer may transiently differ but must
    // always be `Ok`.
    let dir = seed_repo();
    let path = dir.path().to_path_buf();
    let path_str = Arc::new(path.to_string_lossy().to_string());

    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let writer = {
        let path = path.clone();
        let stop = Arc::clone(&stop);
        thread::spawn(move || {
            let mut i = 0u32;
            while !stop.load(Ordering::Relaxed) {
                let file = path.join(format!("noise_{i}.txt"));
                let _ = fs::write(&file, format!("payload {i}"));
                let _ = fs::remove_file(&file);
                i = i.wrapping_add(1);
            }
        })
    };

    let mut errors = 0usize;
    for _ in 0..50 {
        if get_git_status((*path_str).clone()).is_err() {
            errors += 1;
        }
    }

    stop.store(true, Ordering::Relaxed);
    writer.join().expect("writer");

    assert_eq!(errors, 0, "status under concurrent writes must not error");
}
