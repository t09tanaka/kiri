//! Edge-case integration tests for filesystem-touching commands.
//!
//! These tests cover the corner cases that day-to-day usage rarely
//! exercises but that bite hard when they regress:
//! - Unicode (emoji, CJK, RTL) in file and directory names.
//! - 0-byte files.
//! - Mixed line endings.
//! - Long path names.
//! - Permission-denied (EACCES) reads.
//! - Windows-style backslash separators in paths (the parser must not
//!   crash even though we currently only ship on macOS).
//!
//! Each test runs against a hermetic `tempfile::TempDir` and only uses
//! the public API of `app_lib::commands` (no Tauri runtime required).

use app_lib::commands::fs::{create_directory, delete_path, read_directory};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

fn temp() -> TempDir {
    tempfile::tempdir().expect("temp dir")
}

fn write(path: &PathBuf, body: &[u8]) {
    let mut f = File::create(path).expect("create file");
    f.write_all(body).expect("write body");
}

// --- Unicode -----------------------------------------------------------------

#[tokio::test]
async fn read_directory_handles_emoji_filenames() {
    let dir = temp();
    let p = dir.path().join("🐉.txt");
    write(&p, b"hi");

    let entries = read_directory(dir.path().to_string_lossy().to_string())
        .await
        .expect("read_directory should succeed for emoji filename");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "🐉.txt");
    assert!(!entries[0].is_dir);
}

#[tokio::test]
async fn read_directory_handles_cjk_directory_names() {
    let dir = temp();
    let sub = dir.path().join("プロジェクト");
    fs::create_dir(&sub).expect("create cjk dir");
    write(&sub.join("テスト.md"), "# テスト".as_bytes());

    let outer = read_directory(dir.path().to_string_lossy().to_string())
        .await
        .expect("outer");
    assert_eq!(outer.len(), 1);
    assert_eq!(outer[0].name, "プロジェクト");
    assert!(outer[0].is_dir);

    let inner = read_directory(sub.to_string_lossy().to_string())
        .await
        .expect("inner");
    assert_eq!(inner.len(), 1);
    assert_eq!(inner[0].name, "テスト.md");
}

#[tokio::test]
async fn read_directory_handles_rtl_filenames() {
    let dir = temp();
    write(&dir.path().join("مرحبا.txt"), b"hello");

    let entries = read_directory(dir.path().to_string_lossy().to_string())
        .await
        .expect("rtl");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "مرحبا.txt");
}

#[test]
fn create_directory_accepts_nested_unicode_path() {
    let dir = temp();
    let result = create_directory(
        dir.path().to_string_lossy().to_string(),
        "ドキュメント/サブ".to_string(),
    )
    .expect("create nested unicode dir");

    let created = PathBuf::from(&result);
    assert!(created.exists(), "nested unicode path should exist");
    assert!(created.is_dir());
    assert!(created.ends_with("ドキュメント/サブ"));
}

// --- 0-byte files ------------------------------------------------------------

#[tokio::test]
async fn read_directory_returns_zero_byte_files() {
    let dir = temp();
    let empty = dir.path().join("empty.log");
    File::create(&empty).expect("create empty file");

    let entries = read_directory(dir.path().to_string_lossy().to_string())
        .await
        .expect("read");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "empty.log");
    assert!(!entries[0].is_dir);
}

#[test]
fn delete_path_removes_zero_byte_file() {
    let dir = temp();
    let empty = dir.path().join("empty.log");
    File::create(&empty).expect("touch");
    assert!(empty.exists());

    delete_path(empty.to_string_lossy().to_string()).expect("delete");
    assert!(!empty.exists());
}

// --- Mixed line endings ------------------------------------------------------

