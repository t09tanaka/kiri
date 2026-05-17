//! Integration tests for symlink handling in `commands::fs`.
//!
//! `edge_cases.rs` already covers Unicode / long paths / 0-byte / EACCES /
//! mixed line endings / backslash filenames. The `#[cfg(test)] mod tests`
//! block inside `fs.rs` covers basic create / delete / read paths.
//!
//! Symlink behavior was the remaining notable gap: `read_directory` reports
//! entry types from `DirEntry::file_type()` (which does NOT follow
//! symlinks), while `delete_path` decides between `remove_dir_all` and
//! `remove_file` based on `Path::is_dir()` (which DOES follow symlinks).
//! That asymmetry is easy to regress and worth pinning down.
//!
//! All tests are `#[cfg(unix)]` because `std::os::unix::fs::symlink` is
//! Unix-only; the macOS/Linux CI runs both cover this matrix. Windows
//! symlink semantics are a separate concern (requires SeCreateSymbolicLink
//! privilege and admin) and out of scope here.

#![cfg(unix)]

use app_lib::commands::fs::{create_directory, delete_path, read_directory};
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::PathBuf;
use tempfile::TempDir;

fn temp() -> TempDir {
    tempfile::tempdir().expect("temp dir")
}

fn write_file(path: &PathBuf, body: &[u8]) {
    let mut f = File::create(path).expect("create file");
    f.write_all(body).expect("write body");
}

// --- read_directory: symlink classification ---------------------------------

#[tokio::test]
async fn read_directory_reports_symlink_to_file_as_non_dir() {
    let dir = temp();
    let target = dir.path().join("target.txt");
    write_file(&target, b"hello");

    let link = dir.path().join("link.txt");
    symlink(&target, &link).expect("symlink file");

    let mut entries = read_directory(dir.path().to_string_lossy().to_string())
        .await
        .expect("read_directory");
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    assert_eq!(entries.len(), 2);
    let link_entry = entries
        .iter()
        .find(|e| e.name == "link.txt")
        .expect("link.txt present");
    assert!(
        !link_entry.is_dir,
        "symlink to file should be classified as non-dir"
    );
}

#[tokio::test]
async fn read_directory_reports_symlink_to_directory_as_non_dir() {
    // DirEntry::file_type() does not follow symlinks, so a symlink that
    // points to a directory is still reported with `is_dir: false`. This
    // is deliberate: the file tree must not silently recurse into a
    // symlinked directory (which could escape the project root or loop).
    let dir = temp();
    let target_dir = dir.path().join("real-dir");
    fs::create_dir(&target_dir).expect("create real-dir");
    write_file(&target_dir.join("inside.txt"), b"x");

    let link_dir = dir.path().join("link-dir");
    symlink(&target_dir, &link_dir).expect("symlink dir");

    let entries = read_directory(dir.path().to_string_lossy().to_string())
        .await
        .expect("read_directory");

    let link_entry = entries
        .iter()
        .find(|e| e.name == "link-dir")
        .expect("link-dir present in listing");
    assert!(
        !link_entry.is_dir,
        "symlink to directory must be reported as non-dir to prevent silent recursion"
    );

    let real_entry = entries
        .iter()
        .find(|e| e.name == "real-dir")
        .expect("real-dir present");
    assert!(real_entry.is_dir);
}

#[tokio::test]
async fn read_directory_includes_broken_symlinks_in_listing() {
    // A broken symlink (target does not exist) must still surface in the
    // listing so the user can see / delete it from the file tree.
    let dir = temp();
    let dangling = dir.path().join("dangling");
    symlink(dir.path().join("does-not-exist"), &dangling).expect("dangling symlink");

    let entries = read_directory(dir.path().to_string_lossy().to_string())
        .await
        .expect("read_directory");

    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(
        names.contains(&"dangling"),
        "broken symlink should be enumerated, got {:?}",
        names
    );
}

// --- delete_path: symlink semantics -----------------------------------------

