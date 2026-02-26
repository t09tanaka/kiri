use serde::Serialize;
use std::ffi::OsStr;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct CopyResult {
    pub success: bool,
    pub copied: Vec<String>,
    pub errors: Vec<CopyError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CopyError {
    pub path: String,
    pub error: String,
}

/// Generate a unique filename when a file with the same name already exists.
/// e.g., "file.txt" -> "file (1).txt", "file (1).txt" -> "file (2).txt"
pub fn generate_unique_name(name: &str, target_dir: &Path) -> String {
    let target_path = target_dir.join(name);
    if !target_path.exists() {
        return name.to_string();
    }

    let path = Path::new(name);
    let stem = path.file_stem().and_then(OsStr::to_str).unwrap_or(name);
    let extension = path.extension().and_then(OsStr::to_str);

    // Check if stem already ends with " (N)" pattern
    let (base_stem, start_num) = parse_existing_number(stem);

    let mut counter = start_num;
    loop {
        let new_name = match extension {
            Some(ext) => format!("{} ({}).{}", base_stem, counter, ext),
            None => format!("{} ({})", base_stem, counter),
        };

        let new_path = target_dir.join(&new_name);
        if !new_path.exists() {
            return new_name;
        }
        counter += 1;

        // Safety limit to prevent infinite loop
        if counter > 10000 {
            return new_name;
        }
    }
}

/// Parse existing " (N)" pattern from stem and return base name and next number
fn parse_existing_number(stem: &str) -> (&str, u32) {
    if let Some(idx) = stem.rfind(" (") {
        if stem.ends_with(')') {
            let num_str = &stem[idx + 2..stem.len() - 1];
            if let Ok(num) = num_str.parse::<u32>() {
                return (&stem[..idx], num + 1);
            }
        }
    }
    (stem, 1)
}

/// Copy a single file to target directory
fn copy_file(source: &Path, target_dir: &Path) -> Result<String, String> {
    let file_name = source
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "Invalid file name".to_string())?;

    let unique_name = generate_unique_name(file_name, target_dir);
    let target_path = target_dir.join(&unique_name);

    std::fs::copy(source, &target_path)
        .map_err(|e| format!("Failed to copy file: {}", e))?;

    Ok(target_path.to_string_lossy().to_string())
}

/// Recursively copy a directory to target directory
fn copy_directory(source: &Path, target_dir: &Path) -> Result<String, String> {
    let dir_name = source
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "Invalid directory name".to_string())?;

    let unique_name = generate_unique_name(dir_name, target_dir);
    let target_path = target_dir.join(&unique_name);

    // Create the target directory
    std::fs::create_dir(&target_path)
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    // Copy contents recursively
    copy_directory_contents(source, &target_path)?;

    Ok(target_path.to_string_lossy().to_string())
}

/// Copy contents of source directory to target directory
fn copy_directory_contents(source: &Path, target: &Path) -> Result<(), String> {
    let entries = std::fs::read_dir(source)
        .map_err(|e| format!("Failed to read directory: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = path
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or_else(|| "Invalid directory name".to_string())?;
            let new_target = target.join(dir_name);
            std::fs::create_dir(&new_target)
                .map_err(|e| format!("Failed to create subdirectory: {}", e))?;
            copy_directory_contents(&path, &new_target)?;
        } else {
            let file_name = path
                .file_name()
                .and_then(OsStr::to_str)
                .ok_or_else(|| "Invalid file name".to_string())?;
            let target_file = target.join(file_name);
            std::fs::copy(&path, &target_file)
                .map_err(|e| format!("Failed to copy file: {}", e))?;
        }
    }

    Ok(())
}

/// Copy files/directories to specified directory
#[tauri::command]
pub fn copy_paths_to_directory(
    source_paths: Vec<String>,
    target_dir: String,
) -> Result<CopyResult, String> {
    let target_path = Path::new(&target_dir);

    if !target_path.exists() {
        return Err(format!("Target directory does not exist: {}", target_dir));
    }

    if !target_path.is_dir() {
        return Err(format!("Target path is not a directory: {}", target_dir));
    }

    let mut copied: Vec<String> = Vec::new();
    let mut errors: Vec<CopyError> = Vec::new();

    for source in &source_paths {
        let source_path = Path::new(source);

        if !source_path.exists() {
            errors.push(CopyError {
                path: source.clone(),
                error: "Source path does not exist".to_string(),
            });
            continue;
        }

        let result = if source_path.is_dir() {
            copy_directory(source_path, target_path)
        } else {
            copy_file(source_path, target_path)
        };

        match result {
            Ok(path) => copied.push(path),
            Err(e) => errors.push(CopyError {
                path: source.clone(),
                error: e,
            }),
        }
    }

    Ok(CopyResult {
        success: errors.is_empty(),
        copied,
        errors,
    })
}

