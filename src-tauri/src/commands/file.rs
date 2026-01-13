use std::path::Path;

use super::file_io::{create_parent_dirs, read_file_contents, write_file_contents};

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()));
    }

    read_file_contents(path)
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    let path = Path::new(&path);

    create_parent_dirs(path)?;

    write_file_contents(path, &content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_read_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Hello, World!").unwrap();

        let result = read_file(file_path.to_string_lossy().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, World!");
    }

    #[test]
    fn test_write_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        let result = write_file(
            file_path.to_string_lossy().to_string(),
            "Test content".to_string(),
        );
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Test content");
    }

    #[test]
    fn test_read_nonexistent_file() {
        let result = read_file("/nonexistent/path/file.txt".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_read_directory_instead_of_file() {
        let dir = tempdir().unwrap();
        let result = read_file(dir.path().to_string_lossy().to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a file"));
    }

    #[test]
    fn test_write_file_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let nested_path = dir.path().join("nested").join("dir").join("test.txt");

        let result = write_file(
            nested_path.to_string_lossy().to_string(),
            "Nested content".to_string(),
        );
        assert!(result.is_ok());

        let content = fs::read_to_string(&nested_path).unwrap();
        assert_eq!(content, "Nested content");
    }

    #[test]
    fn test_write_file_overwrites_existing() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "Original content").unwrap();

        let result = write_file(
            file_path.to_string_lossy().to_string(),
            "New content".to_string(),
        );
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "New content");
    }

    #[test]
    fn test_write_file_empty_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.txt");

        let result = write_file(file_path.to_string_lossy().to_string(), "".to_string());
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "");
    }

    #[test]
    fn test_read_file_utf8_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("utf8.txt");
        fs::write(&file_path, "こんにちは世界🌍").unwrap();

        let result = read_file(file_path.to_string_lossy().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "こんにちは世界🌍");
    }

    #[test]
    fn test_write_file_parent_exists() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("direct.txt");

        let result = write_file(
            file_path.to_string_lossy().to_string(),
            "direct content".to_string(),
        );
        assert!(result.is_ok());

        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "direct content");
    }
}