#[test]
fn delete_path_on_symlink_to_file_removes_only_the_link() {
    let dir = temp();
    let target = dir.path().join("target.txt");
    write_file(&target, b"keep me");

    let link = dir.path().join("link.txt");
    symlink(&target, &link).expect("symlink");

    delete_path(link.to_string_lossy().to_string()).expect("delete symlink");

    // Symlink path itself uses symlink_metadata to check existence (it
    // does not follow the link), so the link must be gone…
    let link_meta = fs::symlink_metadata(&link);
    assert!(
        link_meta.is_err(),
        "symlink should be removed, but symlink_metadata returned Ok"
    );
    // …while the target file remains untouched.
    assert!(
        target.exists(),
        "delete_path on a symlink-to-file must not touch the target file"
    );
    assert_eq!(fs::read(&target).expect("read target"), b"keep me");
}

#[test]
fn delete_path_on_broken_symlink_does_not_silently_succeed() {
    // delete_path uses `Path::exists()`, which FOLLOWS symlinks. A broken
    // symlink reports `exists() == false` even though the link entry is
    // physically present. This test pins down current behavior: delete_path
    // returns an error and leaves the dangling link in place. If we ever
    // teach the command to clean up broken symlinks instead, this test
    // becomes the place to flip the assertion intentionally.
    let dir = temp();
    let dangling = dir.path().join("dangling");
    symlink(dir.path().join("nonexistent-target"), &dangling).expect("dangling");

    let result = delete_path(dangling.to_string_lossy().to_string());
    assert!(
        result.is_err(),
        "current implementation cannot clean up broken symlinks; saw Ok"
    );
    // The link entry itself is still present (delete did nothing).
    assert!(
        fs::symlink_metadata(&dangling).is_ok(),
        "dangling symlink should still be on disk after the failed delete"
    );
}

#[test]
fn delete_path_on_symlink_to_directory_does_not_recurse_into_target() {
    // delete_path uses `path.is_dir()`, which follows symlinks. The
    // important regression guard is: even if we route into the dir
    // branch, std::fs::remove_dir_all on macOS / Linux must NOT recurse
    // into the target directory and wipe its contents. We assert the
    // contents survive; whether the symlink itself goes away is allowed
    // to vary across Rust versions / kernels (1.83+ refuses; older
    // versions could remove the symlink).
    let dir = temp();
    let target_dir = dir.path().join("real-dir");
    fs::create_dir(&target_dir).expect("create real-dir");
    write_file(&target_dir.join("inside.txt"), b"keep me");

    let link_dir = dir.path().join("link-dir");
    symlink(&target_dir, &link_dir).expect("symlink dir");

    // Either succeeds (symlink removed) or errs (refused) — both are
    // acceptable behaviors; what's NOT acceptable is wiping the target.
    let _ = delete_path(link_dir.to_string_lossy().to_string());

    assert!(
        target_dir.exists() && target_dir.is_dir(),
        "delete on symlink-to-dir must not remove the real target directory"
    );
    assert!(
        target_dir.join("inside.txt").exists(),
        "delete on symlink-to-dir must not remove target contents"
    );
    assert_eq!(
        fs::read(target_dir.join("inside.txt")).expect("read inside"),
        b"keep me"
    );
}

// --- create_directory inside a symlinked parent -----------------------------

#[test]
fn create_directory_inside_symlinked_parent_creates_in_target() {
    // If the parent path is a symlink to a real directory, create_directory
    // should succeed and the new dir should be visible through the real
    // path too.
    let dir = temp();
    let real_parent = dir.path().join("real-parent");
    fs::create_dir(&real_parent).expect("create real-parent");

    let link_parent = dir.path().join("link-parent");
    symlink(&real_parent, &link_parent).expect("symlink parent");

    let result = create_directory(
        link_parent.to_string_lossy().to_string(),
        "child".to_string(),
    )
    .expect("create_directory via symlink");

    let created = PathBuf::from(&result);
    assert!(created.exists());
    assert!(created.is_dir());
    // The new directory must be visible through the real parent path.
    assert!(
        real_parent.join("child").exists(),
        "new dir should be reachable via the real (non-symlinked) parent path"
    );
}
