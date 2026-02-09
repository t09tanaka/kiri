use std::fs;
use std::path::Path;

use base64::Engine;

use super::file_io::read_file_contents;

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
pub fn read_file_as_base64(path: String) -> Result<String, String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Err(format!("File does not exist: {}", path.display()));
    }

    if !path.is_file() {
        return Err(format!("Path is not a file: {}", path.display()));
    }

    let bytes = fs::read(path).map_err(|e| format!("Failed to read file: {}", e))?;
    Ok(base64::engine::general_purpose::STANDARD.encode(&bytes))
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
    fn test_read_file_utf8_content() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("utf8.txt");
        fs::write(&file_path, "こんにちは世界🌍").unwrap();

        let result = read_file(file_path.to_string_lossy().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "こんにちは世界🌍");
    }

    #[test]
    fn test_read_file_as_base64() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.bin");
        let binary_data: Vec<u8> = vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        fs::write(&file_path, &binary_data).unwrap();

        let result = read_file_as_base64(file_path.to_string_lossy().to_string());
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "iVBORw0KGgo=");
    }

    #[test]
    fn test_read_file_as_base64_nonexistent() {
        let result = read_file_as_base64("/nonexistent/path/file.png".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_read_file_as_base64_directory() {
        let dir = tempdir().unwrap();
        let result = read_file_as_base64(dir.path().to_string_lossy().to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a file"));
    }
}
