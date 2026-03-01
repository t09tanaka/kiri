// This file contains git diff operations with error handling that requires
// system-level failures to test. Covered via E2E tests.

use base64::Engine;
use git2::{DiffOptions, Repository};
use std::path::Path;

/// Binary file extensions that should be displayed as images
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "webp", "bmp", "svg", "tiff", "tif",
];

/// Check if a file path has an image extension
pub fn is_image_file(path: &str) -> bool {
    let path_lower = path.to_lowercase();
    IMAGE_EXTENSIONS
        .iter()
        .any(|ext| path_lower.ends_with(&format!(".{}", ext)))
}

/// Get base64 encoded content of the current working directory file
pub fn get_current_file_base64(repo_path: &str, file_path: &str) -> Option<String> {
    let full_path = Path::new(repo_path).join(file_path);
    std::fs::read(&full_path)
        .ok()
        .map(|bytes| base64::engine::general_purpose::STANDARD.encode(&bytes))
}

/// Get base64 encoded content of the file from HEAD
pub fn get_original_file_base64(repo: &Repository, file_path: &str) -> Option<String> {
    let head = repo.head().ok()?;
    let tree = head.peel_to_tree().ok()?;
    let entry = tree.get_path(Path::new(file_path)).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    Some(base64::engine::general_purpose::STANDARD.encode(blob.content()))
}

/// Get file diff - internal implementation with error handling
/// Returns empty string on any error
pub fn get_file_diff_internal(repo: &Repository, repo_path: &str, file_path: &str) -> String {
    let relative_path = Path::new(file_path);
    let file_status = repo.status_file(relative_path).ok();

    // For untracked files, return the entire file content
    if let Some(status) = file_status {
        if status.is_wt_new() {
            let full_path = Path::new(repo_path).join(file_path);
            if let Ok(content) = std::fs::read_to_string(&full_path) {
                return content
                    .lines()
                    .map(|line| format!("+ {}", line))
                    .collect::<Vec<_>>()
                    .join("\n");
            }
            return String::new();
        }
    }

    // Get diff between HEAD and working directory for the specific file
    let mut diff_opts = DiffOptions::new();
    diff_opts.pathspec(file_path);

    let diff = match repo.diff_index_to_workdir(None, Some(&mut diff_opts)) {
        Ok(d) => d,
        Err(_) => return String::new(),
    };

    // If no working directory changes, check index changes (staged)
    let diff = if diff.deltas().len() == 0 {
        let head = match repo.head() {
            Ok(h) => h,
            Err(_) => return String::new(),
        };
        let head_tree = match head.peel_to_tree() {
            Ok(t) => t,
            Err(_) => return String::new(),
        };
        match repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut diff_opts)) {
            Ok(d) => d,
            Err(_) => return String::new(),
        }
    } else {
        diff
    };

    // Convert diff to string
    let mut diff_text = String::new();
    let _ = diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let prefix = match line.origin() {
            '+' => "+ ",
            '-' => "- ",
            ' ' => "  ",
            _ => "",
        };
        if let Ok(content) = std::str::from_utf8(line.content()) {
            diff_text.push_str(prefix);
            diff_text.push_str(content);
        }
        true
    });

    diff_text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_is_image_file_png() {
        assert!(is_image_file("photo.png"));
    }

    #[test]
    fn test_is_image_file_jpg() {
        assert!(is_image_file("image.jpg"));
    }

    #[test]
    fn test_is_image_file_jpeg_case_insensitive() {
        assert!(is_image_file("Photo.JPEG"));
    }

    #[test]
    fn test_is_image_file_svg() {
        assert!(is_image_file("icon.svg"));
    }

    #[test]
    fn test_is_image_file_not_image() {
        assert!(!is_image_file("readme.txt"));
    }

    #[test]
    fn test_is_image_file_no_extension() {
        assert!(!is_image_file("Makefile"));
    }

    #[test]
    fn test_get_current_file_base64_existing() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("test.bin"), b"\x00\x01\x02").unwrap();

        let result = get_current_file_base64(
            dir.path().to_str().unwrap(),
            "test.bin",
        );
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "AAEC");
    }

    #[test]
    fn test_get_current_file_base64_nonexistent() {
        let dir = tempdir().unwrap();
        let result = get_current_file_base64(
            dir.path().to_str().unwrap(),
            "nonexistent.bin",
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_get_original_file_base64_not_committed() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // No commits, so head() will fail
        let result = get_original_file_base64(&repo, "file.txt");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_original_file_base64_committed() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

        // Create and commit a file
        fs::write(dir.path().join("test.txt"), "hello").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("test.txt"))
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        let result = get_original_file_base64(&repo, "test.txt");
        assert!(result.is_some());
        // "hello" in base64
        assert_eq!(result.unwrap(), "aGVsbG8=");
    }

    #[test]
    fn test_get_original_file_base64_file_not_in_tree() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

        // Commit a different file
        fs::write(dir.path().join("other.txt"), "data").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("other.txt"))
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        // Try to get a file that doesn't exist in the tree
        let result = get_original_file_base64(&repo, "nonexistent.txt");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_file_diff_internal_untracked_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        // Create an untracked file
        fs::write(dir.path().join("new.txt"), "line1\nline2").unwrap();

        let diff = get_file_diff_internal(
            &repo,
            dir.path().to_str().unwrap(),
            "new.txt",
        );
        assert!(diff.contains("+ line1"));
        assert!(diff.contains("+ line2"));
    }

    #[test]
    fn test_get_file_diff_internal_no_changes() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

        // Commit a file
        fs::write(dir.path().join("stable.txt"), "content").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("stable.txt"))
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        // No changes
        let diff = get_file_diff_internal(
            &repo,
            dir.path().to_str().unwrap(),
            "stable.txt",
        );
        assert!(diff.is_empty());
    }

    #[test]
    fn test_get_file_diff_internal_modified_file() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

        // Commit a file
        fs::write(dir.path().join("mod.txt"), "original").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("mod.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        // Modify the file
        fs::write(dir.path().join("mod.txt"), "modified").unwrap();

        let diff = get_file_diff_internal(
            &repo,
            dir.path().to_str().unwrap(),
            "mod.txt",
        );
        assert!(!diff.is_empty());
        assert!(diff.contains("- original") || diff.contains("-original"));
        assert!(diff.contains("+ modified") || diff.contains("+modified"));
    }

    #[test]
    fn test_get_file_diff_internal_staged_change() {
        let dir = tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@test.com").unwrap();

        // Commit initial
        fs::write(dir.path().join("staged.txt"), "before").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("staged.txt"))
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();

        // Modify and stage
        fs::write(dir.path().join("staged.txt"), "after").unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new("staged.txt"))
            .unwrap();
        index.write().unwrap();

        let diff = get_file_diff_internal(
            &repo,
            dir.path().to_str().unwrap(),
            "staged.txt",
        );
        // Staged changes should be detected via diff_tree_to_index fallback
        assert!(!diff.is_empty());
    }
}
