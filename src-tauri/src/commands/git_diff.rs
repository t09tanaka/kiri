// This file contains git diff operations with error handling that requires
// system-level failures to test. Covered via E2E tests.

use git2::{DiffOptions, Repository};
use std::path::Path;

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
