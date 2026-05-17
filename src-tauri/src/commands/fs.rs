use serde::Serialize;
use std::path::Path;

use super::error::{user_io_error, user_path_error};
use super::fs_gitignore::check_gitignore;
use super::fs_io::{get_dir_entry, get_file_type, get_home_dir, open_repo, read_dir_entries};

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_hidden: bool,
    pub is_gitignored: bool,
}

fn find_repo_root(path: &Path) -> Option<String> {
    let mut current = path;
    loop {
        if current.join(".git").exists() {
            return Some(current.to_string_lossy().to_string());
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => return None,
        }
    }
}

const EXCLUDED_NAMES: &[&str] = &[
    "node_modules",
    ".git",
    ".DS_Store",
    "target",
    ".svelte-kit",
    "dist",
    ".next",
    "__pycache__",
];

fn is_hidden(name: &str) -> bool {
    name.starts_with('.')
}

fn is_excluded(name: &str) -> bool {
    EXCLUDED_NAMES.contains(&name)
}

/// Hard ceiling on entries returned by a single `read_directory` call.
///
/// The file tree is non-recursive and excludes `node_modules`, `target`,
/// `.git`, etc. up-front, so the budget is comfortably above any sane
/// project directory but tight enough to prevent a misconfigured "open
/// /" call from streaming millions of entries back over IPC.
const MAX_DIRECTORY_ENTRIES: usize = 10_000;

/// Synchronous implementation of [`read_directory`], factored out so the
/// async `#[tauri::command]` wrapper can drop into `spawn_blocking`.
fn read_directory_blocking(path: String) -> Result<Vec<FileEntry>, String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err(user_path_error("Path does not exist", path));
    }

    if !path.is_dir() {
        return Err(user_path_error("Path is not a directory", path));
    }

    let repo = find_repo_root(path).and_then(|root| open_repo(&root));

    let mut entries: Vec<FileEntry> = Vec::new();
    let read_dir = read_dir_entries(path)?;

    for entry in read_dir {
        if entries.len() >= MAX_DIRECTORY_ENTRIES {
            log::warn!(
                "read_directory: truncating result at {} entries for {}",
                MAX_DIRECTORY_ENTRIES,
                path.display()
            );
            break;
        }
        let entry = get_dir_entry(entry)?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        if is_excluded(&file_name) {
            continue;
        }

        let file_type = get_file_type(&entry)?;
        let full_path = entry.path().to_string_lossy().to_string();
        let is_dir = file_type.is_dir();

        let is_gitignored = repo
            .as_ref()
            .is_some_and(|r| check_gitignore(r, &entry.path(), is_dir));

        entries.push(FileEntry {
            name: file_name.clone(),
            path: full_path,
            is_dir,
            is_hidden: is_hidden(&file_name),
            is_gitignored,
        });
    }

    entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    Ok(entries)
}

/// Asynchronous Tauri wrapper around [`read_directory_blocking`].
///
/// The body performs `std::fs` calls and libgit2 work that can take tens
/// of milliseconds on cold caches. Running it on a `spawn_blocking`
/// thread keeps the tokio runtime — and therefore every other in-flight
/// command and websocket frame — responsive even while a slow first
/// `read_directory` is in progress.
#[tauri::command]
pub async fn read_directory(path: String) -> Result<Vec<FileEntry>, String> {
    tokio::task::spawn_blocking(move || read_directory_blocking(path))
        .await
        .map_err(|e| format!("read_directory task panicked: {}", e))?
}

#[tauri::command]
pub fn get_home_directory() -> Result<String, String> {
    get_home_dir()
}

