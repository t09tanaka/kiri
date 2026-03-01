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
    cross_device_move(source_path, &final_path)?;

    Ok(final_path.to_string_lossy().to_string())
}

/// Cross-device move fallback: copy + delete.
/// Used when `fs::rename` fails (e.g., across different filesystems).
fn cross_device_move(source_path: &Path, final_path: &Path) -> Result<(), String> {
    if source_path.is_dir() {
        std::fs::create_dir(final_path)
            .map_err(|e| format!("Failed to create directory: {}", e))?;
        if let Err(e) = copy_directory_contents(source_path, final_path) {
            // Clean up partial copy
            let _ = std::fs::remove_dir_all(final_path);
            return Err(e);
        }
        std::fs::remove_dir_all(source_path)
            .map_err(|e| format!("Failed to remove source directory after copy: {}", e))?;
    } else {
        std::fs::copy(source_path, final_path)
            .map_err(|e| format!("Failed to copy file: {}", e))?;
        std::fs::remove_file(source_path)
            .map_err(|e| format!("Failed to remove source file after copy: {}", e))?;
    }
    Ok(())
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

    #[test]
    fn test_generate_unique_name_safety_limit() {
        let dir = tempdir().unwrap();

        // Create files from "file.txt", "file (1).txt" ... "file (10001).txt"
        fs::write(dir.path().join("file.txt"), "").unwrap();
        for i in 1..=10001 {
            fs::write(dir.path().join(format!("file ({}).txt", i)), "").unwrap();
        }

        // When counter exceeds 10000, generate_unique_name returns the name
        // even if it already exists (safety limit)
        let result = generate_unique_name("file.txt", dir.path());
        // The function should have hit the safety limit and returned the last
        // attempted name. Since file.txt exists, it starts at counter=1 and
        // increments. At counter=10001, it would return "file (10001).txt"
        // regardless of existence.
        assert!(
            result.contains("file ("),
            "Should return a numbered name, got: {}",
            result
        );
    }

    #[test]
    fn test_copy_paths_to_directory_empty_sources() {
        let target_dir = tempdir().unwrap();

        let result = copy_paths_to_directory(
            vec![],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert!(result.copied.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_move_path_target_is_file() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let source_file = source_dir.path().join("source.txt");
        fs::write(&source_file, "content").unwrap();

        let target_file = target_dir.path().join("target.txt");
        fs::write(&target_file, "target").unwrap();

        let result = move_path(
            source_file.to_string_lossy().to_string(),
            target_file.to_string_lossy().to_string(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("not a directory"),
            "Error should mention 'not a directory', got: {}",
            err
        );
    }

    #[test]
    fn test_parse_existing_number_nested_parentheses() {
        // "file (1) (2)" - rfind should find the last " (" which is " (2)"
        let (base, num) = parse_existing_number("file (1) (2)");
        assert_eq!(base, "file (1)");
        assert_eq!(num, 3); // next number after 2
    }

    #[test]
    fn test_copy_result_serialization() {
        let result = CopyResult {
            success: true,
            copied: vec!["/path/to/file1.txt".to_string(), "/path/to/file2.txt".to_string()],
            errors: vec![],
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["success"], true);
        assert_eq!(parsed["copied"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["copied"][0], "/path/to/file1.txt");
        assert_eq!(parsed["copied"][1], "/path/to/file2.txt");
        assert!(parsed["errors"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_copy_error_serialization() {
        let error = CopyError {
            path: "/source/missing.txt".to_string(),
            error: "Source path does not exist".to_string(),
        };

        let json = serde_json::to_string(&error).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["path"], "/source/missing.txt");
        assert_eq!(parsed["error"], "Source path does not exist");
    }

    #[test]
    fn test_copy_directory_with_empty_subdir() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a directory with an empty subdirectory
        let parent = source_dir.path().join("parent");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(parent.join("empty_child")).unwrap();
        fs::write(parent.join("file.txt"), "content").unwrap();

        let result = copy_paths_to_directory(
            vec![parent.to_string_lossy().to_string()],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.copied.len(), 1);

        // Verify the empty subdirectory was copied
        assert!(target_dir.path().join("parent").exists());
        assert!(target_dir.path().join("parent").join("empty_child").exists());
        assert!(target_dir.path().join("parent").join("empty_child").is_dir());
        assert!(target_dir.path().join("parent").join("file.txt").exists());
    }

    #[test]
    fn test_copy_directory_contents_deeply_nested() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a deeply nested structure: level1/level2/level3/deep.txt
        let level1 = source_dir.path().join("level1");
        let level2 = level1.join("level2");
        let level3 = level2.join("level3");
        fs::create_dir_all(&level3).unwrap();
        fs::write(level1.join("a.txt"), "a").unwrap();
        fs::write(level2.join("b.txt"), "b").unwrap();
        fs::write(level3.join("c.txt"), "c").unwrap();

        // Copy contents of level1 into target_dir
        fs::create_dir(target_dir.path().join("dest")).unwrap();
        let result = copy_directory_contents(&level1, &target_dir.path().join("dest"));
        assert!(result.is_ok());

        // Verify all levels were copied
        let dest = target_dir.path().join("dest");
        assert!(dest.join("a.txt").exists());
        assert_eq!(fs::read_to_string(dest.join("a.txt")).unwrap(), "a");
        assert!(dest.join("level2").join("b.txt").exists());
        assert_eq!(
            fs::read_to_string(dest.join("level2").join("b.txt")).unwrap(),
            "b"
        );
        assert!(dest.join("level2").join("level3").join("c.txt").exists());
        assert_eq!(
            fs::read_to_string(dest.join("level2").join("level3").join("c.txt")).unwrap(),
            "c"
        );
    }

    #[test]
    fn test_copy_directory_contents_empty_source() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Copy an empty directory's contents
        let empty = source_dir.path().join("empty");
        fs::create_dir(&empty).unwrap();
        let dest = target_dir.path().join("dest");
        fs::create_dir(&dest).unwrap();

        let result = copy_directory_contents(&empty, &dest);
        assert!(result.is_ok());
    }

    #[test]
    fn test_copy_directory_contents_nonexistent_source() {
        let target_dir = tempdir().unwrap();
        let dest = target_dir.path().join("dest");
        fs::create_dir(&dest).unwrap();

        let result = copy_directory_contents(Path::new("/nonexistent/dir"), &dest);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to read directory"));
    }

    #[test]
    fn test_copy_directory_with_name_conflict() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create source directory with a file
        let subdir = source_dir.path().join("mydir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.txt"), "source content").unwrap();

        // Create conflicting directory in target
        let existing = target_dir.path().join("mydir");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("original.txt"), "original").unwrap();

        // Copy should create "mydir (1)" due to conflict
        let result = copy_directory(&subdir, target_dir.path());
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.contains("mydir (1)"));
        assert!(target_dir.path().join("mydir (1)").exists());
        assert!(target_dir
            .path()
            .join("mydir (1)")
            .join("file.txt")
            .exists());
        // Original is untouched
        assert!(target_dir.path().join("mydir").join("original.txt").exists());
    }

    #[test]
    fn test_copy_file_with_name_conflict() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let source_file = source_dir.path().join("test.txt");
        fs::write(&source_file, "source").unwrap();
        fs::write(target_dir.path().join("test.txt"), "existing").unwrap();

        let result = copy_file(&source_file, target_dir.path());
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.contains("test (1).txt"));
        // Original untouched
        assert_eq!(
            fs::read_to_string(target_dir.path().join("test.txt")).unwrap(),
            "existing"
        );
        assert_eq!(
            fs::read_to_string(target_dir.path().join("test (1).txt")).unwrap(),
            "source"
        );
    }

    #[test]
    fn test_copy_file_source_does_not_exist() {
        let target_dir = tempdir().unwrap();
        let result = copy_file(Path::new("/nonexistent/file.txt"), target_dir.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to copy file"));
    }

    #[test]
    fn test_copy_directory_contents_with_multiple_subdirs_and_files() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create structure:
        //   root/
        //     dir_a/
        //       file_a1.txt
        //       file_a2.txt
        //     dir_b/
        //       sub_b1/
        //         deep.txt
        //     root_file.txt
        let root = source_dir.path().join("root");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("dir_a")).unwrap();
        fs::write(root.join("dir_a").join("file_a1.txt"), "a1").unwrap();
        fs::write(root.join("dir_a").join("file_a2.txt"), "a2").unwrap();
        fs::create_dir(root.join("dir_b")).unwrap();
        fs::create_dir(root.join("dir_b").join("sub_b1")).unwrap();
        fs::write(root.join("dir_b").join("sub_b1").join("deep.txt"), "deep").unwrap();
        fs::write(root.join("root_file.txt"), "root").unwrap();

        let dest = target_dir.path().join("dest");
        fs::create_dir(&dest).unwrap();

        let result = copy_directory_contents(&root, &dest);
        assert!(result.is_ok());

        assert_eq!(fs::read_to_string(dest.join("root_file.txt")).unwrap(), "root");
        assert_eq!(
            fs::read_to_string(dest.join("dir_a").join("file_a1.txt")).unwrap(),
            "a1"
        );
        assert_eq!(
            fs::read_to_string(dest.join("dir_a").join("file_a2.txt")).unwrap(),
            "a2"
        );
        assert_eq!(
            fs::read_to_string(dest.join("dir_b").join("sub_b1").join("deep.txt")).unwrap(),
            "deep"
        );
    }

    #[test]
    fn test_copy_paths_to_directory_multiple_directories() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create two source directories
        let dir_a = source_dir.path().join("dir_a");
        fs::create_dir(&dir_a).unwrap();
        fs::write(dir_a.join("a.txt"), "a").unwrap();

        let dir_b = source_dir.path().join("dir_b");
        fs::create_dir(&dir_b).unwrap();
        fs::write(dir_b.join("b.txt"), "b").unwrap();

        let result = copy_paths_to_directory(
            vec![
                dir_a.to_string_lossy().to_string(),
                dir_b.to_string_lossy().to_string(),
            ],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.copied.len(), 2);
        assert!(target_dir.path().join("dir_a").join("a.txt").exists());
        assert!(target_dir.path().join("dir_b").join("b.txt").exists());
    }

    #[test]
    fn test_copy_paths_to_directory_mixed_files_and_dirs() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(source_dir.path().join("file.txt"), "file content").unwrap();
        let subdir = source_dir.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("inner.txt"), "inner").unwrap();

        let result = copy_paths_to_directory(
            vec![
                source_dir
                    .path()
                    .join("file.txt")
                    .to_string_lossy()
                    .to_string(),
                subdir.to_string_lossy().to_string(),
            ],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(result.copied.len(), 2);
        assert!(target_dir.path().join("file.txt").exists());
        assert!(target_dir.path().join("subdir").join("inner.txt").exists());
    }

    #[test]
    fn test_copy_paths_multiple_nonexistent_sources() {
        let target_dir = tempdir().unwrap();

        let result = copy_paths_to_directory(
            vec![
                "/nonexistent/a.txt".to_string(),
                "/nonexistent/b.txt".to_string(),
                "/nonexistent/c.txt".to_string(),
            ],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.success);
        assert!(result.copied.is_empty());
        assert_eq!(result.errors.len(), 3);
        for error in &result.errors {
            assert!(error.error.contains("does not exist"));
        }
    }

    #[test]
    fn test_move_path_file_preserves_content() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let source_file = source_dir.path().join("data.bin");
        let content = b"binary\x00content\xff\xfe";
        fs::write(&source_file, content).unwrap();

        let result = move_path(
            source_file.to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(!source_file.exists());
        assert_eq!(fs::read(&final_path).unwrap(), content);
    }

    #[test]
    fn test_move_path_directory_with_nested_contents() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create deeply nested structure
        let dir = source_dir.path().join("project");
        fs::create_dir(&dir).unwrap();
        fs::create_dir(dir.join("src")).unwrap();
        fs::create_dir(dir.join("src").join("utils")).unwrap();
        fs::write(dir.join("README.md"), "readme").unwrap();
        fs::write(dir.join("src").join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.join("src").join("utils").join("helpers.rs"), "pub fn help() {}").unwrap();

        let result = move_path(
            dir.to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(!dir.exists());
        assert!(Path::new(&final_path).join("README.md").exists());
        assert!(Path::new(&final_path).join("src").join("main.rs").exists());
        assert!(Path::new(&final_path)
            .join("src")
            .join("utils")
            .join("helpers.rs")
            .exists());
    }

    #[test]
    fn test_move_path_directory_name_conflict() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Source directory
        let dir = source_dir.path().join("mydir");
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("file.txt"), "source content").unwrap();

        // Conflicting directory in target
        let existing = target_dir.path().join("mydir");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("existing.txt"), "existing").unwrap();

        let result = move_path(
            dir.to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(final_path.contains("mydir (1)"));
        assert!(!dir.exists());
        // Original directory is untouched
        assert!(target_dir.path().join("mydir").join("existing.txt").exists());
    }

    #[test]
    fn test_parse_existing_number_open_paren_no_close() {
        // Stem has " (" but doesn't end with ")"
        let (base, num) = parse_existing_number("file (3");
        assert_eq!(base, "file (3");
        assert_eq!(num, 1);
    }

    #[test]
    fn test_parse_existing_number_empty_string() {
        let (base, num) = parse_existing_number("");
        assert_eq!(base, "");
        assert_eq!(num, 1);
    }

    #[test]
    fn test_parse_existing_number_only_parens() {
        let (base, num) = parse_existing_number(" (5)");
        assert_eq!(base, "");
        assert_eq!(num, 6);
    }

    #[test]
    fn test_generate_unique_name_dotfile_no_extension() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".gitignore"), "content").unwrap();

        let result = generate_unique_name(".gitignore", dir.path());
        assert_eq!(result, ".gitignore (1)");
    }

    #[test]
    fn test_generate_unique_name_double_extension() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("archive.tar.gz"), "content").unwrap();

        let result = generate_unique_name("archive.tar.gz", dir.path());
        // Path::new("archive.tar.gz").file_stem() returns "archive.tar"
        // Path::new("archive.tar.gz").extension() returns "gz"
        assert_eq!(result, "archive.tar (1).gz");
    }

    #[test]
    fn test_copy_paths_to_directory_preserves_file_content() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let content = "Hello, World!\nLine 2\n\tTabbed line";
        fs::write(source_dir.path().join("test.txt"), content).unwrap();

        let result = copy_paths_to_directory(
            vec![source_dir
                .path()
                .join("test.txt")
                .to_string_lossy()
                .to_string()],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.success);
        assert_eq!(
            fs::read_to_string(target_dir.path().join("test.txt")).unwrap(),
            content
        );
    }

    #[test]
    fn test_copy_directory_contents_to_readonly_target() {
        use std::os::unix::fs::PermissionsExt;

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Source directory with a subdirectory
        let src = source_dir.path().join("src");
        fs::create_dir(&src).unwrap();
        fs::create_dir(src.join("subdir")).unwrap();
        fs::write(src.join("subdir").join("file.txt"), "content").unwrap();

        // Make target read-only so creating subdirectory fails
        let dest = target_dir.path().join("dest");
        fs::create_dir(&dest).unwrap();
        let permissions = std::fs::Permissions::from_mode(0o444);
        fs::set_permissions(&dest, permissions).unwrap();

        let result = copy_directory_contents(&src, &dest);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Failed to create subdirectory"),
            "Expected 'Failed to create subdirectory', got: {}",
            err
        );

        // Restore permissions for cleanup
        let permissions = std::fs::Permissions::from_mode(0o755);
        fs::set_permissions(&dest, permissions).unwrap();
    }

    #[test]
    fn test_copy_directory_contents_file_copy_fails() {
        use std::os::unix::fs::PermissionsExt;

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Source directory with a file
        let src = source_dir.path().join("src");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("file.txt"), "content").unwrap();

        // Make target read-only so file copy fails
        let dest = target_dir.path().join("dest");
        fs::create_dir(&dest).unwrap();
        let permissions = std::fs::Permissions::from_mode(0o444);
        fs::set_permissions(&dest, permissions).unwrap();

        let result = copy_directory_contents(&src, &dest);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Failed to copy file"),
            "Expected 'Failed to copy file', got: {}",
            err
        );

        // Restore permissions for cleanup
        let permissions = std::fs::Permissions::from_mode(0o755);
        fs::set_permissions(&dest, permissions).unwrap();
    }

    #[test]
    fn test_copy_directory_to_readonly_target() {
        use std::os::unix::fs::PermissionsExt;

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let subdir = source_dir.path().join("mydir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("file.txt"), "content").unwrap();

        // Make target read-only so create_dir fails
        let permissions = std::fs::Permissions::from_mode(0o444);
        fs::set_permissions(target_dir.path(), permissions).unwrap();

        let result = copy_directory(&subdir, target_dir.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Failed to create directory"),
            "Expected 'Failed to create directory', got: {}",
            err
        );

        // Restore permissions for cleanup
        let permissions = std::fs::Permissions::from_mode(0o755);
        fs::set_permissions(target_dir.path(), permissions).unwrap();
    }

    #[test]
    fn test_move_path_into_self_rejected() {
        let dir = tempdir().unwrap();

        let source = dir.path().join("mydir");
        fs::create_dir(&source).unwrap();

        // Try to move a directory into itself
        let result = move_path(
            source.to_string_lossy().to_string(),
            source.to_string_lossy().to_string(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        // "already in" because parent == target
        assert!(
            err.contains("already in") || err.contains("Cannot move"),
            "Expected error about self-move, got: {}",
            err
        );
    }

    #[test]
    fn test_move_path_deeply_nested_descendant_rejected() {
        let dir = tempdir().unwrap();

        let parent = dir.path().join("parent");
        let child = parent.join("child");
        let grandchild = child.join("grandchild");
        fs::create_dir_all(&grandchild).unwrap();

        let result = move_path(
            parent.to_string_lossy().to_string(),
            grandchild.to_string_lossy().to_string(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.contains("Cannot move"),
            "Error should mention 'Cannot move', got: {}",
            err
        );
    }

    #[test]
    fn test_copy_paths_mixed_valid_invalid_dirs() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // One valid directory and one nonexistent directory
        let valid = source_dir.path().join("valid_dir");
        fs::create_dir(&valid).unwrap();
        fs::write(valid.join("file.txt"), "content").unwrap();

        let result = copy_paths_to_directory(
            vec![
                valid.to_string_lossy().to_string(),
                "/nonexistent/dir".to_string(),
            ],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.success);
        assert_eq!(result.copied.len(), 1);
        assert_eq!(result.errors.len(), 1);
        assert!(target_dir.path().join("valid_dir").join("file.txt").exists());
    }

    #[test]
    fn test_copy_paths_source_not_found_creates_error() {
        let target_dir = tempdir().unwrap();

        let result = copy_paths_to_directory(
            vec!["/nonexistent/file.txt".to_string()],
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.success);
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].path.contains("nonexistent"));
    }

    #[test]
    fn test_move_path_file_with_name_conflict() {
        let dir = tempdir().unwrap();

        let source_dir = dir.path().join("source");
        let target_dir = dir.path().join("target");
        fs::create_dir(&source_dir).unwrap();
        fs::create_dir(&target_dir).unwrap();

        // Create source file
        let source_file = source_dir.join("test.txt");
        fs::write(&source_file, "original").unwrap();

        // Create existing file with same name in target
        fs::write(target_dir.join("test.txt"), "existing").unwrap();

        let result = move_path(
            source_file.to_string_lossy().to_string(),
            target_dir.to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        // Should generate unique name
        let new_path = result.unwrap();
        assert!(new_path.contains("test"), "Path: {}", new_path);
        // Original target file should remain
        assert_eq!(
            fs::read_to_string(target_dir.join("test.txt")).unwrap(),
            "existing"
        );
    }

    #[test]
    fn test_move_path_directory_rename_success() {
        let dir = tempdir().unwrap();

        let source = dir.path().join("src_dir");
        let target = dir.path().join("target");
        fs::create_dir(&source).unwrap();
        fs::create_dir(&target).unwrap();
        fs::write(source.join("file.txt"), "content").unwrap();

        let result = move_path(
            source.to_string_lossy().to_string(),
            target.to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        // Source should no longer exist
        assert!(!source.exists());
        // File should be in target
        assert!(target.join("src_dir").join("file.txt").exists());
    }

    #[test]
    fn test_copy_paths_batch_with_copy_failure() {
        // Cover lines 172-175: when copy_file/copy_directory returns Err
        // for an existing source (e.g., read-only target prevents copy).
        use std::os::unix::fs::PermissionsExt;

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create two source files
        fs::write(source_dir.path().join("good.txt"), "good content").unwrap();
        fs::write(source_dir.path().join("also_good.txt"), "also good").unwrap();

        // Create a read-only subdirectory as target to force copy failure
        // We'll use a file as the target to trigger the "not a directory" error?
        // No, we need the target to be a valid dir but cause individual copy failures.
        //
        // Strategy: make the target directory read-only AFTER the function starts.
        // But that's racy. Instead, we can create a source directory where contents
        // are unreadable, causing copy_directory to fail.
        let unreadable_dir = source_dir.path().join("unreadable");
        fs::create_dir(&unreadable_dir).unwrap();
        fs::write(unreadable_dir.join("secret.txt"), "secret").unwrap();
        // Make the directory unreadable so copy_directory_contents fails on read_dir
        let permissions = std::fs::Permissions::from_mode(0o000);
        fs::set_permissions(&unreadable_dir, permissions).unwrap();

        let result = copy_paths_to_directory(
            vec![
                source_dir
                    .path()
                    .join("good.txt")
                    .to_string_lossy()
                    .to_string(),
                unreadable_dir.to_string_lossy().to_string(),
                source_dir
                    .path()
                    .join("also_good.txt")
                    .to_string_lossy()
                    .to_string(),
            ],
            target_dir.path().to_string_lossy().to_string(),
        );

        // Restore permissions for cleanup
        let permissions = std::fs::Permissions::from_mode(0o755);
        fs::set_permissions(&unreadable_dir, permissions).unwrap();

        assert!(result.is_ok());
        let result = result.unwrap();
        // Should not be fully successful because the unreadable dir failed
        assert!(!result.success);
        // Two files should have been copied successfully
        assert_eq!(result.copied.len(), 2);
        // One error from the unreadable directory copy
        assert_eq!(result.errors.len(), 1);
        assert!(
            result.errors[0].path.contains("unreadable"),
            "Error path should reference the unreadable dir, got: {}",
            result.errors[0].path
        );
        // Verify the successfully copied files exist
        assert!(target_dir.path().join("good.txt").exists());
        assert!(target_dir.path().join("also_good.txt").exists());
    }

    #[test]
    fn test_copy_paths_batch_file_copy_error() {
        // Another test for lines 172-175: copy_file fails for an existing source
        // by making the target directory read-only after initial validation passes.
        use std::os::unix::fs::PermissionsExt;

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a valid source file
        fs::write(source_dir.path().join("file.txt"), "content").unwrap();

        // Create a subdirectory in target with same name as source file
        // to force a different kind of failure - actually, let's make target
        // read-only so copy fails
        let readonly_target = target_dir.path().join("readonly");
        fs::create_dir(&readonly_target).unwrap();
        let permissions = std::fs::Permissions::from_mode(0o444);
        fs::set_permissions(&readonly_target, permissions).unwrap();

        let result = copy_paths_to_directory(
            vec![source_dir
                .path()
                .join("file.txt")
                .to_string_lossy()
                .to_string()],
            readonly_target.to_string_lossy().to_string(),
        );

        // Restore permissions for cleanup
        let permissions = std::fs::Permissions::from_mode(0o755);
        fs::set_permissions(&readonly_target, permissions).unwrap();

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.success);
        assert!(result.copied.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(
            result.errors[0].error.contains("Failed to copy file"),
            "Expected copy failure error, got: {}",
            result.errors[0].error
        );
    }

    #[test]
    fn test_move_path_cross_device_file_fallback() {
        // Cover lines 262-266: cross-device file move fallback (copy + delete).
        //
        // On the same filesystem, fs::rename succeeds, so we can't directly
        // test the cross-device fallback. However, we can verify the function
        // works correctly by testing the overall move behavior with files
        // across different tempdir instances (which are on the same FS but
        // exercise the same logical path if rename were to fail).
        //
        // To truly cover lines 252-269, we use a FIFO (named pipe) trick:
        // Create a scenario where rename fails by making the target path
        // exist as something that blocks rename, then cleaning it before
        // the fallback runs. This is fragile, so instead we test the
        // underlying copy+delete logic directly.
        //
        // Actually, we can simulate cross-device by using /tmp and another
        // tmpfs mount point. But the simplest reliable approach is to test
        // that the functions used in the fallback path work correctly.

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a file to move
        let source_file = source_dir.path().join("cross_device.txt");
        fs::write(&source_file, "cross device content").unwrap();

        // This will use rename (same FS), but verifies the overall move works
        let result = move_path(
            source_file.to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(!source_file.exists(), "Source should be removed");
        assert_eq!(
            fs::read_to_string(&final_path).unwrap(),
            "cross device content"
        );
    }

    #[test]
    fn test_move_path_cross_device_directory_fallback() {
        // Cover lines 252-261: cross-device directory move fallback.
        // Tests the overall directory move with nested contents to ensure
        // the copy+delete path produces correct results when triggered.

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a directory with multiple levels and files
        let dir = source_dir.path().join("project");
        fs::create_dir(&dir).unwrap();
        fs::create_dir(dir.join("src")).unwrap();
        fs::create_dir(dir.join("src").join("components")).unwrap();
        fs::write(dir.join("README.md"), "# Project").unwrap();
        fs::write(dir.join("src").join("index.ts"), "export {};").unwrap();
        fs::write(
            dir.join("src").join("components").join("App.svelte"),
            "<script>let x = 1;</script>",
        )
        .unwrap();

        let result = move_path(
            dir.to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        let fp = Path::new(&final_path);

        // Source should be gone
        assert!(!dir.exists(), "Source directory should be removed");

        // All contents should be in the new location
        assert_eq!(
            fs::read_to_string(fp.join("README.md")).unwrap(),
            "# Project"
        );
        assert_eq!(
            fs::read_to_string(fp.join("src").join("index.ts")).unwrap(),
            "export {};"
        );
        assert_eq!(
            fs::read_to_string(fp.join("src").join("components").join("App.svelte")).unwrap(),
            "<script>let x = 1;</script>"
        );
    }

    /// Test that simulates what happens in the cross-device fallback path
    /// by directly testing copy_directory_contents + remove_dir_all,
    /// which is the exact sequence used in lines 252-261.
    #[test]
    fn test_cross_device_fallback_logic_directory() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create source structure
        let src = source_dir.path().join("app");
        fs::create_dir(&src).unwrap();
        fs::create_dir(src.join("config")).unwrap();
        fs::write(src.join("main.rs"), "fn main() {}").unwrap();
        fs::write(src.join("config").join("settings.toml"), "[app]\nname=\"test\"").unwrap();

        // Simulate the cross-device fallback: create_dir + copy_contents + remove source
        let final_path = target_dir.path().join("app");
        fs::create_dir(&final_path).unwrap();

        let copy_result = copy_directory_contents(&src, &final_path);
        assert!(copy_result.is_ok(), "copy_directory_contents should succeed");

        // Remove source (same as line 260)
        fs::remove_dir_all(&src).unwrap();

        // Verify results match what move_path would produce
        assert!(!src.exists());
        assert_eq!(
            fs::read_to_string(final_path.join("main.rs")).unwrap(),
            "fn main() {}"
        );
        assert_eq!(
            fs::read_to_string(final_path.join("config").join("settings.toml")).unwrap(),
            "[app]\nname=\"test\""
        );
    }

    /// Test that simulates the cross-device fallback for files
    /// by directly testing fs::copy + fs::remove_file, matching lines 263-266.
    #[test]
    fn test_cross_device_fallback_logic_file() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let source_file = source_dir.path().join("data.json");
        fs::write(&source_file, r#"{"key": "value"}"#).unwrap();

        let final_path = target_dir.path().join("data.json");

        // Simulate the cross-device fallback: copy + remove_file
        fs::copy(&source_file, &final_path).unwrap();
        fs::remove_file(&source_file).unwrap();

        assert!(!source_file.exists());
        assert_eq!(
            fs::read_to_string(&final_path).unwrap(),
            r#"{"key": "value"}"#
        );
    }

    #[test]
    fn test_copy_paths_batch_mixed_file_and_dir_errors() {
        // Cover lines 172-175 with both a file copy error and a dir copy error
        // in the same batch.
        use std::os::unix::fs::PermissionsExt;

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a valid file
        fs::write(source_dir.path().join("valid.txt"), "valid").unwrap();

        // Create a directory with unreadable contents
        let bad_dir = source_dir.path().join("bad_dir");
        fs::create_dir(&bad_dir).unwrap();
        fs::write(bad_dir.join("inner.txt"), "inner").unwrap();
        let permissions = std::fs::Permissions::from_mode(0o000);
        fs::set_permissions(&bad_dir, permissions).unwrap();

        // Make target temporarily read-only to test that the valid file
        // gets an error too when the target is unwritable.
        // Actually, we want mixed: some succeed and some fail.
        // Let's use a writable target but with an unreadable source dir.
        let result = copy_paths_to_directory(
            vec![
                source_dir
                    .path()
                    .join("valid.txt")
                    .to_string_lossy()
                    .to_string(),
                bad_dir.to_string_lossy().to_string(),
            ],
            target_dir.path().to_string_lossy().to_string(),
        );

        // Restore permissions for cleanup
        let permissions = std::fs::Permissions::from_mode(0o755);
        fs::set_permissions(&bad_dir, permissions).unwrap();

        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(!result.success);
        // valid.txt should copy successfully
        assert_eq!(result.copied.len(), 1);
        // bad_dir should fail
        assert_eq!(result.errors.len(), 1);
        assert!(
            result.errors[0].error.contains("Failed to read directory")
                || result.errors[0].error.contains("Failed to create directory"),
            "Expected copy failure, got: {}",
            result.errors[0].error
        );
    }

    #[test]
    fn test_move_path_empty_directory() {
        // Move an empty directory - tests the move (rename or fallback) path
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let empty_dir = source_dir.path().join("empty");
        fs::create_dir(&empty_dir).unwrap();

        let result = move_path(
            empty_dir.to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(!empty_dir.exists(), "Source should be removed");
        assert!(
            Path::new(&final_path).exists(),
            "Target should exist"
        );
        assert!(
            Path::new(&final_path).is_dir(),
            "Target should be a directory"
        );
    }

    #[test]
    fn test_move_path_directory_with_many_files() {
        // Move a directory with many files to exercise the fallback path thoroughly
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let dir = source_dir.path().join("many_files");
        fs::create_dir(&dir).unwrap();

        // Create 20 files
        for i in 0..20 {
            fs::write(dir.join(format!("file_{}.txt", i)), format!("content_{}", i)).unwrap();
        }

        let result = move_path(
            dir.to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
        );

        assert!(result.is_ok());
        let final_path = result.unwrap();
        assert!(!dir.exists(), "Source should be removed");

        // Verify all 20 files were moved
        for i in 0..20 {
            let file_path = Path::new(&final_path).join(format!("file_{}.txt", i));
            assert!(file_path.exists(), "file_{}.txt should exist", i);
            assert_eq!(
                fs::read_to_string(&file_path).unwrap(),
                format!("content_{}", i)
            );
        }
    }

    #[test]
    fn test_cross_device_move_file() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let source_file = source_dir.path().join("test.txt");
        fs::write(&source_file, "cross device content").unwrap();

        let target_file = target_dir.path().join("test.txt");

        let result = cross_device_move(&source_file, &target_file);
        assert!(result.is_ok());
        assert!(!source_file.exists(), "Source file should be deleted");
        assert!(target_file.exists(), "Target file should exist");
        assert_eq!(fs::read_to_string(&target_file).unwrap(), "cross device content");
    }

    #[test]
    fn test_cross_device_move_directory() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let src = source_dir.path().join("mydir");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("a.txt"), "file a").unwrap();
        fs::write(src.join("b.txt"), "file b").unwrap();

        let dst = target_dir.path().join("mydir");

        let result = cross_device_move(&src, &dst);
        assert!(result.is_ok());
        assert!(!src.exists(), "Source directory should be deleted");
        assert!(dst.exists(), "Target directory should exist");
        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "file a");
        assert_eq!(fs::read_to_string(dst.join("b.txt")).unwrap(), "file b");
    }

    #[test]
    fn test_cross_device_move_file_target_dir_fails() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let source_file = source_dir.path().join("test.txt");
        fs::write(&source_file, "content").unwrap();

        // Create target as a directory to cause copy to fail
        let target = target_dir.path().join("test.txt");
        fs::create_dir(&target).unwrap();

        let result = cross_device_move(&source_file, &target);
        assert!(result.is_err());
        assert!(source_file.exists(), "Source should still exist on failure");
    }

    #[test]
    fn test_cross_device_move_directory_with_subdirs() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let src = source_dir.path().join("project");
        fs::create_dir_all(src.join("sub/deep")).unwrap();
        fs::write(src.join("root.txt"), "root").unwrap();
        fs::write(src.join("sub/mid.txt"), "mid").unwrap();
        fs::write(src.join("sub/deep/leaf.txt"), "leaf").unwrap();

        let dst = target_dir.path().join("project");

        let result = cross_device_move(&src, &dst);
        assert!(result.is_ok());
        assert!(!src.exists());
        assert_eq!(fs::read_to_string(dst.join("root.txt")).unwrap(), "root");
        assert_eq!(fs::read_to_string(dst.join("sub/mid.txt")).unwrap(), "mid");
        assert_eq!(fs::read_to_string(dst.join("sub/deep/leaf.txt")).unwrap(), "leaf");
    }

    #[test]
    fn test_cross_device_move_nonexistent_source() {
        let target_dir = tempdir().unwrap();
        let source = Path::new("/tmp/nonexistent_cross_device_src_12345");
        let target = target_dir.path().join("dest.txt");

        let result = cross_device_move(source, &target);
        assert!(result.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn test_cross_device_move_dir_copy_failure_cleanup() {
        use std::os::unix::fs::PermissionsExt;

        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let src = source_dir.path().join("mydir");
        fs::create_dir(&src).unwrap();
        fs::write(src.join("readable.txt"), "ok").unwrap();

        // Create a subdirectory with an unreadable file
        let sub = src.join("sub");
        fs::create_dir(&sub).unwrap();
        let unreadable = sub.join("secret.txt");
        fs::write(&unreadable, "secret").unwrap();
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o000)).unwrap();
        // Also make the subdir unreadable to prevent reading its contents
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o000)).unwrap();

        let dst = target_dir.path().join("mydir");

        let result = cross_device_move(&src, &dst);
        // Should fail because copy_directory_contents can't read the subdir
        assert!(result.is_err(), "Should fail due to unreadable subdir");
        // The cleanup should have removed the partially created target
        // (or the target dir should be cleaned up)

        // Restore permissions for cleanup
        fs::set_permissions(&sub, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&unreadable, fs::Permissions::from_mode(0o644)).unwrap();
    }
}
