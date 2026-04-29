//! Locates the bundled `SKILL.md` Claude skill file and installs it as
//! `~/.claude/skills/kiri-cli/SKILL.md` so that Claude Code running inside
//! kiri terminals can auto-load the kiri-cli skill.
//!
//! Mirrors the cli_install.rs pattern: source resolution from bundled resources
//! or dev-fallback paths, frontmatter version comparison, and atomic idempotent
//! installation. Used by Tauri commands `kiri_skill_status` / `install_kiri_skill`
//! (registered in Task 12).

use serde::{Deserialize, Serialize};
use std::io;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// What needs to happen to bring the installed skill up to the bundled source version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallAction {
    /// Nothing installed yet — fresh install.
    Install,
    /// Installed version is older than source — overwrite.
    Upgrade,
    /// Installed version >= source version — nothing to do.
    None,
}

/// Snapshot returned by `skill_status_inner` for the frontend confirmation dialog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillStatus {
    pub action: InstallAction,
    pub source_version: Option<String>,
    pub installed_version: Option<String>,
    /// Path the skill would be installed to (for display).
    pub install_path: String,
}

/// Result of `install_skill_inner`. `installed_version` is the frontmatter version
/// that ended up on disk (i.e. source_version on success, installed_version on no-op).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallReport {
    pub action: InstallAction,
    pub installed_version: Option<String>,
    pub install_path: String,
}

/// `~/.claude/skills/kiri-cli` — the directory we install the skill into.
pub fn claude_skill_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("skills").join("kiri-cli"))
}

/// `~/.claude/skills/kiri-cli/SKILL.md` — the installed skill file path.
pub fn skill_install_path() -> Option<PathBuf> {
    claude_skill_dir().map(|d| d.join("SKILL.md"))
}

/// Find the bundled `SKILL.md` source file.
///
/// Search order:
/// 1. Bundled resource (release builds): `<resource_dir>/skills/kiri-cli/SKILL.md`
/// 2. Dev fallback: walk up from `current_exe()` directory looking for
///    `resources/skills/kiri-cli/SKILL.md` (covers `target/debug/` and `target/release/`).
pub fn locate_skill_source(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(resource_dir) = app.path().resource_dir() {
        let candidate = resource_dir.join("skills").join("kiri-cli").join("SKILL.md");
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            for rel in ["", "..", "../..", "../../..", "../../../.."] {
                let candidate = exe_dir
                    .join(rel)
                    .join("resources")
                    .join("skills")
                    .join("kiri-cli")
                    .join("SKILL.md");
                if candidate.is_file() {
                    return candidate.canonicalize().ok().or(Some(candidate));
                }
            }
        }
    }

    None
}

/// Extract the `version:` value from a YAML-style frontmatter block.
///
/// Expects the file to begin with `---\n` (or `---\r\n`), followed by key/value
/// lines, then a closing `---` fence. Returns the trimmed value after `version:`,
/// or `None` if the block is absent, malformed, or has no version key.
pub fn parse_frontmatter_version(contents: &str) -> Option<String> {
    let rest = contents
        .strip_prefix("---\n")
        .or_else(|| contents.strip_prefix("---\r\n"))?;

    let mut in_block = false;
    let mut version_value: Option<String> = None;

    for line in rest.lines() {
        // `lines()` strips `\n` but keeps trailing `\r` on CRLF input.
        if line.trim_end_matches('\r') == "---" {
            in_block = true;
            break;
        }
        if let Some(value) = line.strip_prefix("version:") {
            // `trim()` also strips the trailing `\r` left by CRLF line endings.
            version_value = Some(value.trim().to_string());
        }
    }

    if !in_block {
        return None;
    }

    version_value
}

/// Compare an installed version string against the bundled source version.
///
/// Returns:
/// - `InstallAction::None` when `source` is `None` (nothing to install).
/// - `InstallAction::Install` when `installed` is `None`.
/// - `InstallAction::Upgrade` when the installed version is strictly older.
/// - `InstallAction::None` when installed version is equal or newer.
/// - `InstallAction::Upgrade` when either version cannot be parsed (force re-install).
pub fn compare_versions(installed: Option<&str>, source: Option<&str>) -> InstallAction {
    let Some(src) = source else {
        return InstallAction::None;
    };

    let Some(inst) = installed else {
        return InstallAction::Install;
    };

    let parse = |v: &str| -> Option<(u64, u64, u64)> {
        let parts: Vec<&str> = v.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse::<u64>().ok()?;
        let minor = parts[1].parse::<u64>().ok()?;
        let patch = parts[2].parse::<u64>().ok()?;
        Some((major, minor, patch))
    };

    match (parse(inst), parse(src)) {
        (Some(i), Some(s)) => {
            if i < s {
                InstallAction::Upgrade
            } else {
                InstallAction::None
            }
        }
        // Unparseable version — force re-install.
        _ => InstallAction::Upgrade,
    }
}

