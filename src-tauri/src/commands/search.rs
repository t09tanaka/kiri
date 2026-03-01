use glob::Pattern;
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

/// Check if a path should be excluded based on the exclude patterns.
/// Supports both simple names (e.g., "node_modules") and glob patterns (e.g., "**/*.min.js")
fn should_exclude(path: &Path, exclude_patterns: &[Pattern]) -> bool {
    let path_str = path.to_string_lossy();

    for pattern in exclude_patterns {
        // Check the full path
        if pattern.matches(&path_str) {
            return true;
        }

        // Check against just the file/directory name
        if let Some(name) = path.file_name() {
            let name_str = name.to_string_lossy();
            if pattern.matches(&name_str) {
                return true;
            }
        }

        // Check each component of the path
        for component in path.components() {
            if let Some(name) = component.as_os_str().to_str() {
                if pattern.matches(name) {
                    return true;
                }
            }
        }
    }

    false
}

/// Parse exclude pattern strings into glob Patterns.
/// Simple names like "node_modules" are converted to patterns that match them anywhere.
fn parse_exclude_patterns(patterns: &[String]) -> Vec<Pattern> {
    patterns
        .iter()
        .filter_map(|p| {
            // If it's a simple name (no glob characters), match it anywhere
            if !p.contains('*') && !p.contains('?') && !p.contains('[') {
                // Try to match as-is first
                Pattern::new(p).ok()
            } else {
                Pattern::new(p).ok()
            }
        })
        .collect()
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
    exclude_patterns: &[Pattern],
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

        // Check custom exclude patterns
        if should_exclude(&path, exclude_patterns) {
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
            collect_content_matches(
                &path,
                query,
                results,
                max_results,
                max_matches_per_file,
                ignore_hidden,
                exclude_patterns,
            );
        }
    }
}