/// Move a file or directory to a target directory.
/// Tries fs::rename first (fast, same filesystem), falls back to copy + delete.
#[tauri::command]
pub fn move_path(source: String, target_dir: String) -> Result<String, String> {
    let source_path = Path::new(&source);
    let target_dir_path = Path::new(&target_dir);

    // Validate source exists
    if !source_path.exists() {
        return Err(format!("Source path does not exist: {}", source));
    }

    // Validate target_dir exists and is a directory
    if !target_dir_path.exists() {
        return Err(format!("Target directory does not exist: {}", target_dir));
    }
    if !target_dir_path.is_dir() {
        return Err(format!("Target path is not a directory: {}", target_dir));
    }

    // Canonicalize target directory once for both checks
    let canon_target = target_dir_path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve target path: {}", e))?;

    // Prevent moving to same directory (source's parent == target_dir)
    if let Some(parent) = source_path.parent() {
        let canon_parent = parent
            .canonicalize()
            .map_err(|e| format!("Failed to resolve source parent path: {}", e))?;
        if canon_parent == canon_target {
            return Err(format!(
                "Item is already in the target directory: {}",
                source
            ));
        }
    }

    // Prevent moving directory into its own descendant
    if source_path.is_dir() {
        let canon_source = source_path
            .canonicalize()
            .map_err(|e| format!("Failed to resolve source path: {}", e))?;
        if canon_target.starts_with(&canon_source) {
            return Err(format!(
                "Cannot move a directory into its own subdirectory: {} -> {}",
                source, target_dir
            ));
        }
    }

    // Determine the final name, handling conflicts
    let file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid source file name".to_string())?;

    let unique_name = generate_unique_name(file_name, target_dir_path);
    let final_path = target_dir_path.join(&unique_name);

    // Try fs::rename first (fast, same filesystem)
    if std::fs::rename(source_path, &final_path).is_ok() {
        return Ok(final_path.to_string_lossy().to_string());
    }

    // Fall back to copy + delete for cross-device moves
    if source_path.is_dir() {
        std::fs::create_dir(&final_path)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
        if let Err(e) = copy_directory_contents(source_path, &final_path) {
            // Clean up partial copy
            let _ = std::fs::remove_dir_all(&final_path);
            return Err(e);
        }
        std::fs::remove_dir_all(source_path)
            .map_err(|e| format!("Failed to remove source directory after copy: {}", e))?;
    } else {
        std::fs::copy(source_path, &final_path)
            .map_err(|e| format!("Failed to copy file: {}", e))?;
        std::fs::remove_file(source_path)
            .map_err(|e| format!("Failed to remove source file after copy: {}", e))?;
    }

    Ok(final_path.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_generate_unique_name_no_conflict() {
        let dir = tempdir().unwrap();
        let result = generate_unique_name("file.txt", dir.path());
        assert_eq!(result, "file.txt");
    }

    #[test]
    fn test_generate_unique_name_with_conflict() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file.txt"), "content").unwrap();

        let result = generate_unique_name("file.txt", dir.path());
        assert_eq!(result, "file (1).txt");
    }

    #[test]
    fn test_generate_unique_name_multiple_conflicts() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file.txt"), "content").unwrap();
        fs::write(dir.path().join("file (1).txt"), "content").unwrap();
        fs::write(dir.path().join("file (2).txt"), "content").unwrap();

        let result = generate_unique_name("file.txt", dir.path());
        assert_eq!(result, "file (3).txt");
    }

    #[test]
    fn test_generate_unique_name_no_extension() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file"), "content").unwrap();

        let result = generate_unique_name("file", dir.path());
        assert_eq!(result, "file (1)");
    }

    #[test]
    fn test_generate_unique_name_directory() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("folder")).unwrap();

        let result = generate_unique_name("folder", dir.path());
        assert_eq!(result, "folder (1)");
    }

    #[test]
    fn test_generate_unique_name_existing_numbered() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("file (1).txt"), "content").unwrap();

        let result = generate_unique_name("file (1).txt", dir.path());
        assert_eq!(result, "file (2).txt");
    }

    #[test]
    fn test_parse_existing_number_no_number() {
        let (base, num) = parse_existing_number("file");
        assert_eq!(base, "file");
        assert_eq!(num, 1);
    }

    #[test]
    fn test_parse_existing_number_with_number() {
        let (base, num) = parse_existing_number("file (3)");
        assert_eq!(base, "file");
        assert_eq!(num, 4);
    }

    #[test]
    fn test_parse_existing_number_invalid_format() {
        let (base, num) = parse_existing_number("file (abc)");
        assert_eq!(base, "file (abc)");
        assert_eq!(num, 1);
    }

    #[test]
    fn test_copy_paths_to_directory_single_file() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(source_dir.path().join("test.txt"), "content").unwrap();

        let result = copy_paths_to_directory(
            vec![source_dir.path().join("test.txt").to_string_lossy().to_string()],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.copied.len(), 1);
        assert!(result.errors.is_empty());
        assert!(target_dir.path().join("test.txt").exists());
    }

    #[test]
    fn test_copy_paths_to_directory_multiple_files() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(source_dir.path().join("file1.txt"), "content1").unwrap();
        fs::write(source_dir.path().join("file2.txt"), "content2").unwrap();

        let result = copy_paths_to_directory(
            vec![
                source_dir.path().join("file1.txt").to_string_lossy().to_string(),
                source_dir.path().join("file2.txt").to_string_lossy().to_string(),
            ],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.copied.len(), 2);
        assert!(target_dir.path().join("file1.txt").exists());
        assert!(target_dir.path().join("file2.txt").exists());
    }

    #[test]
    fn test_copy_paths_to_directory_with_conflict() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(source_dir.path().join("test.txt"), "source content").unwrap();
        fs::write(target_dir.path().join("test.txt"), "existing content").unwrap();

        let result = copy_paths_to_directory(
            vec![source_dir.path().join("test.txt").to_string_lossy().to_string()],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.copied.len(), 1);
        assert!(target_dir.path().join("test.txt").exists());
        assert!(target_dir.path().join("test (1).txt").exists());

        // Verify original was not overwritten
        let original_content = fs::read_to_string(target_dir.path().join("test.txt")).unwrap();
        assert_eq!(original_content, "existing content");

        // Verify copy has correct content
        let copy_content = fs::read_to_string(target_dir.path().join("test (1).txt")).unwrap();
        assert_eq!(copy_content, "source content");
    }

    #[test]
    fn test_copy_paths_to_directory_directory() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let subdir = source_dir.path().join("mydir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.txt"), "content").unwrap();

        let result = copy_paths_to_directory(
            vec![subdir.to_string_lossy().to_string()],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.copied.len(), 1);
        assert!(target_dir.path().join("mydir").exists());
        assert!(target_dir.path().join("mydir").join("file.txt").exists());
    }

    #[test]
    fn test_copy_paths_to_directory_nested_directory() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let nested = source_dir.path().join("level1").join("level2");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("deep.txt"), "deep content").unwrap();
        fs::write(source_dir.path().join("level1").join("mid.txt"), "mid content").unwrap();

        let result = copy_paths_to_directory(
            vec![source_dir.path().join("level1").to_string_lossy().to_string()],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert!(target_dir.path().join("level1").exists());
        assert!(target_dir.path().join("level1").join("mid.txt").exists());
        assert!(target_dir.path().join("level1").join("level2").exists());
        assert!(target_dir.path().join("level1").join("level2").join("deep.txt").exists());
    }

    #[test]
    fn test_copy_paths_to_directory_nonexistent_source() {
        let target_dir = tempdir().unwrap();

        let result = copy_paths_to_directory(
            vec!["/nonexistent/path/file.txt".to_string()],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.success);
        assert!(result.copied.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].error.contains("does not exist"));
    }

    #[test]
    fn test_copy_paths_to_directory_nonexistent_target() {
        let source_dir = tempdir().unwrap();
        fs::write(source_dir.path().join("test.txt"), "content").unwrap();

        let result = copy_paths_to_directory(
            vec![source_dir.path().join("test.txt").to_string_lossy().to_string()],
            "/nonexistent/target/directory".to_string(),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_copy_paths_to_directory_target_is_file() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(source_dir.path().join("source.txt"), "content").unwrap();
        let target_file = target_dir.path().join("target.txt");
        fs::write(&target_file, "target").unwrap();

        let result = copy_paths_to_directory(
            vec![source_dir.path().join("source.txt").to_string_lossy().to_string()],
            target_file.to_string_lossy().to_string(),
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a directory"));
    }

    #[test]
    fn test_copy_paths_to_directory_mixed_success_error() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(source_dir.path().join("valid.txt"), "content").unwrap();

        let result = copy_paths_to_directory(
            vec![
                source_dir.path().join("valid.txt").to_string_lossy().to_string(),
                "/nonexistent/file.txt".to_string(),
            ],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.success); // Has errors
        assert_eq!(result.copied.len(), 1);
        assert_eq!(result.errors.len(), 1);
    }

    #[test]
    fn test_copy_file_function() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let source_file = source_dir.path().join("test.txt");
        fs::write(&source_file, "content").unwrap();

        let result = copy_file(&source_file, target_dir.path());
        assert!(result.is_ok());
        assert!(target_dir.path().join("test.txt").exists());
    }

    #[test]
    fn test_copy_directory_function() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let subdir = source_dir.path().join("mydir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.txt"), "content").unwrap();

        let result = copy_directory(&subdir, target_dir.path());
        assert!(result.is_ok());
        assert!(target_dir.path().join("mydir").exists());
        assert!(target_dir.path().join("mydir").join("file.txt").exists());
    }

    #[test]
    fn test_move_path_file() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let source_file = source_dir.path().join("test.txt");
        fs::write(&source_file, "hello world").unwrap();

        let result = move_path(
            source_file.to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(Path::new(&final_path).exists());
        assert!(!source_file.exists(), "Source file should be removed after move");
        assert_eq!(
            fs::read_to_string(&final_path).unwrap(),
            "hello world"
        );
    }

    #[test]
    fn test_move_path_directory() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let subdir = source_dir.path().join("mydir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.txt"), "content").unwrap();
        fs::create_dir(subdir.join("nested")).unwrap();
        fs::write(subdir.join("nested").join("deep.txt"), "deep").unwrap();

        let result = move_path(
            subdir.to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(Path::new(&final_path).exists());
        assert!(!subdir.exists(), "Source directory should be removed after move");
        assert!(Path::new(&final_path).join("file.txt").exists());
        assert!(Path::new(&final_path).join("nested").join("deep.txt").exists());
    }

    #[test]
    fn test_move_path_name_conflict() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let source_file = source_dir.path().join("test.txt");
        fs::write(&source_file, "source content").unwrap();
        fs::write(target_dir.path().join("test.txt"), "existing content").unwrap();

        let result = move_path(
            source_file.to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(final_path.contains("test (1).txt"));
        assert!(!source_file.exists(), "Source file should be removed after move");
        // Original should not be overwritten
        assert_eq!(
            fs::read_to_string(target_dir.path().join("test.txt")).unwrap(),
            "existing content"
        );
        assert_eq!(
            fs::read_to_string(&final_path).unwrap(),
            "source content"
        );
    }

    #[test]
    fn test_move_path_into_descendant_rejected() {
        let dir = tempdir().unwrap();

        let parent = dir.path().join("parent");
        let child = parent.join("child");
        fs::create_dir_all(&child).unwrap();

        let result = move_path(
            parent.to_string_lossy().to_string(),
            child.to_string_lossy().to_string(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Cannot move"), "Error should mention 'Cannot move', got: {}", err);
    }

    #[test]
    fn test_move_path_to_same_directory_rejected() {
        let dir = tempdir().unwrap();

        let source_file = dir.path().join("test.txt");
        fs::write(&source_file, "content").unwrap();

        let result = move_path(
            source_file.to_string_lossy().to_string(),
            dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("already in"), "Error should mention 'already in', got: {}", err);
    }

    #[test]
    fn test_move_path_nonexistent_source() {
        let target_dir = tempdir().unwrap();

        let result = move_path(
            "/nonexistent/path/file.txt".to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("does not exist"), "Error should mention 'does not exist', got: {}", err);
    }

    #[test]
    fn test_move_path_nonexistent_target() {
        let source_dir = tempdir().unwrap();
        let source_file = source_dir.path().join("test.txt");
        fs::write(&source_file, "content").unwrap();

        let result = move_path(
            source_file.to_string_lossy().to_string(),
            "/nonexistent/target/directory".to_string(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("does not exist"), "Error should mention 'does not exist', got: {}", err);
    }
}
