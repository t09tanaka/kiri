//! Locates the workspace-built `kiri-cli` binary and installs it as
//! `~/.kiri/bin/kiri` so that PTYs spawned with `~/.kiri/bin` on PATH can
//! invoke it as `kiri`.
//!
//! Two consumers:
//! 1. [`ensure_installed`] — run once at app startup (best-effort; logs
//!    on failure but does not abort).
//! 2. [`kiri_bin_dir`] / [`socket_path_for`] — pure path helpers,
//!    re-exported from [`crate::commands::cli_install_paths`] for
//!    backwards compatibility and used by the PTY env injector in
//!    [`crate::commands::terminal::build_shell_command`].

use std::path::PathBuf;
use tauri::{AppHandle, Manager};

// Re-export the pure path helpers so existing call sites
// (`cli_install::kiri_bin_dir`, etc.) keep working. The implementations
// live in [`super::cli_install_paths`] so they can be unit-tested
// without dragging in any Tauri runtime.
pub use super::cli_install_paths::{kiri_bin_dir, needs_copy, socket_dir, socket_path_for};

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