/// Compute the current install status without writing anything.
pub fn skill_status_inner(app: &AppHandle) -> SkillStatus {
    let install_path = skill_install_path()
        .unwrap_or_default()
        .display()
        .to_string();

    let source_path = locate_skill_source(app);
    if source_path.is_none() {
        log::warn!(
            "kiri-cli skill source not found in bundle or workspace; install dialog will be suppressed"
        );
    }
    let source_version = source_path
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|c| parse_frontmatter_version(&c));

    let installed_version = skill_install_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|c| parse_frontmatter_version(&c));

    let action = compare_versions(
        installed_version.as_deref(),
        source_version.as_deref(),
    );

    SkillStatus {
        action,
        source_version,
        installed_version,
        install_path,
    }
}

/// Install (or upgrade) the skill file atomically.
///
/// When `force` is `false`, skips writing if the action would be `None`
/// (already up to date). When `force` is `true`, always overwrites.
pub fn install_skill_inner(app: &AppHandle, force: bool) -> io::Result<InstallReport> {
    let install_path = skill_install_path()
        .unwrap_or_default()
        .display()
        .to_string();

    let src_path = locate_skill_source(app).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "kiri-cli skill source not found",
        )
    })?;

    let content = std::fs::read_to_string(&src_path)
        .map_err(|e| io::Error::new(e.kind(), format!("reading {}: {e}", src_path.display())))?;
    let source_version = parse_frontmatter_version(&content);

    let installed_version = skill_install_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|c| parse_frontmatter_version(&c));

    let action = compare_versions(installed_version.as_deref(), source_version.as_deref());

    if !force && action == InstallAction::None {
        log::info!(
            "kiri-cli skill is already up to date ({}); skipping install",
            installed_version.as_deref().unwrap_or("unknown")
        );
        return Ok(InstallReport {
            action: InstallAction::None,
            installed_version,
            install_path,
        });
    }

    let dest = skill_install_path().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "could not determine home directory")
    })?;

    let dir = claude_skill_dir().ok_or_else(|| {
        io::Error::new(io::ErrorKind::NotFound, "could not determine home directory")
    })?;

    std::fs::create_dir_all(&dir)?;

    // Atomic write: write to a `.tmp` sibling, then rename over the destination.
    // If the rename fails, best-effort remove the orphan temp file before bubbling.
    {
        use std::io::Write;
        let tmp_path = dest.with_extension("md.tmp");
        let mut tmp_file = std::fs::File::create(&tmp_path).map_err(|e| {
            io::Error::new(e.kind(), format!("creating {}: {e}", tmp_path.display()))
        })?;
        tmp_file.write_all(content.as_bytes()).map_err(|e| {
            io::Error::new(e.kind(), format!("writing {}: {e}", tmp_path.display()))
        })?;
        tmp_file
            .flush()
            .map_err(|e| io::Error::new(e.kind(), format!("flushing {}: {e}", tmp_path.display())))?;
        drop(tmp_file);
        if let Err(e) = std::fs::rename(&tmp_path, &dest) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(io::Error::new(
                e.kind(),
                format!("rename {} -> {}: {e}", tmp_path.display(), dest.display()),
            ));
        }
    }

    let effective_action = if force && action == InstallAction::None {
        // User explicitly forced a re-install of the same version.
        InstallAction::Upgrade
    } else {
        action
    };

    log::info!(
        "installed kiri-cli skill ({:?}): {}",
        effective_action,
        dest.display()
    );

    Ok(InstallReport {
        action: effective_action,
        installed_version: source_version,
        install_path,
    })
}

/// Tauri command: report whether the bundled skill needs install / upgrade / nothing.
#[tauri::command]
pub fn kiri_skill_status(app: tauri::AppHandle) -> SkillStatus {
    skill_status_inner(&app)
}

