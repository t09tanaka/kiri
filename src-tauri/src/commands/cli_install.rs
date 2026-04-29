//! Locates the workspace-built `kiri-cli` binary and installs it as
//! `~/.kiri/bin/kiri` so that PTYs spawned with `~/.kiri/bin` on PATH can
//! invoke it as `kiri`.
//!
//! Two consumers:
//! 1. [`ensure_installed`] — run once at app startup (best-effort; logs
//!    on failure but does not abort).
//! 2. [`kiri_bin_dir`] / [`socket_path_for`] — used by the PTY env injector
//!    in [`crate::commands::terminal::build_shell_command`].

use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

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

/// Find the `kiri-cli` binary that was built alongside this binary.
///
/// Search order:
/// 1. Bundled resource (release builds): `<resource_dir>/kiri-cli`
/// 2. Sibling of `current_exe` (dev builds): `<exe_dir>/kiri-cli`
/// 3. Workspace target dirs relative to `current_exe`:
///    `<exe_dir>/../kiri-cli`, `<exe_dir>/../../kiri-cli`
pub fn locate_cli_binary(app: &AppHandle) -> Option<PathBuf> {
    let exe_name = if cfg!(windows) { "kiri-cli.exe" } else { "kiri-cli" };

    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join(exe_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            for rel in ["", "..", "../.."] {
                let candidate = exe_dir.join(rel).join(exe_name);
                if candidate.is_file() {
                    return candidate.canonicalize().ok().or(Some(candidate));
                }
            }
        }
    }

    None
}

/// Copy the located `kiri-cli` binary to `~/.kiri/bin/kiri` (mode 0755).
/// Skips the copy if the destination is already up to date.
///
/// Returns the path to the installed binary, or `None` if the cli binary
/// could not be located. Errors during copy/permission set are logged and
/// returned as `Err`.
pub fn ensure_installed(app: &AppHandle) -> std::io::Result<Option<PathBuf>> {
    let Some(src) = locate_cli_binary(app) else {
        log::info!(
            "kiri-cli binary not found near {:?}; in-PTY `kiri` command will not be available",
            std::env::current_exe()
        );
        return Ok(None);
    };
    let Some(bin_dir) = kiri_bin_dir() else {
        return Ok(None);
    };
    std::fs::create_dir_all(&bin_dir)?;
    let dest_name = if cfg!(windows) { "kiri.exe" } else { "kiri" };
    let dest = bin_dir.join(dest_name);

    if needs_copy(&src, &dest)? {
        std::fs::copy(&src, &dest)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dest, std::fs::Permissions::from_mode(0o755))?;
        }
        log::info!("installed kiri CLI: {} -> {}", src.display(), dest.display());
    }
    Ok(Some(dest))
}

fn needs_copy(src: &Path, dest: &Path) -> std::io::Result<bool> {
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
        let p = socket_path_for("window-7").unwrap();
        assert!(p.ends_with("window-7.sock"));
        assert!(p.to_string_lossy().contains("/.kiri/instances/"));
    }

    #[test]
    fn kiri_bin_dir_under_home() {
        let p = kiri_bin_dir().unwrap();
        assert!(p.ends_with(".kiri/bin"));
    }

    #[test]
    fn needs_copy_when_dest_missing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::write(&src, b"hello").unwrap();
        assert!(needs_copy(&src, &dest).unwrap());
    }

    #[test]
    fn no_copy_when_identical() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::write(&src, b"hello").unwrap();
        std::fs::copy(&src, &dest).unwrap();
        // dest mtime == src mtime (or later), so no copy needed
        assert!(!needs_copy(&src, &dest).unwrap());
    }

    #[test]
    fn needs_copy_when_size_differs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let src = tmp.path().join("src");
        let dest = tmp.path().join("dest");
        std::fs::write(&src, b"hello").unwrap();
        std::fs::write(&dest, b"x").unwrap();
        assert!(needs_copy(&src, &dest).unwrap());
    }
}
