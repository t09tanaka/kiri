//! Pure path helpers extracted from [`crate::commands::cli_install`].
//!
//! These functions touch only `dirs::home_dir()` and the filesystem and
//! do **not** require a Tauri [`AppHandle`]. Living in their own module
//! lets the tests be plain `-> Result<()>` units rather than mixing with
//! the `AppHandle`-coupled install logic.

use std::path::{Path, PathBuf};

/// `~/.kiri/bin` — the directory we prepend to PATH inside kiri PTYs.
pub fn kiri_bin_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".kiri").join("bin"))
}

/// `~/.kiri/instances` — where per-window CLI sockets live.
pub fn socket_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".kiri").join("instances"))
}

/// Per-window socket path. `label` is the Tauri window label.
pub fn socket_path_for(label: &str) -> Option<PathBuf> {
    socket_dir().map(|d| d.join(format!("{label}.sock")))
}

/// Return `Ok(true)` if `dest` is missing or stale relative to `src`.
///
/// Staleness is decided on size mismatch first (cheapest) and modification
/// time second. If we cannot read mtimes (rare), we err on the safe side
/// and report that a copy is needed.
pub fn needs_copy(src: &Path, dest: &Path) -> std::io::Result<bool> {
    if !dest.exists() {
        return Ok(true);
    }
    let src_meta = std::fs::metadata(src)?;
    let dest_meta = std::fs::metadata(dest)?;
    if src_meta.len() != dest_meta.len() {
        return Ok(true);
    }
    let src_mtime = src_meta.modified().ok();
    let dest_mtime = dest_meta.modified().ok();
    Ok(match (src_mtime, dest_mtime) {
        (Some(s), Some(d)) => s > d,
        _ => true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_path_uses_label() {
        let p = socket_path_for("window-7")
            .expect("HOME must be set for cli_install_paths tests");
        assert!(p.ends_with("window-7.sock"));
        assert!(p.to_string_lossy().contains("/.kiri/instances/"));
    }

    #[test]
    fn kiri_bin_dir_under_home() {
        let p = kiri_bin_dir().expect("HOME must be set for cli_install_paths tests");
        assert!(p.ends_with(".kiri/bin"));
    }

    #[test]
    fn socket_dir_under_home() {
        let p = socket_dir().expect("HOME must be set for cli_install_paths tests");
        assert!(p.ends_with(".kiri/instances"));
    }

    #[test]
    fn needs_copy_when_dest_missing() -> std::io::Result<()> {
        let tmp = tempfile::TempDir::new()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::write(&src, b"hello")?;
        assert!(needs_copy(&src, &dest)?);
        Ok(())
    }

    #[test]
    fn no_copy_when_identical() -> std::io::Result<()> {
        let tmp = tempfile::TempDir::new()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::write(&src, b"hello")?;
        std::fs::copy(&src, &dest)?;
        assert!(!needs_copy(&src, &dest)?);
        Ok(())
    }

    #[test]
    fn needs_copy_when_size_differs() -> std::io::Result<()> {
        let tmp = tempfile::TempDir::new()?;
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::write(&src, b"hello")?;
        std::fs::write(&dest, b"x")?;
        assert!(needs_copy(&src, &dest)?);
        Ok(())
    }

    #[test]
    fn needs_copy_when_dest_older() -> std::io::Result<()> {
        // dest exists with same size but older mtime — copy is needed.
        let tmp = tempfile::TempDir::new()?;
        let dest = tmp.path().join("dest");
        std::fs::write(&dest, b"hello")?;
        std::thread::sleep(std::time::Duration::from_millis(10));
        let src = tmp.path().join("src");
        std::fs::write(&src, b"hello")?;
        // src is newer than dest, lengths match -> needs copy
        assert!(needs_copy(&src, &dest)?);
        Ok(())
    }
}