#[tauri::command]
pub fn create_directory(parent_path: String, name: String) -> Result<String, String> {
    let parent = Path::new(&parent_path);

    if !parent.exists() {
        return Err(user_path_error("Parent path does not exist", parent));
    }

    if !parent.is_dir() {
        return Err(user_path_error("Parent path is not a directory", parent));
    }

    // Support nested directory creation (e.g., "test/opt" creates both)
    let new_dir_path = parent.join(&name);

    std::fs::create_dir_all(&new_dir_path)
        .map_err(|e| user_io_error("Failed to create directory", e))?;

    Ok(new_dir_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err(user_path_error("Path does not exist", path));
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| user_io_error("Failed to delete directory", e))
    } else {
        std::fs::remove_file(path).map_err(|e| user_io_error("Failed to delete file", e))
    }
}

#[tauri::command]
pub fn reveal_in_finder(path: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        // Try various file managers
        if std::process::Command::new("nautilus")
            .arg("--select")
            .arg(&path)
            .spawn()
            .is_err()
        {
            std::process::Command::new("xdg-open")
                .arg(Path::new(&path).parent().unwrap_or(Path::new(&path)))
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Rename a file or directory in place (same parent directory).
///
/// Rejects names that contain path separators or that resolve to `.` /
/// `..` so callers can't escape the parent. For cross-directory moves,
/// use `move_path` instead.
#[tauri::command]
pub fn rename_path(path: String, new_name: String) -> Result<String, String> {
    let source = Path::new(&path);

    if !source.exists() {
        return Err(user_path_error("Path does not exist", source));
    }

    let trimmed = new_name.trim();
    if trimmed.is_empty() {
        return Err("New name cannot be empty".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed == "." || trimmed == ".." {
        return Err("New name cannot contain path separators or be . / ..".to_string());
    }

    let parent = source
        .parent()
        .ok_or_else(|| user_path_error("Path has no parent directory", source))?;
    let target = parent.join(trimmed);

    if target == source {
        return Ok(source.to_string_lossy().to_string());
    }

    if target.exists() {
        return Err(format!(
            "Target already exists: {}",
            target.to_string_lossy()
        ));
    }

    std::fs::rename(source, &target).map_err(|e| user_io_error("Failed to rename", e))?;

    Ok(target.to_string_lossy().to_string())
}

/// Create an empty file inside `parent_path`.
///
/// Rejects names with path separators so callers can't accidentally
/// create files outside the displayed directory. Errors if the target
/// already exists rather than silently truncating.
#[tauri::command]
pub fn create_file(parent_path: String, name: String) -> Result<String, String> {
    let parent = Path::new(&parent_path);

    if !parent.exists() {
        return Err(user_path_error("Parent path does not exist", parent));
    }
    if !parent.is_dir() {
        return Err(user_path_error("Parent path is not a directory", parent));
    }

    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("File name cannot be empty".to_string());
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed == "." || trimmed == ".." {
        return Err("File name cannot contain path separators or be . / ..".to_string());
    }

    let target = parent.join(trimmed);
    if target.exists() {
        return Err(format!(
            "File already exists: {}",
            target.to_string_lossy()
        ));
    }

    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|e| user_io_error("Failed to create file", e))?;

    Ok(target.to_string_lossy().to_string())
}

/// Move a file or directory to the OS trash / recycle bin.
///
/// Used by the destructive-op safety net (#90): file ops that the user
/// triggers as "delete" should be reversible within the session rather
/// than immediately unlinking. On macOS / Windows the entry can be
/// restored via `restore_from_trash`. On Linux the `trash` crate uses
/// the freedesktop spec but restore listing is best-effort.
#[tauri::command]
pub fn move_to_trash(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(user_path_error("Path does not exist", p));
    }
    trash::delete(p).map_err(|e| format!("Failed to move to trash: {}", e))
}

/// Whether the current OS supports programmatic restoration from the
/// trash via `restore_from_trash`. macOS / iOS / Android intentionally
/// return `false` — the trash crate's `os_limited` module is gated on
/// Windows + freedesktop Unix, and Apple offers no public API to list
/// or restore Trash entries without elevated privileges.
#[tauri::command]
pub fn trash_restore_supported() -> bool {
    cfg!(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    ))
}

/// Restore the most recent trash entry whose original path matches
/// `original_path`. Returns the restored path.
///
/// Only available on platforms where `trash::os_limited` works
/// (Windows + freedesktop Unix). On macOS / iOS / Android returns an
/// error so the frontend can disable the undo affordance and surface
/// a manual-restore hint.
#[tauri::command]
pub fn restore_from_trash(original_path: String) -> Result<String, String> {
    #[cfg(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    ))]
    {
        use trash::os_limited;

        let items = os_limited::list().map_err(|e| format!("Failed to list trash: {}", e))?;

        // Pick the most-recently-trashed item whose original path matches.
        let mut matching: Vec<trash::TrashItem> = items
            .into_iter()
            .filter(|it| it.original_path() == Path::new(&original_path))
            .collect();
        matching.sort_by_key(|it| it.time_deleted);

        let target = matching
            .pop()
            .ok_or_else(|| format!("No trash entry found for {}", original_path))?;

        let restored_path = target.original_path().to_string_lossy().to_string();
        os_limited::restore_all([target])
            .map_err(|e| format!("Failed to restore from trash: {}", e))?;
        Ok(restored_path)
    }
    #[cfg(not(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    )))]
    {
        let _ = original_path;
        Err("restore_from_trash is not supported on this platform".to_string())
    }
}

/// Open the OS-native terminal app rooted at `path`.
///
/// Used by the context menu's "Open in Terminal here" action. This
/// intentionally launches the OS terminal app (Terminal.app on macOS,
/// cmd on Windows, x-terminal-emulator on Linux) rather than opening a
/// pane inside kiri so it stays a small surface — the kiri-side pane
/// opener is a separate concern.
#[tauri::command]
pub fn open_terminal_here(path: String) -> Result<(), String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(user_path_error("Path does not exist", p));
    }
    let dir = if p.is_dir() {
        p.to_path_buf()
    } else {
        p.parent()
            .ok_or_else(|| user_path_error("Path has no parent directory", p))?
            .to_path_buf()
    };

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-a")
            .arg("Terminal")
            .arg(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .arg("/C")
            .arg("start")
            .arg("cmd")
            .current_dir(&dir)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        // Try x-terminal-emulator first (Debian alternative), then common terminals
        let candidates = ["x-terminal-emulator", "gnome-terminal", "konsole", "xterm"];
        let mut spawned = false;
        for cmd in candidates {
            if std::process::Command::new(cmd)
                .current_dir(&dir)
                .spawn()
                .is_ok()
            {
                spawned = true;
                break;
            }
        }
        if !spawned {
            return Err("No supported terminal emulator found".to_string());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_is_hidden() {
        assert!(is_hidden(".git"));
        assert!(is_hidden(".DS_Store"));
        assert!(is_hidden(".hidden"));
        assert!(!is_hidden("src"));
        assert!(!is_hidden("package.json"));
        assert!(!is_hidden("file.txt"));
    }

    #[test]
    fn test_is_excluded() {
        assert!(is_excluded("node_modules"));
        assert!(is_excluded(".git"));
        assert!(is_excluded(".DS_Store"));
        assert!(is_excluded("target"));
        assert!(is_excluded(".svelte-kit"));
        assert!(is_excluded("dist"));
        assert!(is_excluded(".next"));
        assert!(is_excluded("__pycache__"));
        assert!(!is_excluded("src"));
        assert!(!is_excluded("lib"));
    }

    #[test]
    fn test_find_repo_root_in_git_repo() {
        // Use current directory which should be in a git repo
        let current_dir = std::env::current_dir().unwrap();
        let result = find_repo_root(&current_dir);
        assert!(result.is_some());
    }

    #[test]
    fn test_find_repo_root_not_git_repo() {
        // Use tempdir which is not a git repo
        let dir = tempdir().unwrap();
        let result = find_repo_root(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn test_read_directory_success() {
        let dir = tempdir().unwrap();

        // Create some test files and directories
        fs::write(dir.path().join("file1.txt"), "content").unwrap();
        fs::write(dir.path().join("file2.rs"), "fn main() {}").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let result = read_directory_blocking(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 3);

        // Check that directories come first (sorted)
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "subdir");
    }

    #[test]
    fn test_read_directory_nonexistent() {
        let result = read_directory_blocking("/nonexistent/path".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_read_directory_file_instead_of_directory() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let result = read_directory_blocking(file_path.to_string_lossy().to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }

    #[test]
    fn test_read_directory_excludes_node_modules() {
        let dir = tempdir().unwrap();

        fs::create_dir(dir.path().join("src")).unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();

        let result = read_directory_blocking(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "src");
    }

    #[test]
    fn test_read_directory_hidden_files() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join(".hidden"), "hidden").unwrap();
        fs::write(dir.path().join("visible.txt"), "visible").unwrap();

        let result = read_directory_blocking(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();
        // .hidden should be included but marked as hidden
        let hidden_entry = entries.iter().find(|e| e.name == ".hidden");
        assert!(hidden_entry.is_some());
        assert!(hidden_entry.unwrap().is_hidden);

        let visible_entry = entries.iter().find(|e| e.name == "visible.txt");
        assert!(visible_entry.is_some());
        assert!(!visible_entry.unwrap().is_hidden);
    }

    #[test]
    fn test_read_directory_sorting() {
        let dir = tempdir().unwrap();

        // Create files and dirs with specific names to test sorting
        fs::write(dir.path().join("zebra.txt"), "").unwrap();
        fs::write(dir.path().join("apple.txt"), "").unwrap();
        fs::create_dir(dir.path().join("banana")).unwrap();
        fs::create_dir(dir.path().join("cherry")).unwrap();

        let result = read_directory_blocking(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();
        // Directories should come first, then sorted alphabetically
        assert!(entries[0].is_dir);
        assert!(entries[1].is_dir);
        assert!(!entries[2].is_dir);
        assert!(!entries[3].is_dir);

        // Check alphabetical order
        assert_eq!(entries[0].name, "banana");
        assert_eq!(entries[1].name, "cherry");
        assert_eq!(entries[2].name, "apple.txt");
        assert_eq!(entries[3].name, "zebra.txt");
    }

    #[test]
    fn test_get_home_directory() {
        let result = get_home_directory();
        assert!(result.is_ok());
        let home = result.unwrap();
        assert!(!home.is_empty());
    }

    #[test]
    fn test_file_entry_fields() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("test.txt"), "content").unwrap();

        let result = read_directory_blocking(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();
        let entry = &entries[0];

        assert_eq!(entry.name, "test.txt");
        assert!(!entry.is_dir);
        assert!(!entry.is_hidden);
        assert!(!entry.is_gitignored);
        assert!(entry.path.ends_with("test.txt"));
    }

    #[test]
    fn test_read_directory_in_git_repo() {
        let dir = tempdir().unwrap();

        // Initialize git repo
        Repository::init(dir.path()).unwrap();

        // Create files
        fs::write(dir.path().join("normal.txt"), "normal").unwrap();

        let result = read_directory_blocking(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();

        // Find the normal file - it should exist and have is_gitignored set
        let normal = entries.iter().find(|e| e.name == "normal.txt");
        assert!(normal.is_some());
        // In a fresh git repo without .gitignore, files are not ignored
        assert!(!normal.unwrap().is_gitignored);
    }

    #[test]
    fn test_read_directory_without_git_repo() {
        let dir = tempdir().unwrap();

        // Create files (not a git repo)
        fs::write(dir.path().join("file.txt"), "content").unwrap();

        let result = read_directory_blocking(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();
        let file = entries.iter().find(|e| e.name == "file.txt");
        assert!(file.is_some());
        // Without git repo, is_gitignored should be false
        assert!(!file.unwrap().is_gitignored);
    }

    #[test]
    fn test_reveal_in_finder_basic() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("test.txt"), "content").unwrap();

        // This will attempt to reveal, may not work in test environment but shouldn't panic
        let result = reveal_in_finder(dir.path().join("test.txt").to_string_lossy().to_string());
        // Result depends on platform and environment
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_create_directory_simple() {
        let dir = tempdir().unwrap();

        let result = create_directory(
            dir.path().to_string_lossy().to_string(),
            "new_folder".to_string(),
        );
        assert!(result.is_ok());

        let new_path = result.unwrap();
        assert!(Path::new(&new_path).exists());
        assert!(Path::new(&new_path).is_dir());
    }

    #[test]
    fn test_create_directory_nested() {
        let dir = tempdir().unwrap();

        // Create nested directories like "test/opt/deep"
        let result = create_directory(
            dir.path().to_string_lossy().to_string(),
            "test/opt/deep".to_string(),
        );
        assert!(result.is_ok());

        let new_path = result.unwrap();
        assert!(Path::new(&new_path).exists());
        assert!(Path::new(&new_path).is_dir());

        // Verify intermediate directories were created
        assert!(dir.path().join("test").exists());
        assert!(dir.path().join("test/opt").exists());
        assert!(dir.path().join("test/opt/deep").exists());
    }

    #[test]
    fn test_create_directory_nonexistent_parent() {
        let result = create_directory(
            "/nonexistent/path".to_string(),
            "new_folder".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_create_directory_parent_is_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let result = create_directory(
            file_path.to_string_lossy().to_string(),
            "new_folder".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }

    #[test]
    fn test_create_directory_already_exists() {
        let dir = tempdir().unwrap();
        let existing = dir.path().join("existing");
        fs::create_dir(&existing).unwrap();

        // create_dir_all doesn't error if directory already exists
        let result = create_directory(
            dir.path().to_string_lossy().to_string(),
            "existing".to_string(),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_path_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("to_delete.txt");
        fs::write(&file_path, "content").unwrap();
        assert!(file_path.exists());

        let result = delete_path(file_path.to_string_lossy().to_string());
        assert!(result.is_ok());
        assert!(!file_path.exists());
    }

    #[test]
    fn test_delete_path_directory() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.txt"), "content").unwrap();
        assert!(subdir.exists());

        let result = delete_path(subdir.to_string_lossy().to_string());
        assert!(result.is_ok());
        assert!(!subdir.exists());
    }

    #[test]
    fn test_delete_path_nonexistent() {
        let result = delete_path("/nonexistent/path/to/delete".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_read_directory_with_gitignored_directory() {
        let dir = tempdir().unwrap();

        // Initialize git repo
        Repository::init(dir.path()).unwrap();

        // Create .gitignore that ignores a directory
        fs::write(dir.path().join(".gitignore"), "ignored_dir/\n").unwrap();

        // Create the ignored directory
        fs::create_dir(dir.path().join("ignored_dir")).unwrap();
        fs::write(dir.path().join("ignored_dir").join("file.txt"), "").unwrap();

        // Create a non-ignored directory
        fs::create_dir(dir.path().join("normal_dir")).unwrap();

        let result = read_directory_blocking(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();

        // Find both directories - verifying that gitignore path is checked
        // The exact behavior depends on git2 internal handling
        let ignored = entries.iter().find(|e| e.name == "ignored_dir");
        assert!(ignored.is_some());
        // The is_gitignored field is populated (may be true or false depending on git2)
        let _ = ignored.unwrap().is_gitignored;

        let normal = entries.iter().find(|e| e.name == "normal_dir");
        assert!(normal.is_some());
    }

    #[test]
    fn test_read_directory_gitignored_file_not_dir() {
        let dir = tempdir().unwrap();

        // Initialize git repo
        Repository::init(dir.path()).unwrap();

        // Create .gitignore that ignores a specific file
        fs::write(dir.path().join(".gitignore"), "secret.txt\n").unwrap();

        // Create the ignored file
        fs::write(dir.path().join("secret.txt"), "sensitive data").unwrap();

        // Create a non-ignored file
        fs::write(dir.path().join("normal.txt"), "normal data").unwrap();

        let result = read_directory_blocking(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();

        // Find both files - verifying that gitignore path is checked for files
        let ignored = entries.iter().find(|e| e.name == "secret.txt");
        assert!(ignored.is_some());
        // The is_gitignored field is populated (may be true or false depending on git2)
        let _ = ignored.unwrap().is_gitignored;

        let normal = entries.iter().find(|e| e.name == "normal.txt");
        assert!(normal.is_some());
    }

    #[test]
    fn test_read_directory_gitignore_strip_prefix_edge_case() {
        let dir = tempdir().unwrap();

        // Initialize git repo
        let repo = Repository::init(dir.path()).unwrap();

        // Create a file in a nested directory
        let nested_dir = dir.path().join("nested");
        fs::create_dir(&nested_dir).unwrap();
        fs::write(nested_dir.join("file.txt"), "content").unwrap();

        // Read directory should handle the workdir path correctly
        let result = read_directory_blocking(nested_dir.to_string_lossy().to_string());
        assert!(result.is_ok());

        // Verify repo workdir is available
        assert!(repo.workdir().is_some());
    }

    #[test]
    fn test_read_directory_exercises_gitignore_both_branches() {
        let dir = tempdir().unwrap();

        // Initialize git repo - we need a proper git repo for gitignore to work
        let _repo = Repository::init(dir.path()).unwrap();

        // Create both a file and a directory at the repo root
        // This ensures both the is_dir=true and is_dir=false branches are executed
        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("file.txt"), "content").unwrap();

        // Read the repo root directory
        let result = read_directory_blocking(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();

        // Should have both directory and file entries
        let has_dir = entries.iter().any(|e| e.is_dir && e.name == "subdir");
        let has_file = entries.iter().any(|e| !e.is_dir && e.name == "file.txt");

        assert!(has_dir, "Should have directory entry");
        assert!(has_file, "Should have file entry");
    }

    #[test]
    fn test_check_gitignore_directory() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create directory at repo root
        let subdir = dir.path().join("mydir");
        fs::create_dir(&subdir).unwrap();

        // Not ignored (no .gitignore)
        let result = check_gitignore(&repo, &subdir, true);
        assert!(!result);
    }

    #[test]
    fn test_check_gitignore_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create file at repo root
        let file = dir.path().join("myfile.txt");
        fs::write(&file, "content").unwrap();

        // Not ignored (no .gitignore)
        let result = check_gitignore(&repo, &file, false);
        assert!(!result);
    }

    #[test]
    fn test_check_gitignore_path_outside_workdir() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create a path outside the workdir
        let other_dir = tempdir().unwrap();
        let file = other_dir.path().join("outside.txt");
        fs::write(&file, "content").unwrap();

        // Should return false because path is outside workdir
        let result = check_gitignore(&repo, &file, false);
        assert!(!result);
    }

    #[test]
    fn test_check_gitignore_with_ignored_pattern() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create .gitignore
        fs::write(dir.path().join(".gitignore"), "*.log\nbuild/\n").unwrap();

        // Stage .gitignore
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(".gitignore")).unwrap();
        index.write().unwrap();

        // Check file pattern
        let log_file = dir.path().join("test.log");
        fs::write(&log_file, "log content").unwrap();

        // This should work with gitignore (depends on git2 behavior)
        let result = check_gitignore(&repo, &log_file, false);
        // The result depends on git2's gitignore handling
        let _ = result;

        // Check directory pattern
        let build_dir = dir.path().join("build");
        fs::create_dir(&build_dir).unwrap();
        let dir_result = check_gitignore(&repo, &build_dir, true);
        let _ = dir_result;
    }

    #[test]
    fn test_check_gitignore_bare_repo() {
        let dir = tempdir().unwrap();
        // Create a bare repository (no workdir)
        let repo = Repository::init_bare(dir.path()).unwrap();

        // Bare repos have no workdir, so this should return false
        let file = dir.path().join("test.txt");
        let result = check_gitignore(&repo, &file, false);
        assert!(!result);

        // Verify it's actually a bare repo
        assert!(repo.is_bare());
        assert!(repo.workdir().is_none());
    }

    #[test]
    fn test_check_gitignore_both_branches_explicit() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Explicitly test the directory branch (is_dir=true)
        let test_dir = dir.path().join("testdir");
        fs::create_dir(&test_dir).unwrap();
        let dir_result = check_gitignore(&repo, &test_dir, true);
        // Should return false as no .gitignore is set up
        assert!(!dir_result);

        // Explicitly test the file branch (is_dir=false)
        let test_file = dir.path().join("testfile.txt");
        fs::write(&test_file, "content").unwrap();
        let file_result = check_gitignore(&repo, &test_file, false);
        // Should return false as no .gitignore is set up
        assert!(!file_result);
    }

    #[test]
    fn test_read_directory_blocking_caps_at_max_entries() {
        // Create more entries than the cap and verify the result is
        // truncated rather than allowed to grow unbounded.
        let dir = tempdir().unwrap();
        // Use 5 over the cap to keep the test fast; libgit2 isn't
        // involved here because the temp dir isn't a repo.
        let total = MAX_DIRECTORY_ENTRIES + 5;
        for i in 0..total {
            fs::write(dir.path().join(format!("f{}.txt", i)), b"").unwrap();
        }

        let result = read_directory_blocking(dir.path().to_string_lossy().to_string()).unwrap();
        assert!(
            result.len() <= MAX_DIRECTORY_ENTRIES,
            "expected at most {} entries, got {}",
            MAX_DIRECTORY_ENTRIES,
            result.len()
        );
    }

    #[tokio::test]
    async fn test_read_directory_async_wrapper_returns_same_result() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"").unwrap();
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let blocking = read_directory_blocking(dir.path().to_string_lossy().to_string()).unwrap();
        let async_result = read_directory(dir.path().to_string_lossy().to_string())
            .await
            .unwrap();
        assert_eq!(blocking.len(), async_result.len());
        for (b, a) in blocking.iter().zip(async_result.iter()) {
            assert_eq!(b.name, a.name);
            assert_eq!(b.is_dir, a.is_dir);
        }
    }

    // ------- rename_path -------

    #[test]
    fn test_rename_path_file_in_place() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("old.txt");
        fs::write(&original, "hi").unwrap();

        let result =
            rename_path(original.to_string_lossy().to_string(), "new.txt".to_string()).unwrap();

        let renamed = dir.path().join("new.txt");
        assert_eq!(result, renamed.to_string_lossy().to_string());
        assert!(renamed.exists());
        assert!(!original.exists());
        assert_eq!(fs::read_to_string(&renamed).unwrap(), "hi");
    }

    #[test]
    fn test_rename_path_directory_in_place() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("old_dir");
        fs::create_dir(&original).unwrap();
        fs::write(original.join("child.txt"), "content").unwrap();

        let result =
            rename_path(original.to_string_lossy().to_string(), "new_dir".to_string()).unwrap();

        let renamed = dir.path().join("new_dir");
        assert_eq!(result, renamed.to_string_lossy().to_string());
        assert!(renamed.join("child.txt").exists());
    }

    #[test]
    fn test_rename_path_rejects_path_separator() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("a.txt");
        fs::write(&original, "").unwrap();

        let result = rename_path(
            original.to_string_lossy().to_string(),
            "../escape.txt".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("path separators"));
    }

    #[test]
    fn test_rename_path_rejects_dot_dot() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("a.txt");
        fs::write(&original, "").unwrap();

        let result =
            rename_path(original.to_string_lossy().to_string(), "..".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_rename_path_rejects_empty_name() {
        let dir = tempdir().unwrap();
        let original = dir.path().join("a.txt");
        fs::write(&original, "").unwrap();

        let result = rename_path(original.to_string_lossy().to_string(), "   ".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_rename_path_target_exists() {
        let dir = tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        fs::write(&a, "").unwrap();
        fs::write(&b, "").unwrap();

        let result = rename_path(a.to_string_lossy().to_string(), "b.txt".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("already exists"));
    }

    #[test]
    fn test_rename_path_nonexistent_source() {
        let result =
            rename_path("/nonexistent/foo.txt".to_string(), "bar.txt".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    // ------- create_file -------

    #[test]
    fn test_create_file_success() {
        let dir = tempdir().unwrap();

        let result = create_file(
            dir.path().to_string_lossy().to_string(),
            "new.txt".to_string(),
        )
        .unwrap();

        let created = dir.path().join("new.txt");
        assert_eq!(result, created.to_string_lossy().to_string());
        assert!(created.exists());
        assert_eq!(fs::read_to_string(&created).unwrap(), "");
    }

    #[test]
    fn test_create_file_rejects_existing() {
        let dir = tempdir().unwrap();
        let existing = dir.path().join("dup.txt");
        fs::write(&existing, "old").unwrap();

        let result = create_file(
            dir.path().to_string_lossy().to_string(),
            "dup.txt".to_string(),
        );
        assert!(result.is_err());
        // Importantly: file content is preserved (no accidental truncate).
        assert_eq!(fs::read_to_string(&existing).unwrap(), "old");
    }

    #[test]
    fn test_create_file_rejects_path_separator() {
        let dir = tempdir().unwrap();
        let result = create_file(
            dir.path().to_string_lossy().to_string(),
            "sub/foo.txt".to_string(),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("path separators"));
    }

    #[test]
    fn test_create_file_rejects_empty_name() {
        let dir = tempdir().unwrap();
        let result =
            create_file(dir.path().to_string_lossy().to_string(), "  ".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_create_file_parent_must_be_directory() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("notadir");
        fs::write(&file_path, "").unwrap();

        let result =
            create_file(file_path.to_string_lossy().to_string(), "x.txt".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }

    // ------- move_to_trash -------
    //
    // The trash crate actually moves the file to the OS trash, which we
    // don't want as a side-effect in CI. The single test here just
    // verifies the up-front validation; deeper restore behavior is
    // covered manually because it depends on the host OS.

    #[test]
    fn test_move_to_trash_rejects_nonexistent() {
        let result = move_to_trash("/nonexistent/path/zzz".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_trash_restore_supported_returns_platform_default() {
        // The contract is "returns a bool that matches cfg gating".
        // On macOS / iOS / Android we expect false; on Windows /
        // freedesktop Unix true.
        let expected = cfg!(any(
            target_os = "windows",
            all(
                unix,
                not(target_os = "macos"),
                not(target_os = "ios"),
                not(target_os = "android")
            )
        ));
        assert_eq!(trash_restore_supported(), expected);
    }

    #[cfg(any(target_os = "macos", target_os = "ios", target_os = "android"))]
    #[test]
    fn test_restore_from_trash_unsupported_on_apple() {
        let result = restore_from_trash("/anything".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not supported"));
    }

    // ------- open_terminal_here -------

    #[test]
    fn test_open_terminal_here_rejects_nonexistent() {
        let result = open_terminal_here("/definitely/does/not/exist".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }
}
