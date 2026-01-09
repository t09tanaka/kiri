use serde::Serialize;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct FileSearchResult {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub score: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentMatch {
    pub line: usize,
    pub content: String,
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContentSearchResult {
    pub path: String,
    pub name: String,
    pub matches: Vec<ContentMatch>,
}

fn fuzzy_match(query: &str, target: &str) -> Option<i32> {
    let query_lower = query.to_lowercase();
    let target_lower = target.to_lowercase();

    let mut score = 0i32;
    let mut query_idx = 0;
    let query_chars: Vec<char> = query_lower.chars().collect();
    let target_chars: Vec<char> = target_lower.chars().collect();

    if query_chars.is_empty() {
        return Some(0);
    }

    let mut prev_match_idx: Option<usize> = None;

    for (i, tc) in target_chars.iter().enumerate() {
        if query_idx < query_chars.len() && *tc == query_chars[query_idx] {
            score += 10;

            if let Some(prev) = prev_match_idx {
                if i == prev + 1 {
                    score += 5;
                }
            }

            if i == 0 || target_chars.get(i.saturating_sub(1)).map_or(false, |c| {
                *c == '/' || *c == '\\' || *c == '_' || *c == '-' || *c == '.'
            }) {
                score += 10;
            }

            prev_match_idx = Some(i);
            query_idx += 1;
        }
    }

    if query_idx == query_chars.len() {
        Some(score)
    } else {
        None
    }
}

fn collect_files(
    dir: &Path,
    query: &str,
    results: &mut Vec<FileSearchResult>,
    max_results: usize,
    ignore_hidden: bool,
) {
    if results.len() >= max_results {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if results.len() >= max_results {
            break;
        }

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if ignore_hidden && name.starts_with('.') {
            continue;
        }

        let is_dir = path.is_dir();

        if is_dir {
            let dir_name = name.to_lowercase();
            if dir_name != "node_modules"
                && dir_name != "target"
                && dir_name != ".git"
                && dir_name != "dist"
                && dir_name != "build"
            {
                collect_files(&path, query, results, max_results, ignore_hidden);
            }
        } else if let Some(score) = fuzzy_match(query, &name) {
            results.push(FileSearchResult {
                path: path.to_string_lossy().to_string(),
                name,
                is_dir,
                score,
            });
        }
    }
}

#[tauri::command]
pub fn search_files(
    root_path: String,
    query: String,
    max_results: usize,
) -> Result<Vec<FileSearchResult>, String> {
    let root = Path::new(&root_path);

    if !root.exists() {
        return Err("Path does not exist".to_string());
    }

    let mut results = Vec::new();
    collect_files(root, &query, &mut results, max_results, true);

    results.sort_by(|a, b| b.score.cmp(&a.score));

    Ok(results)
}

fn search_file_content(
    file_path: &Path,
    query: &str,
    max_matches_per_file: usize,
) -> Option<ContentSearchResult> {
    let file = fs::File::open(file_path).ok()?;
    let reader = BufReader::new(file);
    let query_lower = query.to_lowercase();

    let mut matches = Vec::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        if matches.len() >= max_matches_per_file {
            break;
        }

        if let Ok(line) = line_result {
            let line_lower = line.to_lowercase();

            if let Some(start) = line_lower.find(&query_lower) {
                matches.push(ContentMatch {
                    line: line_num + 1,
                    content: line.clone(),
                    start,
                    end: start + query.len(),
                });
            }
        }
    }

    if matches.is_empty() {
        None
    } else {
        Some(ContentSearchResult {
            path: file_path.to_string_lossy().to_string(),
            name: file_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            matches,
        })
    }
}

fn collect_content_matches(
    dir: &Path,
    query: &str,
    results: &mut Vec<ContentSearchResult>,
    max_results: usize,
    max_matches_per_file: usize,
    ignore_hidden: bool,
) {
    if results.len() >= max_results {
        return;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        if results.len() >= max_results {
            break;
        }

        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if ignore_hidden && name.starts_with('.') {
            continue;
        }

        if path.is_file() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            let searchable = matches!(
                ext,
                "rs" | "ts" | "tsx" | "js" | "jsx" | "svelte" | "html" | "css" | "scss" | "json"
                    | "md" | "toml" | "yaml" | "yml" | "txt"
            );

            if searchable {
                if let Some(result) = search_file_content(&path, query, max_matches_per_file) {
                    results.push(result);
                }
            }
        } else if path.is_dir() {
            let dir_name = name.to_lowercase();
            if dir_name != "node_modules"
                && dir_name != "target"
                && dir_name != ".git"
                && dir_name != "dist"
                && dir_name != "build"
            {
                collect_content_matches(
                    &path,
                    query,
                    results,
                    max_results,
                    max_matches_per_file,
                    ignore_hidden,
                );
            }
        }
    }
}

#[tauri::command]
pub fn search_content(
    root_path: String,
    query: String,
    max_results: usize,
) -> Result<Vec<ContentSearchResult>, String> {
    if query.len() < 2 {
        return Ok(Vec::new());
    }

    let root = Path::new(&root_path);

    if !root.exists() {
        return Err("Path does not exist".to_string());
    }

    let mut results = Vec::new();
    collect_content_matches(root, &query, &mut results, max_results, 10, true);

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("ft", "FileTree.svelte").is_some());
        assert!(fuzzy_match("main", "main.rs").is_some());
        assert!(fuzzy_match("xyz", "main.rs").is_none());
    }

    #[test]
    fn test_fuzzy_match_empty_query() {
        assert!(fuzzy_match("", "anything").is_some());
    }
}
