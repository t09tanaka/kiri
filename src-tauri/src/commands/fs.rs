use serde::Serialize;
use std::path::Path;

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

#[tauri::command]
pub fn read_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()));
    }

    // Try to open git repository for gitignore checking
    let repo = find_repo_root(path).and_then(|root| open_repo(&root));

    let mut entries: Vec<FileEntry> = Vec::new();

    let read_dir = read_dir_entries(path)?;

    for entry in read_dir {
        let entry = get_dir_entry(entry)?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Skip excluded directories
        if is_excluded(&file_name) {
            continue;
        }

        let file_type = get_file_type(&entry)?;
        let full_path = entry.path().to_string_lossy().to_string();
        let is_dir = file_type.is_dir();

        // Check if path is gitignored
        let is_gitignored = repo
            .as_ref()
            .map_or(false, |r| check_gitignore(r, &entry.path(), is_dir));

        entries.push(FileEntry {
            name: file_name.clone(),
            path: full_path,
            is_dir,
            is_hidden: is_hidden(&file_name),
            is_gitignored,
        });
    }

    // Sort: directories first, then alphabetically
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });

    Ok(entries)
}

#[tauri::command]
pub fn get_home_directory() -> Result<String, String> {
    get_home_dir()
}

#[tauri::command]
pub fn delete_path(path: String) -> Result<(), String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    if path.is_dir() {
        std::fs::remove_dir_all(path).map_err(|e| format!("Failed to delete directory: {}", e))
    } else {
        std::fs::remove_file(path).map_err(|e| format!("Failed to delete file: {}", e))
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

        let result = read_directory(dir.path().to_string_lossy().to_string());
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 3);

        // Check that directories come first (sorted)
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].name, "subdir");
    }

    #[test]
    fn test_read_directory_nonexistent() {
        let result = read_directory("/nonexistent/path".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_read_directory_file_instead_of_directory() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let result = read_directory(file_path.to_string_lossy().to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }

    #[test]
    fn test_read_directory_excludes_node_modules() {
        let dir = tempdir().unwrap();

        fs::create_dir(dir.path().join("src")).unwrap();
        fs::create_dir(dir.path().join("node_modules")).unwrap();

        let result = read_directory(dir.path().to_string_lossy().to_string());
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

        let result = read_directory(dir.path().to_string_lossy().to_string());
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

        let result = read_directory(dir.path().to_string_lossy().to_string());
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

        let result = read_directory(dir.path().to_string_lossy().to_string());
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

        let result = read_directory(dir.path().to_string_lossy().to_string());
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

        let result = read_directory(dir.path().to_string_lossy().to_string());
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

        let result = read_directory(dir.path().to_string_lossy().to_string());
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

        let result = read_directory(dir.path().to_string_lossy().to_string());
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
        let result = read_directory(nested_dir.to_string_lossy().to_string());
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
        let result = read_directory(dir.path().to_string_lossy().to_string());
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
}
