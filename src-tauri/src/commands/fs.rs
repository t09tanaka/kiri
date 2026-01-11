use git2::Repository;
use serde::Serialize;
use std::fs;
use std::path::Path;

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
    let repo = find_repo_root(path).and_then(|root| Repository::open(&root).ok());

    let mut entries: Vec<FileEntry> = Vec::new();

    let read_dir = fs::read_dir(path).map_err(|e| e.to_string())?;

    for entry in read_dir {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Skip excluded directories
        if is_excluded(&file_name) {
            continue;
        }

        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let full_path = entry.path().to_string_lossy().to_string();
        let is_dir = file_type.is_dir();

        // Check if path is gitignored
        let is_gitignored = repo.as_ref().map_or(false, |r| {
            let entry_path = entry.path();
            if let Ok(repo_path) = entry_path.strip_prefix(r.workdir().unwrap_or(Path::new(""))) {
                // For directories, append a trailing slash for correct gitignore matching
                if is_dir {
                    let dir_path = format!("{}/", repo_path.to_string_lossy());
                    r.is_path_ignored(&dir_path).unwrap_or(false)
                } else {
                    r.is_path_ignored(repo_path).unwrap_or(false)
                }
            } else {
                false
            }
        });

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
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .ok_or_else(|| "Could not determine home directory".to_string())
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

    #[test]
    fn test_is_hidden() {
        assert!(is_hidden(".git"));
        assert!(is_hidden(".DS_Store"));
        assert!(!is_hidden("src"));
        assert!(!is_hidden("package.json"));
    }

    #[test]
    fn test_is_excluded() {
        assert!(is_excluded("node_modules"));
        assert!(is_excluded(".git"));
        assert!(!is_excluded("src"));
    }
}