#[tokio::test]
async fn read_directory_unaffected_by_mixed_line_endings_in_content() {
    // read_directory only enumerates names; content is irrelevant, but
    // we want a regression guard that a CRLF/LF body doesn't somehow
    // break enumeration on macOS HFS+/APFS or future case-insensitive
    // overlays.
    let dir = temp();
    write(&dir.path().join("crlf.txt"), b"line1\r\nline2\r\nline3");
    write(&dir.path().join("lf.txt"), b"line1\nline2\nline3");
    write(&dir.path().join("mixed.txt"), b"a\r\nb\nc\r");

    let entries = read_directory(dir.path().to_string_lossy().to_string())
        .await
        .expect("read");
    let mut names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["crlf.txt", "lf.txt", "mixed.txt"]);
}

// --- Long names --------------------------------------------------------------

#[tokio::test]
async fn read_directory_handles_long_filename_within_pathmax() {
    // APFS allows 255-byte file names. Pick 200 ASCII chars to stay
    // safely below the limit while still exercising "long name" code
    // paths.
    let dir = temp();
    let long = "a".repeat(200);
    write(&dir.path().join(&long), b"x");

    let entries = read_directory(dir.path().to_string_lossy().to_string())
        .await
        .expect("read");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name.len(), 200);
}

// --- Permission denied -------------------------------------------------------

#[cfg(unix)]
#[tokio::test]
async fn read_directory_reports_eacces_on_unreadable_directory() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp();
    let sub = dir.path().join("locked");
    fs::create_dir(&sub).expect("create sub");
    write(&sub.join("hidden.txt"), b"secret");

    // Strip read+execute. Root can still read - skip in that case.
    if nix_is_root() {
        eprintln!("skipping: running as root, chmod 000 is ineffective");
        return;
    }

    let mut perms = fs::metadata(&sub).expect("metadata").permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&sub, perms).expect("chmod 000");

    let result = read_directory(sub.to_string_lossy().to_string()).await;

    // Restore so TempDir's drop can clean up.
    let mut restore = fs::metadata(&sub).expect("metadata").permissions();
    restore.set_mode(0o755);
    fs::set_permissions(&sub, restore).expect("chmod 755");

    assert!(
        result.is_err(),
        "expected Err on locked directory, got Ok({:?})",
        result
    );
}

#[cfg(unix)]
fn nix_is_root() -> bool {
    // Avoid pulling in `nix` / `libc` just for getuid: shell out.
    std::process::Command::new("id")
        .arg("-u")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim() == "0")
        .unwrap_or(false)
}

// --- Not-found / not-a-directory --------------------------------------------

#[tokio::test]
async fn read_directory_errors_for_missing_path() {
    let dir = temp();
    let missing = dir.path().join("does-not-exist");
    let result = read_directory(missing.to_string_lossy().to_string()).await;
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(
        msg.contains("does not exist"),
        "unexpected error message: {msg}"
    );
}

#[tokio::test]
async fn read_directory_errors_when_path_is_a_file() {
    let dir = temp();
    let f = dir.path().join("not-a-dir.txt");
    write(&f, b"hello");

    let result = read_directory(f.to_string_lossy().to_string()).await;
    assert!(result.is_err());
    let msg = result.unwrap_err();
    assert!(
        msg.contains("not a directory"),
        "unexpected error message: {msg}"
    );
}

#[test]
fn delete_path_errors_for_missing_path() {
    let dir = temp();
    let missing = dir.path().join("ghost");
    let result = delete_path(missing.to_string_lossy().to_string());
    assert!(result.is_err());
}

// --- Backslash separators ----------------------------------------------------

#[tokio::test]
async fn read_directory_does_not_crash_on_backslash_separators() {
    // We ship on macOS; backslashes are legal filename characters on
    // Unix. The point is the parser must not split on `\` and must
    // surface a sensible "does not exist" error rather than a panic.
    let dir = temp();
    let weird = dir.path().join("with\\backslash");
    let result = read_directory(weird.to_string_lossy().to_string()).await;
    assert!(result.is_err(), "non-existent backslash path should Err");

    // Now create a real file whose name contains a backslash and make
    // sure enumeration surfaces it unchanged.
    write(&dir.path().join("a\\b.txt"), b"x");
    let entries = read_directory(dir.path().to_string_lossy().to_string())
        .await
        .expect("read");
    let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"a\\b.txt"));
}
