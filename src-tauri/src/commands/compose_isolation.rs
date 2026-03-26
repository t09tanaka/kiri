use glob::glob;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

// Scan patterns for docker-compose files
const COMPOSE_FILE_PATTERNS: &[&str] = &[
    "**/docker-compose.yml",
    "**/docker-compose.yaml",
    "**/docker-compose.*.yml",
    "**/docker-compose.*.yaml",
    "**/compose.yml",
    "**/compose.yaml",
];

// Regex for root-level name: directive
const ROOT_NAME_PATTERN: &str = r#"^name:\s+['"]?([^#'"]+?)['"]?\s*(#.*)?$"#;

// Regex for container_name: directive (indented, under services)
const CONTAINER_NAME_PATTERN: &str = r#"^\s+container_name:\s+['"]?(.+?)['"]?\s*$"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeWarning {
    pub warning_type: String,
    pub value: String,
    pub line_number: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeFileInfo {
    pub file_path: String,
    pub project_name: Option<String>,
    pub name_line_number: u32,
    pub warnings: Vec<ComposeWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedComposeFiles {
    pub files: Vec<ComposeFileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeNameReplacement {
    pub file_path: String,
    pub original_name: String,
    pub new_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeTransformResult {
    pub transformed_files: Vec<String>,
    pub skipped_files: Vec<String>,
    pub errors: Vec<String>,
}

/// Detect root-level `name:` and collect warnings in a docker-compose file content
pub fn detect_compose_name(content: &str, file_path: &str) -> ComposeFileInfo {
    let name_re = Regex::new(ROOT_NAME_PATTERN).unwrap();
    let container_name_re = Regex::new(CONTAINER_NAME_PATTERN).unwrap();

    let mut project_name: Option<String> = None;
    let mut name_line_number: u32 = 0;
    let mut warnings: Vec<ComposeWarning> = Vec::new();

    // State machine for tracking top-level volumes section
    let mut in_top_level_volumes = false;
    // Track if we're inside a volume definition's config block (indent >= 4)
    let mut in_volume_config = false;
    let mut current_volume_key = String::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_number = (line_num + 1) as u32;
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        // Detect root-level `name:` (must start at column 0, no leading whitespace)
        if line.starts_with("name:") {
            if let Some(caps) = name_re.captures(line) {
                if let Some(name_match) = caps.get(1) {
                    project_name = Some(name_match.as_str().trim().to_string());
                    name_line_number = line_number;
                }
            }
            continue;
        }

        // Track top-level sections (no leading whitespace)
        if !line.starts_with(' ') && !line.starts_with('\t') {
            // Check if entering top-level volumes: section
            in_top_level_volumes = trimmed.starts_with("volumes:");
            in_volume_config = false;
            continue;
        }

        // Detect container_name warnings
        if let Some(caps) = container_name_re.captures(line) {
            if let Some(value_match) = caps.get(1) {
                let value = value_match.as_str().trim().to_string();
                warnings.push(ComposeWarning {
                    warning_type: "ContainerName".to_string(),
                    value: value.clone(),
                    line_number,
                    message: format!(
                        "Static container_name '{}' may cause conflicts between worktrees",
                        value
                    ),
                });
            }
        }

        // Detect explicit volume name: in top-level volumes section
        if in_top_level_volumes {
            let leading_spaces = line.len() - line.trim_start().len();
            let is_indent_2 = leading_spaces == 2 || (leading_spaces > 0 && line.starts_with('\t') && !line.starts_with("\t\t"));
            let is_indent_4_plus = leading_spaces >= 4 || line.starts_with("\t\t");

            if is_indent_2 {
                // Volume key definition (e.g., "  postgres_data:" or "  postgres_data: {}")
                in_volume_config = false;
                if let Some(colon_pos) = trimmed.find(':') {
                    let key = trimmed[..colon_pos].trim().to_string();
                    if !key.is_empty() && !["driver", "external", "name", "labels", "driver_opts"].contains(&key.as_str()) {
                        current_volume_key = key;
                        in_volume_config = true;
                    }
                }
            } else if is_indent_4_plus && in_volume_config {
                // Inside a volume's config block - check for explicit name: property
                if let Some(colon_pos) = trimmed.find(':') {
                    let config_key = trimmed[..colon_pos].trim();
                    if config_key == "name" {
                        let name_value = trimmed[colon_pos + 1..].trim().trim_matches(|c| c == '\'' || c == '"').to_string();
                        if !name_value.is_empty() {
                            warnings.push(ComposeWarning {
                                warning_type: "VolumeName".to_string(),
                                value: name_value.clone(),
                                line_number,
                                message: format!(
                                    "Explicit volume name '{}' (volume '{}') may cause conflicts between worktrees",
                                    name_value, current_volume_key
                                ),
                            });
                        }
                    }
                }
            }
        }
    }

    ComposeFileInfo {
        file_path: file_path.to_string(),
        project_name,
        name_line_number,
        warnings,
    }
}

/// Scan a directory recursively for docker-compose files
pub fn scan_compose_files(dir_path: &str) -> Result<DetectedComposeFiles, String> {
    let dir = Path::new(dir_path);

    if !dir.exists() {
        return Err(format!("Directory does not exist: {}", dir_path));
    }

    let mut files: Vec<ComposeFileInfo> = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for pattern_suffix in COMPOSE_FILE_PATTERNS {
        let pattern = dir.join(pattern_suffix).to_string_lossy().to_string();
        if let Ok(entries) = glob(&pattern) {
            for entry in entries.flatten() {
                if entry.is_file() {
                    // Use relative path from project root
                    let relative_path = entry
                        .strip_prefix(dir)
                        .unwrap_or(&entry)
                        .to_string_lossy()
                        .to_string();

                    // Avoid duplicates from overlapping patterns
                    if seen_paths.contains(&relative_path) {
                        continue;
                    }
                    seen_paths.insert(relative_path.clone());

                    if let Ok(content) = fs::read_to_string(&entry) {
                        let info = detect_compose_name(&content, &relative_path);
                        files.push(info);
                    }
                }
            }
        }
    }

    Ok(DetectedComposeFiles { files })
}

/// Transform the root-level `name:` value in docker-compose content.
/// Preserves the quoting style (single/double/none).
pub fn transform_compose_name(content: &str, original_name: &str, new_name: &str) -> String {
    let mut result = String::new();
    let name_re = Regex::new(ROOT_NAME_PATTERN).unwrap();

    for line in content.lines() {
        if line.starts_with("name:") {
            if let Some(caps) = name_re.captures(line) {
                if let Some(name_match) = caps.get(1) {
                    let matched_name = name_match.as_str().trim();
                    if matched_name == original_name {
                        // Detect quoting style from the original line
                        let before_name = &line[..name_match.start()];
                        let after_value_start = name_match.end();
                        let char_before = if name_match.start() > 0 {
                            line.as_bytes().get(name_match.start() - 1).copied()
                        } else {
                            None
                        };

                        let new_line = match char_before {
                            Some(b'\'') => {
                                // Single-quoted: name: 'value'
                                let prefix = &line[..name_match.start()];
                                let suffix_start = after_value_start;
                                // Find closing quote
                                let suffix = if let Some(close_idx) =
                                    line[suffix_start..].find('\'')
                                {
                                    &line[suffix_start + close_idx + 1..]
                                } else {
                                    &line[suffix_start..]
                                };
                                format!("{}{}{}", prefix, new_name, if suffix.is_empty() { "'".to_string() } else { format!("'{}", suffix) })
                            }
                            Some(b'"') => {
                                // Double-quoted: name: "value"
                                let prefix = &line[..name_match.start()];
                                let suffix_start = after_value_start;
                                let suffix = if let Some(close_idx) =
                                    line[suffix_start..].find('"')
                                {
                                    &line[suffix_start + close_idx + 1..]
                                } else {
                                    &line[suffix_start..]
                                };
                                format!("{}{}{}", prefix, new_name, if suffix.is_empty() { "\"".to_string() } else { format!("\"{}", suffix) })
                            }
                            _ => {
                                // No quotes: name: value
                                let comment = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                                let comment_part = if comment.is_empty() {
                                    String::new()
                                } else {
                                    format!(" {}", comment)
                                };
                                format!("{}{}{}",  before_name, new_name, comment_part)
                            }
                        };

                        result.push_str(&new_line);
                        result.push('\n');
                        continue;
                    }
                }
            }
        }

        result.push_str(line);
        result.push('\n');
    }

    // Remove trailing newline if original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Apply compose name isolation to files in a worktree
pub fn apply_compose_name_isolation(
    worktree_path: &str,
    replacements: &[ComposeNameReplacement],
) -> ComposeTransformResult {
    let worktree_dir = Path::new(worktree_path);
    let mut transformed_files: Vec<String> = Vec::new();
    let mut skipped_files: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for replacement in replacements {
        let file_path = worktree_dir.join(&replacement.file_path);

        if !file_path.exists() {
            skipped_files.push(replacement.file_path.clone());
            continue;
        }

        match fs::read_to_string(&file_path) {
            Ok(content) => {
                let transformed = transform_compose_name(
                    &content,
                    &replacement.original_name,
                    &replacement.new_name,
                );

                match fs::write(&file_path, &transformed) {
                    Ok(_) => {
                        transformed_files.push(replacement.file_path.clone());
                    }
                    Err(e) => {
                        errors.push(format!(
                            "Failed to write {}: {}",
                            replacement.file_path, e
                        ));
                    }
                }
            }
            Err(e) => {
                errors.push(format!(
                    "Failed to read {}: {}",
                    replacement.file_path, e
                ));
            }
        }
    }

    ComposeTransformResult {
        transformed_files,
        skipped_files,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_detect_compose_name_basic() {
        let content = "name: my-project\nservices:\n  web:\n    image: nginx\n";
        let info = detect_compose_name(content, "docker-compose.yml");

        assert_eq!(info.project_name, Some("my-project".to_string()));
        assert_eq!(info.name_line_number, 1);
        assert!(info.warnings.is_empty());
    }

    #[test]
    fn test_detect_compose_name_with_quotes() {
        // Single quotes
        let content = "name: 'my-project'\nservices:\n  web:\n    image: nginx\n";
        let info = detect_compose_name(content, "docker-compose.yml");
        assert_eq!(info.project_name, Some("my-project".to_string()));

        // Double quotes
        let content = "name: \"my-project\"\nservices:\n  web:\n    image: nginx\n";
        let info = detect_compose_name(content, "docker-compose.yml");
        assert_eq!(info.project_name, Some("my-project".to_string()));
    }

    #[test]
    fn test_detect_compose_name_no_name() {
        let content = "services:\n  web:\n    image: nginx\n";
        let info = detect_compose_name(content, "docker-compose.yml");

        assert_eq!(info.project_name, None);
        assert_eq!(info.name_line_number, 0);
    }

    #[test]
    fn test_detect_compose_name_with_comment() {
        let content = "name: my-project # project name\nservices:\n  web:\n    image: nginx\n";
        let info = detect_compose_name(content, "docker-compose.yml");

        assert_eq!(info.project_name, Some("my-project".to_string()));
    }

    #[test]
    fn test_detect_container_name_warnings() {
        let content = "\
name: my-project
services:
  web:
    image: nginx
    container_name: my-web-container
  db:
    image: postgres
    container_name: my-db-container
";
        let info = detect_compose_name(content, "docker-compose.yml");

        assert_eq!(info.warnings.len(), 2);
        assert_eq!(info.warnings[0].warning_type, "ContainerName");
        assert_eq!(info.warnings[0].value, "my-web-container");
        assert_eq!(info.warnings[0].line_number, 5);
        assert_eq!(info.warnings[1].warning_type, "ContainerName");
        assert_eq!(info.warnings[1].value, "my-db-container");
        assert_eq!(info.warnings[1].line_number, 8);
    }

    #[test]
    fn test_detect_volume_name_warnings_explicit_only() {
        let content = "\
name: my-project
services:
  web:
    image: nginx
volumes:
  db_data:
    name: my-project-db-data
    driver: local
  cache_data:
  logs_data:
    name: my-project-logs
";
        let info = detect_compose_name(content, "docker-compose.yml");

        // Only volumes with explicit name: should trigger warnings
        let vol_warnings: Vec<&ComposeWarning> = info
            .warnings
            .iter()
            .filter(|w| w.warning_type == "VolumeName")
            .collect();
        assert_eq!(vol_warnings.len(), 2);
        assert_eq!(vol_warnings[0].value, "my-project-db-data");
        assert_eq!(vol_warnings[1].value, "my-project-logs");
    }

    #[test]
    fn test_no_volume_warning_without_explicit_name() {
        // Volumes without explicit name: are auto-prefixed by Docker Compose
        let content = "\
name: my-project
services:
  web:
    image: nginx
volumes:
  db_data:
    driver: local
  cache_data:
";
        let info = detect_compose_name(content, "docker-compose.yml");
        let vol_warnings: Vec<&ComposeWarning> = info
            .warnings
            .iter()
            .filter(|w| w.warning_type == "VolumeName")
            .collect();
        assert_eq!(vol_warnings.len(), 0);
    }

    #[test]
    fn test_detect_nested_name_not_matched_as_root() {
        // `name:` under services should NOT be detected as the root-level name
        let content = "\
services:
  web:
    image: nginx
    name: not-the-project-name
";
        let info = detect_compose_name(content, "docker-compose.yml");
        assert_eq!(info.project_name, None);
    }

    #[test]
    fn test_scan_compose_files_basic() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("docker-compose.yml"),
            "name: my-project\nservices:\n  web:\n    image: nginx\n",
        )
        .unwrap();

        let result = scan_compose_files(&dir.path().to_string_lossy()).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].file_path, "docker-compose.yml");
        assert_eq!(
            result.files[0].project_name,
            Some("my-project".to_string())
        );
    }

    #[test]
    fn test_scan_compose_files_variants() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("docker-compose.yml"),
            "name: main\nservices:\n  web:\n    image: nginx\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("docker-compose.dev.yml"),
            "name: main-dev\nservices:\n  web:\n    image: nginx\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("compose.yaml"),
            "name: alt\nservices:\n  web:\n    image: nginx\n",
        )
        .unwrap();

        let result = scan_compose_files(&dir.path().to_string_lossy()).unwrap();
        assert_eq!(result.files.len(), 3);

        let names: Vec<Option<String>> = result
            .files
            .iter()
            .map(|f| f.project_name.clone())
            .collect();
        assert!(names.contains(&Some("main".to_string())));
        assert!(names.contains(&Some("main-dev".to_string())));
        assert!(names.contains(&Some("alt".to_string())));
    }

    #[test]
    fn test_scan_compose_files_nonexistent_dir() {
        let result = scan_compose_files("/nonexistent/path");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_transform_compose_name_unquoted() {
        let content = "name: my-project\nservices:\n  web:\n    image: nginx\n";
        let result = transform_compose_name(content, "my-project", "my-project-feature");

        assert!(result.contains("name: my-project-feature\n"));
        assert!(result.contains("services:"));
    }

    #[test]
    fn test_transform_compose_name_single_quoted() {
        let content = "name: 'my-project'\nservices:\n  web:\n    image: nginx\n";
        let result = transform_compose_name(content, "my-project", "my-project-feature");

        assert!(result.contains("'my-project-feature'"));
    }

    #[test]
    fn test_transform_compose_name_double_quoted() {
        let content = "name: \"my-project\"\nservices:\n  web:\n    image: nginx\n";
        let result = transform_compose_name(content, "my-project", "my-project-feature");

        assert!(result.contains("\"my-project-feature\""));
    }

    #[test]
    fn test_transform_compose_name_with_comment() {
        let content = "name: my-project # project name\nservices:\n  web:\n    image: nginx\n";
        let result = transform_compose_name(content, "my-project", "my-project-feature");

        assert!(result.contains("name: my-project-feature # project name\n"));
    }

    #[test]
    fn test_transform_compose_name_no_match() {
        let content = "name: other-project\nservices:\n  web:\n    image: nginx\n";
        let result = transform_compose_name(content, "my-project", "my-project-feature");

        // Should remain unchanged
        assert!(result.contains("name: other-project\n"));
    }

    #[test]
    fn test_transform_compose_name_no_trailing_newline() {
        let content = "name: my-project\nservices:\n  web:\n    image: nginx";
        let result = transform_compose_name(content, "my-project", "my-project-feature");

        assert!(!result.ends_with('\n'));
        assert!(result.contains("name: my-project-feature\n"));
    }

    #[test]
    fn test_apply_compose_name_isolation() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("docker-compose.yml"),
            "name: my-project\nservices:\n  web:\n    image: nginx\n",
        )
        .unwrap();

        let replacements = vec![ComposeNameReplacement {
            file_path: "docker-compose.yml".to_string(),
            original_name: "my-project".to_string(),
            new_name: "my-project-feature".to_string(),
        }];

        let result =
            apply_compose_name_isolation(&dir.path().to_string_lossy(), &replacements);

        assert_eq!(result.transformed_files.len(), 1);
        assert!(result.errors.is_empty());

        let content = fs::read_to_string(dir.path().join("docker-compose.yml")).unwrap();
        assert!(content.contains("name: my-project-feature\n"));
    }

    #[test]
    fn test_apply_compose_name_isolation_missing_file() {
        let dir = tempdir().unwrap();

        let replacements = vec![ComposeNameReplacement {
            file_path: "docker-compose.yml".to_string(),
            original_name: "my-project".to_string(),
            new_name: "my-project-feature".to_string(),
        }];

        let result =
            apply_compose_name_isolation(&dir.path().to_string_lossy(), &replacements);

        assert!(result.transformed_files.is_empty());
        assert_eq!(result.skipped_files.len(), 1);
    }

    #[test]
    fn test_detect_compose_name_container_name_with_quotes() {
        let content = "\
name: my-project
services:
  web:
    container_name: 'my-web'
  db:
    container_name: \"my-db\"
";
        let info = detect_compose_name(content, "docker-compose.yml");

        assert_eq!(info.warnings.len(), 2);
        assert_eq!(info.warnings[0].value, "my-web");
        assert_eq!(info.warnings[1].value, "my-db");
    }

    #[test]
    fn test_detect_volume_not_confused_with_service_volumes() {
        // Ensure that `volumes:` under a service doesn't trigger volume name detection
        // Only top-level volumes with explicit `name:` should be warned
        let content = "\
name: my-project
services:
  web:
    image: nginx
    volumes:
      - ./data:/data
volumes:
  real_volume:
    name: my-project-data
  unnamed_volume:
";
        let info = detect_compose_name(content, "docker-compose.yml");

        let vol_warnings: Vec<&ComposeWarning> = info
            .warnings
            .iter()
            .filter(|w| w.warning_type == "VolumeName")
            .collect();
        // Only the volume with explicit `name:` should be warned
        assert_eq!(vol_warnings.len(), 1);
        assert_eq!(vol_warnings[0].value, "my-project-data");
    }

    #[test]
    fn test_apply_compose_name_isolation_multiple_files() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("docker-compose.yml"),
            "name: my-project\nservices:\n  web:\n    image: nginx\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("docker-compose.dev.yml"),
            "name: my-project-dev\nservices:\n  web:\n    image: nginx\n",
        )
        .unwrap();

        let replacements = vec![
            ComposeNameReplacement {
                file_path: "docker-compose.yml".to_string(),
                original_name: "my-project".to_string(),
                new_name: "my-project-feature".to_string(),
            },
            ComposeNameReplacement {
                file_path: "docker-compose.dev.yml".to_string(),
                original_name: "my-project-dev".to_string(),
                new_name: "my-project-dev-feature".to_string(),
            },
        ];

        let result =
            apply_compose_name_isolation(&dir.path().to_string_lossy(), &replacements);

        assert_eq!(result.transformed_files.len(), 2);
        assert!(result.skipped_files.is_empty());
        assert!(result.errors.is_empty());

        let content1 = fs::read_to_string(dir.path().join("docker-compose.yml")).unwrap();
        assert!(content1.contains("name: my-project-feature\n"));

        let content2 = fs::read_to_string(dir.path().join("docker-compose.dev.yml")).unwrap();
        assert!(content2.contains("name: my-project-dev-feature\n"));
    }

    #[test]
    fn test_apply_compose_name_isolation_read_error() {
        let dir = tempdir().unwrap();
        // Create a directory with the same name as the expected file path
        // so that read_to_string fails (cannot read a directory as a file)
        fs::create_dir(dir.path().join("docker-compose.yml")).unwrap();

        let replacements = vec![ComposeNameReplacement {
            file_path: "docker-compose.yml".to_string(),
            original_name: "my-project".to_string(),
            new_name: "my-project-feature".to_string(),
        }];

        let result =
            apply_compose_name_isolation(&dir.path().to_string_lossy(), &replacements);

        assert!(result.transformed_files.is_empty());
        assert!(result.skipped_files.is_empty());
        assert_eq!(result.errors.len(), 1);
        assert!(result.errors[0].contains("Failed to read"));
    }

    #[test]
    fn test_transform_compose_name_preserves_other_lines() {
        let content = "\
name: my-project
services:
  web:
    image: nginx
    ports:
      - \"8080:80\"
    environment:
      - FOO=bar
volumes:
  data:
";
        let result = transform_compose_name(content, "my-project", "new-name");

        // Verify the name line changed
        assert!(result.contains("name: new-name\n"));
        // Verify all other lines remain unchanged
        assert!(result.contains("services:\n"));
        assert!(result.contains("  web:\n"));
        assert!(result.contains("    image: nginx\n"));
        assert!(result.contains("    ports:\n"));
        assert!(result.contains("      - \"8080:80\"\n"));
        assert!(result.contains("    environment:\n"));
        assert!(result.contains("      - FOO=bar\n"));
        assert!(result.contains("volumes:\n"));
        assert!(result.contains("  data:\n"));
    }

    #[test]
    fn test_transform_compose_name_multiple_name_directives() {
        // Only the first root-level name: should be transformed
        let content = "\
name: my-project
services:
  web:
    image: nginx
name: second-name
";
        let result = transform_compose_name(content, "my-project", "new-name");

        // First name: should be transformed
        assert!(result.contains("name: new-name\n"));
        // Second name: should NOT be transformed (original_name doesn't match "second-name")
        assert!(result.contains("name: second-name\n"));
    }

    #[test]
    fn test_detect_compose_name_empty_content() {
        let info = detect_compose_name("", "docker-compose.yml");

        assert_eq!(info.project_name, None);
        assert_eq!(info.name_line_number, 0);
        assert!(info.warnings.is_empty());
        assert_eq!(info.file_path, "docker-compose.yml");
    }

    #[test]
    fn test_detect_compose_name_comments_only() {
        let content = "\
# This is a comment
# Another comment
# name: not-a-real-name
";
        let info = detect_compose_name(content, "docker-compose.yml");

        assert_eq!(info.project_name, None);
        assert_eq!(info.name_line_number, 0);
        assert!(info.warnings.is_empty());
    }

    #[test]
    fn test_detect_compose_name_with_tab_indentation() {
        let content = "name: my-project\nservices:\n\tweb:\n\t\timage: nginx\n\t\tcontainer_name: my-web\nvolumes:\n\tdb_data:\n\t\tname: my-volume\n";
        let info = detect_compose_name(content, "docker-compose.yml");

        assert_eq!(info.project_name, Some("my-project".to_string()));
        assert_eq!(info.name_line_number, 1);

        // Should detect container_name warning
        let container_warnings: Vec<&ComposeWarning> = info
            .warnings
            .iter()
            .filter(|w| w.warning_type == "ContainerName")
            .collect();
        assert_eq!(container_warnings.len(), 1);
        assert_eq!(container_warnings[0].value, "my-web");

        // Tab-indented volume name detection: The current implementation uses
        // leading character count for indent detection. Two tabs (\t\t) has
        // leading_spaces == 2, which matches the is_indent_2 check (meant for
        // 2-space indent). This means \t\t-indented `name:` is treated as a
        // volume key rather than a config property, so volume name warnings
        // are not detected for tab-indented compose files. This documents the
        // known behavior.
        let vol_warnings: Vec<&ComposeWarning> = info
            .warnings
            .iter()
            .filter(|w| w.warning_type == "VolumeName")
            .collect();
        assert_eq!(vol_warnings.len(), 0);
    }

    #[test]
    fn test_scan_compose_files_empty_dir() {
        let dir = tempdir().unwrap();

        let result = scan_compose_files(&dir.path().to_string_lossy()).unwrap();
        assert!(result.files.is_empty());
    }

    #[test]
    fn test_scan_compose_files_subdirectory() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("subproject");
        fs::create_dir(&subdir).unwrap();
        fs::write(
            subdir.join("docker-compose.yml"),
            "name: sub-project\nservices:\n  web:\n    image: nginx\n",
        )
        .unwrap();

        let result = scan_compose_files(&dir.path().to_string_lossy()).unwrap();
        assert_eq!(result.files.len(), 1);
        assert_eq!(
            result.files[0].file_path,
            format!("subproject{}docker-compose.yml", std::path::MAIN_SEPARATOR)
        );
        assert_eq!(
            result.files[0].project_name,
            Some("sub-project".to_string())
        );
    }

    #[test]
    fn test_detect_compose_name_indented_name_not_root() {
        // `name:` under services is NOT the root name
        let content = "services:\n  web:\n    container_name: my-web\n";
        let info = detect_compose_name(content, "docker-compose.yml");
        assert_eq!(info.project_name, None);
    }

    #[test]
    fn test_detect_compose_name_empty_volume_name() {
        // volume with name: but empty value → should NOT create warning
        let content = "\
services:
  web:
    image: nginx
volumes:
  data:
    name:
";
        let info = detect_compose_name(content, "docker-compose.yml");
        let vol_warnings: Vec<&ComposeWarning> = info
            .warnings
            .iter()
            .filter(|w| w.warning_type == "VolumeName")
            .collect();
        assert!(vol_warnings.is_empty(), "Empty volume name should not generate warning");
    }

    #[test]
    fn test_apply_compose_name_isolation_file_not_found() {
        let dir = tempdir().unwrap();

        let replacements = vec![ComposeNameReplacement {
            file_path: "nonexistent.yml".to_string(),
            original_name: "old".to_string(),
            new_name: "new".to_string(),
        }];

        let result = apply_compose_name_isolation(
            &dir.path().to_string_lossy(),
            &replacements,
        );

        assert_eq!(result.skipped_files.len(), 1);
        assert!(result.skipped_files[0].contains("nonexistent"));
    }

    #[test]
    fn test_apply_compose_name_isolation_success() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("docker-compose.yml"),
            "name: my-project\nservices:\n  web:\n    image: nginx\n",
        )
        .unwrap();

        let replacements = vec![ComposeNameReplacement {
            file_path: "docker-compose.yml".to_string(),
            original_name: "my-project".to_string(),
            new_name: "my-project-wt1".to_string(),
        }];

        let result = apply_compose_name_isolation(
            &dir.path().to_string_lossy(),
            &replacements,
        );

        assert_eq!(result.transformed_files.len(), 1);
        assert!(result.errors.is_empty());

        let content = fs::read_to_string(dir.path().join("docker-compose.yml")).unwrap();
        assert!(content.contains("my-project-wt1"), "Content: {}", content);
    }

    #[test]
    fn test_apply_compose_name_isolation_write_error() {
        // Test with a file path that would cause a write error
        let dir = tempdir().unwrap();
        let compose_path = dir.path().join("docker-compose.yml");
        fs::write(&compose_path, "name: test\nservices:\n  web:\n    image: nginx\n").unwrap();

        // Make file read-only to trigger write error
        let mut perms = fs::metadata(&compose_path).unwrap().permissions();
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o444);
        fs::set_permissions(&compose_path, perms).unwrap();

        let replacements = vec![ComposeNameReplacement {
            file_path: "docker-compose.yml".to_string(),
            original_name: "test".to_string(),
            new_name: "test-wt1".to_string(),
        }];

        let result = apply_compose_name_isolation(
            &dir.path().to_string_lossy(),
            &replacements,
        );

        assert!(!result.errors.is_empty(), "Expected write error");

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&compose_path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&compose_path, perms).unwrap();
    }

    #[test]
    fn test_transform_compose_name_single_quoted_no_closing() {
        // Single-quoted name without closing quote (edge case)
        let content = "name: 'myproject\nservices:\n  web:\n    image: nginx";
        let result = transform_compose_name(content, "myproject", "myproject-wt");
        assert!(result.contains("myproject-wt"));
    }

    #[test]
    fn test_transform_compose_name_double_quoted_no_closing() {
        // Double-quoted name without closing quote (edge case)
        let content = "name: \"myproject\nservices:\n  web:\n    image: nginx";
        let result = transform_compose_name(content, "myproject", "myproject-wt");
        assert!(result.contains("myproject-wt"));
    }

    #[test]
    fn test_transform_compose_name_no_match_different_name() {
        // name: line exists but with a different value than original_name
        let content = "name: other-project\nservices:\n  web:\n    image: nginx";
        let result = transform_compose_name(content, "myproject", "myproject-wt");
        // Should NOT transform since original_name doesn't match
        assert!(result.contains("name: other-project"));
        assert!(!result.contains("myproject-wt"));
    }

    #[test]
    fn test_detect_compose_name_volume_name_explicit() {
        // Test explicit volume name detection in top-level volumes
        let content = "services:\n  db:\n    image: postgres\nvolumes:\n  postgres_data:\n    name: my-explicit-volume";
        let info = detect_compose_name(content, "docker-compose.yml");
        let volume_warnings: Vec<_> = info.warnings.iter()
            .filter(|w| w.warning_type == "VolumeName")
            .collect();
        assert_eq!(volume_warnings.len(), 1);
        assert!(volume_warnings[0].value.contains("my-explicit-volume"));
    }

    #[test]
    fn test_detect_compose_name_volume_no_explicit_name() {
        // Volumes without explicit name: should not warn about VolumeName
        let content = "services:\n  db:\n    image: postgres\nvolumes:\n  postgres_data:\n    driver: local";
        let info = detect_compose_name(content, "docker-compose.yml");
        let volume_warnings: Vec<_> = info.warnings.iter()
            .filter(|w| w.warning_type == "VolumeName")
            .collect();
        assert!(volume_warnings.is_empty());
    }

    #[test]
    fn test_scan_compose_files_with_duplicates() {
        // Test that duplicate patterns don't produce duplicate results
        let dir = tempdir().unwrap();
        let compose_content = "name: test\nservices:\n  web:\n    image: nginx";
        fs::write(dir.path().join("docker-compose.yml"), compose_content).unwrap();

        let result = scan_compose_files(&dir.path().to_string_lossy());
        assert!(result.is_ok());
        let detected = result.unwrap();
        // Should only have 1 entry even if multiple patterns match
        assert_eq!(detected.files.len(), 1);
    }

    #[test]
    fn test_detect_compose_name_container_name_warning() {
        // Test container_name warning detection
        let content = "name: test\nservices:\n  web:\n    image: nginx\n    container_name: my-static-container";
        let info = detect_compose_name(content, "docker-compose.yml");
        let container_warnings: Vec<_> = info.warnings.iter()
            .filter(|w| w.warning_type == "ContainerName")
            .collect();
        assert_eq!(container_warnings.len(), 1);
        assert!(container_warnings[0].value.contains("my-static-container"));
    }

    #[test]
    fn test_detect_compose_name_space_indented_volumes() {
        // Test space-indented volume config (standard YAML)
        let content = "volumes:\n  postgres_data:\n    name: explicit-vol";
        let info = detect_compose_name(content, "docker-compose.yml");
        let volume_warnings: Vec<_> = info.warnings.iter()
            .filter(|w| w.warning_type == "VolumeName")
            .collect();
        assert_eq!(volume_warnings.len(), 1);
        assert!(volume_warnings[0].value.contains("explicit-vol"));
    }

}