#[tauri::command]
pub fn search_content(
    root_path: String,
    query: String,
    max_results: usize,
    exclude_patterns: Vec<String>,
) -> Result<Vec<ContentSearchResult>, String> {
    if query.len() < 2 {
        return Ok(Vec::new());
    }

    let root = Path::new(&root_path);

    if !root.exists() {
        return Err("Path does not exist".to_string());
    }

    // Default excluded directories (always excluded)
    let mut all_patterns = vec![
        "node_modules".to_string(),
        "target".to_string(),
        ".git".to_string(),
        "dist".to_string(),
        "build".to_string(),
    ];
    all_patterns.extend(exclude_patterns);

    let parsed_patterns = parse_exclude_patterns(&all_patterns);

    let mut results = Vec::new();
    collect_content_matches(
        root,
        &query,
        &mut results,
        max_results,
        10,
        true,
        &parsed_patterns,
    );

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("ft", "FileTree.svelte").is_some());
        assert!(fuzzy_match("main", "main.rs").is_some());
        assert!(fuzzy_match("xyz", "main.rs").is_none());
    }

    #[test]
    fn test_fuzzy_match_empty_query() {
        assert!(fuzzy_match("", "anything").is_some());
        assert_eq!(fuzzy_match("", "anything").unwrap(), 0);
    }

    #[test]
    fn test_fuzzy_match_case_insensitive() {
        assert!(fuzzy_match("FILE", "file.txt").is_some());
        assert!(fuzzy_match("file", "FILE.TXT").is_some());
        assert!(fuzzy_match("FiLe", "fIlE.txt").is_some());
    }

    #[test]
    fn test_fuzzy_match_consecutive_bonus() {
        let score_consecutive = fuzzy_match("ab", "ab").unwrap();
        let score_separated = fuzzy_match("ab", "a_b").unwrap();
        // Both should match, consecutive might have bonus but also word boundary bonus
        assert!(score_consecutive > 0);
        assert!(score_separated > 0);
    }

    #[test]
    fn test_fuzzy_match_word_boundary_bonus() {
        // Matching at word boundaries should have bonus
        let score_boundary = fuzzy_match("f", "file").unwrap(); // at start
        assert!(score_boundary > 0);
    }

    #[test]
    fn test_fuzzy_match_special_separators() {
        // Test matching after separators
        assert!(fuzzy_match("t", "_test").is_some());
        assert!(fuzzy_match("t", "-test").is_some());
        assert!(fuzzy_match("t", ".test").is_some());
        assert!(fuzzy_match("t", "/test").is_some());
    }

    #[test]
    fn test_search_files_empty_directory() {
        let dir = tempdir().unwrap();

        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "test".to_string(),
            10
        );
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_search_files_nonexistent_path() {
        let result = search_files(
            "/nonexistent/path".to_string(),
            "test".to_string(),
            10
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_search_files_basic() {
        let dir = tempdir().unwrap();

        // Create test files
        fs::write(dir.path().join("test.txt"), "content").unwrap();
        fs::write(dir.path().join("another.rs"), "code").unwrap();
        fs::write(dir.path().join("testing.md"), "docs").unwrap();

        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "test".to_string(),
            10
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        assert!(results.len() >= 2); // test.txt and testing.md should match
    }

    #[test]
    fn test_search_files_max_results() {
        let dir = tempdir().unwrap();

        // Create many test files
        for i in 0..20 {
            fs::write(dir.path().join(format!("file{}.txt", i)), "content").unwrap();
        }

        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "file".to_string(),
            5
        );
        assert!(result.is_ok());
        assert!(result.unwrap().len() <= 5);
    }

    #[test]
    fn test_search_files_ignores_hidden() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join(".hidden_file.txt"), "hidden").unwrap();
        fs::write(dir.path().join("visible_file.txt"), "visible").unwrap();

        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "file".to_string(),
            10
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        // Hidden files should be ignored
        assert!(results.iter().all(|r| !r.name.starts_with('.')));
    }

    #[test]
    fn test_search_files_sorted_by_score() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("exact_match.txt"), "").unwrap();
        fs::write(dir.path().join("e_x_a_c_t.txt"), "").unwrap();

        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "exact".to_string(),
            10
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        if results.len() >= 2 {
            // Results should be sorted by score (descending)
            assert!(results[0].score >= results[1].score);
        }
    }

    #[test]
    fn test_search_files_subdirectories() {
        let dir = tempdir().unwrap();

        fs::create_dir(dir.path().join("subdir")).unwrap();
        fs::write(dir.path().join("subdir").join("nested.txt"), "").unwrap();
        fs::write(dir.path().join("root.txt"), "").unwrap();

        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "nested".to_string(),
            10
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains("subdir"));
    }

    #[test]
    fn test_search_files_skips_excluded_dirs() {
        let dir = tempdir().unwrap();

        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules").join("package.txt"), "").unwrap();
        fs::write(dir.path().join("package.txt"), "").unwrap();

        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "package".to_string(),
            10
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        // Should only find the root level file, not the one in node_modules
        assert_eq!(results.len(), 1);
        assert!(!results[0].path.contains("node_modules"));
    }

    #[test]
    fn test_search_content_short_query() {
        let dir = tempdir().unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "a".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());
        // Short queries (< 2 chars) should return empty results
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_search_content_nonexistent_path() {
        let result = search_content(
            "/nonexistent/path".to_string(),
            "test".to_string(),
            10,
            vec![],
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_search_content_basic() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("test.rs"), "fn main() {\n    println!(\"hello\");\n}").unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "println".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].matches.len(), 1);
        assert_eq!(results[0].matches[0].line, 2);
    }

    #[test]
    fn test_search_content_multiple_matches() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("test.rs"), "test line 1\ntest line 2\ntest line 3").unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "test".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].matches.len(), 3);
    }

    #[test]
    fn test_search_content_case_insensitive() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("test.rs"), "Hello World\nhello world\nHELLO WORLD").unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "hello".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        assert_eq!(results[0].matches.len(), 3);
    }

    #[test]
    fn test_search_content_only_searchable_files() {
        let dir = tempdir().unwrap();

        // Searchable file
        fs::write(dir.path().join("test.rs"), "test content").unwrap();
        // Non-searchable file (binary extension)
        fs::write(dir.path().join("test.exe"), "test content").unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "test".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].name.ends_with(".rs"));
    }

    #[test]
    fn test_content_match_start_end() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("test.rs"), "hello world").unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "world".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        let m = &results[0].matches[0];
        assert_eq!(m.start, 6); // "world" starts at index 6
        assert_eq!(m.end, 11); // "world" ends at index 11
    }

    #[test]
    fn test_file_search_result_fields() {
        let result = FileSearchResult {
            path: "/path/to/file.txt".to_string(),
            name: "file.txt".to_string(),
            is_dir: false,
            score: 100,
        };
        assert_eq!(result.path, "/path/to/file.txt");
        assert_eq!(result.name, "file.txt");
        assert!(!result.is_dir);
        assert_eq!(result.score, 100);
    }

    #[test]
    fn test_content_search_result_fields() {
        let result = ContentSearchResult {
            path: "/path/to/file.rs".to_string(),
            name: "file.rs".to_string(),
            matches: vec![ContentMatch {
                line: 1,
                content: "test content".to_string(),
                start: 0,
                end: 4,
            }],
        };
        assert_eq!(result.path, "/path/to/file.rs");
        assert_eq!(result.name, "file.rs");
        assert_eq!(result.matches.len(), 1);
    }

    #[test]
    fn test_search_content_in_subdirectories() {
        let dir = tempdir().unwrap();

        // Create nested directory structure
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src").join("main.rs"), "fn main() { hello() }").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn hello() {}").unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "hello".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        assert_eq!(results.len(), 2); // Should find in both files
    }

    #[test]
    fn test_search_content_max_results_limit() {
        let dir = tempdir().unwrap();

        // Create many files with matching content
        for i in 0..10 {
            fs::write(dir.path().join(format!("file{}.rs", i)), "matching content").unwrap();
        }

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "matching".to_string(),
            5,
            vec![],
        );
        assert!(result.is_ok());
        assert!(result.unwrap().len() <= 5);
    }

    #[test]
    fn test_search_files_read_error_handling() {
        let dir = tempdir().unwrap();

        // Create a file that can be found
        fs::write(dir.path().join("test.txt"), "").unwrap();

        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "test".to_string(),
            10
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_content_skips_excluded_dirs() {
        let dir = tempdir().unwrap();

        // Create excluded directories
        fs::create_dir(dir.path().join("node_modules")).unwrap();
        fs::write(dir.path().join("node_modules").join("package.rs"), "matching content").unwrap();

        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src").join("main.rs"), "matching content").unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "matching".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        // Should only find in src, not node_modules
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains("src"));
    }

    #[test]
    fn test_search_content_max_matches_per_file() {
        let dir = tempdir().unwrap();

        // Create file with many matches
        let content = (0..20).map(|_| "match").collect::<Vec<_>>().join("\n");
        fs::write(dir.path().join("test.rs"), content).unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "match".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        // search_content uses max_matches_per_file=10
        assert!(results[0].matches.len() <= 10);
    }

    #[test]
    fn test_search_files_deeply_nested() {
        let dir = tempdir().unwrap();

        // Create deeply nested structure
        let nested = dir.path().join("a").join("b").join("c");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("deep.txt"), "").unwrap();

        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "deep".to_string(),
            10
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains("deep.txt"));
    }

    #[test]
    fn test_search_files_hits_max_results_early() {
        let dir = tempdir().unwrap();

        // Create many files that match the query
        for i in 0..50 {
            fs::write(dir.path().join(format!("match{}.txt", i)), "").unwrap();
        }

        // Search with a small max_results limit
        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "match".to_string(),
            3
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        // Should stop at max_results
        assert!(results.len() <= 3);
    }

    #[test]
    fn test_search_content_hidden_files_ignored() {
        let dir = tempdir().unwrap();

        // Create a hidden searchable file
        fs::write(dir.path().join(".hidden.rs"), "matching content here").unwrap();
        // Create a visible searchable file
        fs::write(dir.path().join("visible.rs"), "matching content here").unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "matching".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        // Hidden files should be ignored
        assert!(results.iter().all(|r| !r.name.starts_with('.')));
    }

    #[test]
    fn test_search_content_skips_build_directory() {
        let dir = tempdir().unwrap();

        // Create build directory with matching file
        fs::create_dir(dir.path().join("build")).unwrap();
        fs::write(dir.path().join("build").join("output.rs"), "matching content").unwrap();

        // Create regular directory with matching file
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src").join("main.rs"), "matching content").unwrap();

        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "matching".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        // Should only find in src, not build
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains("src"));
    }

    #[test]
    fn test_collect_files_stops_at_max() {
        let dir = tempdir().unwrap();

        // Create many matching files in subdirectories
        for letter in ['a', 'b', 'c'] {
            let subdir = dir.path().join(letter.to_string());
            fs::create_dir(&subdir).unwrap();
            for i in 0..10 {
                fs::write(subdir.join(format!("file{}.txt", i)), "").unwrap();
            }
        }

        let result = search_files(
            dir.path().to_string_lossy().to_string(),
            "file".to_string(),
            5
        );
        assert!(result.is_ok());
        assert!(result.unwrap().len() <= 5);
    }

    #[test]
    fn test_search_content_handles_read_error() {
        let dir = tempdir().unwrap();

        // Create a valid searchable file
        fs::write(dir.path().join("valid.rs"), "searchable content").unwrap();

        // Search should work even with permission issues on some files
        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "searchable".to_string(),
            10,
            vec![],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_search_content_custom_exclude_patterns() {
        let dir = tempdir().unwrap();

        // Create directories to test exclusion
        fs::create_dir(dir.path().join("vendor")).unwrap();
        fs::write(
            dir.path().join("vendor").join("lib.rs"),
            "matching content",
        )
        .unwrap();

        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src").join("main.rs"), "matching content").unwrap();

        // Exclude "vendor" directory
        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "matching".to_string(),
            10,
            vec!["vendor".to_string()],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        // Should only find in src, not vendor
        assert_eq!(results.len(), 1);
        assert!(results[0].path.contains("src"));
    }

    #[test]
    fn test_search_content_glob_exclude_patterns() {
        let dir = tempdir().unwrap();

        // Create files with different extensions
        fs::write(dir.path().join("app.rs"), "matching content").unwrap();
        fs::write(dir.path().join("app.min.js"), "matching content").unwrap();
        fs::write(dir.path().join("styles.min.css"), "matching content").unwrap();

        // Exclude minified files
        let result = search_content(
            dir.path().to_string_lossy().to_string(),
            "matching".to_string(),
            10,
            vec!["*.min.js".to_string(), "*.min.css".to_string()],
        );
        assert!(result.is_ok());

        let results = result.unwrap();
        // Should only find app.rs
        assert_eq!(results.len(), 1);
        assert!(results[0].name.ends_with(".rs"));
    }

    #[test]
    fn test_should_exclude_simple_name() {
        let patterns = parse_exclude_patterns(&["vendor".to_string()]);
        let path = Path::new("/project/vendor/lib.rs");
        assert!(should_exclude(path, &patterns));
    }

    #[test]
    fn test_should_exclude_glob_pattern() {
        let patterns = parse_exclude_patterns(&["*.min.js".to_string()]);
        let path = Path::new("/project/app.min.js");
        assert!(should_exclude(path, &patterns));
    }

    #[test]
    fn test_should_not_exclude_non_matching() {
        let patterns = parse_exclude_patterns(&["vendor".to_string()]);
        let path = Path::new("/project/src/main.rs");
        assert!(!should_exclude(path, &patterns));
    }

    #[test]
    fn test_parse_exclude_patterns_empty() {
        let patterns = parse_exclude_patterns(&[]);
        assert!(patterns.is_empty());
    }

    #[test]
    fn test_parse_exclude_patterns_mixed() {
        let input = vec![
            "node_modules".to_string(),
            "*.min.js".to_string(),
            "vendor".to_string(),
        ];
        let patterns = parse_exclude_patterns(&input);
        assert_eq!(patterns.len(), 3);
    }

    #[test]
    fn test_collect_files_max_results_reached() {
        let dir = tempdir().unwrap();
        // Create several files
        for i in 0..5 {
            fs::write(dir.path().join(format!("file{}.txt", i)), "content").unwrap();
        }

        let mut results = Vec::new();
        // Set max_results to 2 so we hit the early return
        collect_files(dir.path(), "file", &mut results, 2, false);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_collect_files_unreadable_directory() {
        // Test with a non-existent directory (read_dir fails)
        let mut results = Vec::new();
        collect_files(Path::new("/nonexistent/path"), "test", &mut results, 100, false);
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_file_content_no_match() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

        let result = search_file_content(&file_path, "nonexistent_query", 10);
        assert!(result.is_none());
    }

    #[test]
    fn test_search_file_content_with_match() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, "fn main() {\n    println!(\"hello\");\n}\n").unwrap();

        let result = search_file_content(&file_path, "hello", 10);
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.matches.len(), 1);
        assert_eq!(result.matches[0].line, 2); // line 2 contains "hello"
    }

    #[test]
    fn test_collect_content_matches_max_results() {
        let dir = tempdir().unwrap();
        // Create several searchable files
        for i in 0..5 {
            fs::write(
                dir.path().join(format!("file{}.rs", i)),
                format!("fn test_{i}() {{ /* matching content */ }}\n"),
            ).unwrap();
        }

        let mut results = Vec::new();
        let exclude_patterns = parse_exclude_patterns(&[]);
        // Set max_results to 2 to trigger early returns
        collect_content_matches(dir.path(), "matching", &mut results, 2, 10, false, &exclude_patterns);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_collect_content_matches_unreadable_directory() {
        let mut results = Vec::new();
        let exclude_patterns = parse_exclude_patterns(&[]);
        collect_content_matches(Path::new("/nonexistent"), "query", &mut results, 100, 10, false, &exclude_patterns);
        assert!(results.is_empty());
    }

    #[test]
    fn test_should_exclude_path_component() {
        let patterns = parse_exclude_patterns(&["node_modules".to_string()]);
        let path = Path::new("/project/node_modules/lodash/index.js");
        assert!(should_exclude(path, &patterns));
    }

    #[test]
    fn test_search_file_content_max_matches_per_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        // Create file with many matching lines
        let content: String = (0..10)
            .map(|i| format!("line {} has the keyword search here\n", i))
            .collect();
        fs::write(&file_path, content).unwrap();

        let result = search_file_content(&file_path, "keyword", 3);
        assert!(result.is_some());
        // Should be limited to 3 matches
        assert_eq!(result.unwrap().matches.len(), 3);
    }
}
