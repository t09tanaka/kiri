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