/// Tauri command: copy the bundled skill into ~/.claude/skills/kiri-cli/SKILL.md.
/// `force=true` overwrites even when versions match.
#[tauri::command]
pub fn install_kiri_skill(app: tauri::AppHandle, force: bool) -> Result<InstallReport, String> {
    install_skill_inner(&app, force).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- parse_frontmatter_version ---

    #[test]
    fn parse_frontmatter_version_extracts_value() {
        let input = "---\nname: x\nversion: 1.2.3\n---\n# body\n";
        assert_eq!(
            parse_frontmatter_version(input),
            Some("1.2.3".to_string())
        );
    }

    #[test]
    fn parse_frontmatter_version_missing_returns_none() {
        let input = "---\nname: x\ndescription: no version here\n---\n";
        assert_eq!(parse_frontmatter_version(input), None);
    }

    #[test]
    fn parse_frontmatter_version_no_frontmatter_returns_none() {
        let input = "# Just a plain markdown file\nversion: 1.0.0\n";
        assert_eq!(parse_frontmatter_version(input), None);
    }

    #[test]
    fn parse_frontmatter_version_unterminated_returns_none() {
        let input = "---\nname: x\nversion: 1.0.0\n";
        assert_eq!(parse_frontmatter_version(input), None);
    }

    #[test]
    fn parse_frontmatter_version_handles_crlf() {
        let input = "---\r\nname: x\r\nversion: 1.2.3\r\n---\r\n# body\r\n";
        assert_eq!(
            parse_frontmatter_version(input),
            Some("1.2.3".to_string())
        );
    }

    // --- compare_versions ---

    #[test]
    fn compare_versions_install_when_no_installed() {
        assert_eq!(
            compare_versions(None, Some("0.1.0")),
            InstallAction::Install
        );
    }

    #[test]
    fn compare_versions_none_when_equal() {
        assert_eq!(
            compare_versions(Some("1.2.3"), Some("1.2.3")),
            InstallAction::None
        );
    }

    #[test]
    fn compare_versions_upgrade_when_installed_older() {
        // Minor bump
        assert_eq!(
            compare_versions(Some("0.1.0"), Some("0.2.0")),
            InstallAction::Upgrade
        );
        // Patch bump
        assert_eq!(
            compare_versions(Some("0.1.0"), Some("0.1.1")),
            InstallAction::Upgrade
        );
        // Major bump
        assert_eq!(
            compare_versions(Some("0.9.9"), Some("1.0.0")),
            InstallAction::Upgrade
        );
    }

    #[test]
    fn compare_versions_none_when_installed_newer() {
        assert_eq!(
            compare_versions(Some("2.0.0"), Some("1.0.0")),
            InstallAction::None
        );
    }

    #[test]
    fn compare_versions_none_when_source_missing() {
        assert_eq!(compare_versions(Some("1.0.0"), None), InstallAction::None);
    }

    #[test]
    fn compare_versions_upgrade_on_unparseable() {
        assert_eq!(
            compare_versions(Some("foo"), Some("1.0.0")),
            InstallAction::Upgrade
        );
    }

    // --- path helpers ---

    #[test]
    fn claude_skill_dir_under_home() {
        let dir = claude_skill_dir().expect("home dir must be available in test env");
        assert!(
            dir.ends_with(".claude/skills/kiri-cli"),
            "expected path ending with .claude/skills/kiri-cli, got: {}",
            dir.display()
        );
    }

    #[test]
    fn skill_install_path_ends_with_skill_md() {
        let path = skill_install_path().expect("home dir must be available in test env");
        assert!(
            path.ends_with(".claude/skills/kiri-cli/SKILL.md"),
            "expected path ending with .claude/skills/kiri-cli/SKILL.md, got: {}",
            path.display()
        );
    }

    // --- filesystem integration (pure logic, no AppHandle) ---

    #[test]
    fn install_writes_file_atomically() {
        use std::io::Write;

        let tmp = tempfile::TempDir::new().unwrap();
        let skill_dir = tmp.path().join(".claude").join("skills").join("kiri-cli");
        let dest = skill_dir.join("SKILL.md");
        let tmp_path = dest.with_extension("md.tmp");

        let content = "---\nname: kiri-cli\nversion: 0.1.0\n---\n# body\n";

        std::fs::create_dir_all(&skill_dir).unwrap();

        // Simulate atomic write used by install_skill_inner.
        let mut tmp_file = std::fs::File::create(&tmp_path).unwrap();
        tmp_file.write_all(content.as_bytes()).unwrap();
        tmp_file.flush().unwrap();
        drop(tmp_file);
        std::fs::rename(&tmp_path, &dest).unwrap();

        let on_disk = std::fs::read_to_string(&dest).unwrap();
        assert_eq!(on_disk, content);
        assert_eq!(
            parse_frontmatter_version(&on_disk),
            Some("0.1.0".to_string())
        );
        // After a successful rename, the temp file must no longer exist.
        assert!(!tmp_path.exists(), "temp file should be gone after rename");
    }
}
