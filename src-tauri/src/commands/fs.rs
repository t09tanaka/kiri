use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub is_hidden: bool,
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

        entries.push(FileEntry {
            name: file_name.clone(),
            path: full_path,
            is_dir: file_type.is_dir(),
            is_hidden: is_hidden(&file_name),
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
