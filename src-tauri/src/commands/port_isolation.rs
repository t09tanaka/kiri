use glob::{glob, glob_with, MatchOptions};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

// Regex patterns for port detection
const ENV_PORT_PATTERN: &str = r"^([A-Z_]*PORT[A-Z_]*)=(\d+)";
// Pattern for URLs with ports: VAR_URL=protocol://host:PORT or VAR_URL=protocol://user:pass@host:PORT
const ENV_URL_PORT_PATTERN: &str = r"^([A-Z][A-Z0-9_]*_URL)=\S+://(?:[^:@/]+(?::[^@/]+)?@)?[^:/]+:(\d+)";
const DOCKERFILE_EXPOSE_PATTERN: &str = r"^EXPOSE\s+(\d+)";
const COMPOSE_PORT_PATTERN: &str = r#"^\s*-\s*["']?(\d+):(\d+)["']?"#;

// Port range for allocation
const PORT_RANGE_START: u16 = 20000;
const PORT_RANGE_END: u16 = 39999;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortAssignment {
    pub variable_name: String,
    pub original_value: u16,
    pub assigned_value: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortSource {
    pub file_path: String,
    pub variable_name: String,
    pub port_value: u16,
    pub line_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPorts {
    pub env_ports: Vec<PortSource>,
    pub dockerfile_ports: Vec<PortSource>,
    pub compose_ports: Vec<PortSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortAllocationResult {
    pub assignments: Vec<PortAssignment>,
    pub next_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    pub content: String,
    pub replacements: Vec<PortReplacement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortReplacement {
    pub variable_name: String,
    pub original_value: u16,
    pub new_value: u16,
    pub line_number: u32,
}

/// Detect port variables in .env file content
pub fn detect_ports_in_env_file(content: &str, file_path: &str) -> Vec<PortSource> {
    let port_re = Regex::new(ENV_PORT_PATTERN).unwrap();
    let url_re = Regex::new(ENV_URL_PORT_PATTERN).unwrap();
    let mut ports = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        // Skip comments and empty lines
        if trimmed.starts_with('#') || trimmed.is_empty() {
            continue;
        }

        // Check for PORT variables (e.g., PORT=3000, DB_PORT=5432)
        if let Some(caps) = port_re.captures(trimmed) {
            if let (Some(var_name), Some(port_str)) = (caps.get(1), caps.get(2)) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    ports.push(PortSource {
                        file_path: file_path.to_string(),
                        variable_name: var_name.as_str().to_string(),
                        port_value: port,
                        line_number: (line_num + 1) as u32,
                    });
                }
            }
        }
        // Check for URL variables with ports (e.g., REDIS_URL=redis://localhost:6379)
        else if let Some(caps) = url_re.captures(trimmed) {
            if let (Some(var_name), Some(port_str)) = (caps.get(1), caps.get(2)) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    ports.push(PortSource {
                        file_path: file_path.to_string(),
                        variable_name: var_name.as_str().to_string(),
                        port_value: port,
                        line_number: (line_num + 1) as u32,
                    });
                }
            }
        }
    }

    ports
}

/// Detect EXPOSE directives in Dockerfile content
pub fn detect_ports_in_dockerfile(content: &str, file_path: &str) -> Vec<PortSource> {
    let re = Regex::new(DOCKERFILE_EXPOSE_PATTERN).unwrap();
    let mut ports = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        if let Some(caps) = re.captures(trimmed) {
            if let Some(port_str) = caps.get(1) {
                if let Ok(port) = port_str.as_str().parse::<u16>() {
                    ports.push(PortSource {
                        file_path: file_path.to_string(),
                        variable_name: "EXPOSE".to_string(),
                        port_value: port,
                        line_number: (line_num + 1) as u32,
                    });
                }
            }
        }
    }

    ports
}

/// Detect port mappings in docker-compose.yml content
pub fn detect_ports_in_compose(content: &str, file_path: &str) -> Vec<PortSource> {
    let re = Regex::new(COMPOSE_PORT_PATTERN).unwrap();
    let mut ports = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        if let Some(caps) = re.captures(line) {
            if let Some(host_port_str) = caps.get(1) {
                if let Ok(port) = host_port_str.as_str().parse::<u16>() {
                    ports.push(PortSource {
                        file_path: file_path.to_string(),
                        variable_name: format!("COMPOSE:{}", port),
                        port_value: port,
                        line_number: (line_num + 1) as u32,
                    });
                }
            }
        }
    }

    ports
}

/// Scan a directory for all .env* files and detect ports
pub fn scan_env_files_for_ports(dir: &Path) -> Vec<PortSource> {
    let mut all_ports = Vec::new();
    let pattern = dir.join("**/.env*").to_string_lossy().to_string();

    log::info!("Scanning for .env files with pattern: {}", pattern);

    // Use glob options to match dot files (hidden files)
    let options = MatchOptions {
        require_literal_leading_dot: false, // Match .env* files
        ..Default::default()
    };

    match glob_with(&pattern, options) {
        Ok(entries) => {
            let entries_vec: Vec<_> = entries.flatten().collect();
            log::info!("Found {} entries matching pattern", entries_vec.len());
            for entry in entries_vec {
                log::info!("Processing entry: {:?}", entry);
                if entry.is_file() {
                    if let Ok(content) = fs::read_to_string(&entry) {
                        let file_path = entry.to_string_lossy().to_string();
                        let ports = detect_ports_in_env_file(&content, &file_path);
                        log::info!("Found {} ports in {}", ports.len(), file_path);
                        all_ports.extend(ports);
                    }
                }
            }
        }
        Err(e) => {
            log::error!("Glob pattern error: {}", e);
        }
    }

    log::info!("Total ports found: {}", all_ports.len());
    all_ports
}

/// Scan a directory (including subdirectories) for Dockerfile and detect ports
pub fn scan_dockerfile_for_ports(dir: &Path) -> Vec<PortSource> {
    let mut all_ports = Vec::new();

    let dockerfile_patterns = [
        "**/Dockerfile",
        "**/Dockerfile.*",
    ];

    for pattern_suffix in dockerfile_patterns {
        let pattern = dir.join(pattern_suffix).to_string_lossy().to_string();
        if let Ok(entries) = glob(&pattern) {
            for entry in entries.flatten() {
                if entry.is_file() {
                    if let Ok(content) = fs::read_to_string(&entry) {
                        let file_path = entry.to_string_lossy().to_string();
                        let ports = detect_ports_in_dockerfile(&content, &file_path);
                        all_ports.extend(ports);
                    }
                }
            }
        }
    }

    all_ports
}

/// Scan a directory (including subdirectories) for docker-compose files and detect ports
pub fn scan_compose_for_ports(dir: &Path) -> Vec<PortSource> {
    let mut all_ports = Vec::new();

    let compose_patterns = [
        "**/docker-compose.yml",
        "**/docker-compose.yaml",
        "**/docker-compose.*.yml",
        "**/docker-compose.*.yaml",
        "**/compose.yml",
        "**/compose.yaml",
    ];

    for pattern_suffix in compose_patterns {
        let pattern = dir.join(pattern_suffix).to_string_lossy().to_string();
        if let Ok(entries) = glob(&pattern) {
            for entry in entries.flatten() {
                if entry.is_file() {
                    if let Ok(content) = fs::read_to_string(&entry) {
                        let file_path = entry.to_string_lossy().to_string();
                        let ports = detect_ports_in_compose(&content, &file_path);
                        all_ports.extend(ports);
                    }
                }
            }
        }
    }

    all_ports
}

/// Detect all ports in a directory
pub fn detect_all_ports(dir_path: String) -> Result<DetectedPorts, String> {
    let dir = Path::new(&dir_path);

    if !dir.exists() {
        return Err(format!("Directory does not exist: {}", dir_path));
    }

    let env_ports = scan_env_files_for_ports(dir);
    let dockerfile_ports = scan_dockerfile_for_ports(dir);
    let compose_ports = scan_compose_for_ports(dir);

    Ok(DetectedPorts {
        env_ports,
        dockerfile_ports,
        compose_ports,
    })
}

/// Allocate unique ports for the given detected ports
pub fn allocate_ports(
    ports: &[PortSource],
    start_port: u16,
) -> Result<PortAllocationResult, String> {
    if start_port < PORT_RANGE_START || start_port > PORT_RANGE_END {
        return Err(format!(
            "Start port must be between {} and {}",
            PORT_RANGE_START, PORT_RANGE_END
        ));
    }

    let mut assignments = Vec::new();
    let mut next_port = start_port;
    let mut seen_vars: HashMap<String, u16> = HashMap::new();

    for port_source in ports {
        // Skip if we've already assigned this variable
        if let Some(&assigned_port) = seen_vars.get(&port_source.variable_name) {
            assignments.push(PortAssignment {
                variable_name: port_source.variable_name.clone(),
                original_value: port_source.port_value,
                assigned_value: assigned_port,
            });
            continue;
        }

        // Check if we have room for more ports
        if next_port > PORT_RANGE_END {
            return Err("Port range exhausted".to_string());
        }

        let assigned_port = next_port;
        seen_vars.insert(port_source.variable_name.clone(), assigned_port);
        next_port += 1;

        assignments.push(PortAssignment {
            variable_name: port_source.variable_name.clone(),
            original_value: port_source.port_value,
            assigned_value: assigned_port,
        });
    }

    Ok(PortAllocationResult {
        assignments,
        next_port,
    })
}

/// Transform .env content by replacing port values
pub fn transform_env_content(
    content: &str,
    assignments: &[PortAssignment],
) -> TransformResult {
    let mut result = String::new();
    let mut replacements = Vec::new();

    // Build a map of variable_name -> assignment
    let assignment_map: HashMap<&str, &PortAssignment> = assignments
        .iter()
        .map(|a| (a.variable_name.as_str(), a))
        .collect();

    let port_re = Regex::new(ENV_PORT_PATTERN).unwrap();
    // For URL replacement, capture the part before the port and the port itself
    // Pattern: (VAR_URL=...://host):PORT -> captures prefix and port separately
    let url_re =
        Regex::new(r"^([A-Z][A-Z0-9_]*_URL=\S+://(?:[^:@/]+(?::[^@/]+)?@)?[^:/]+):(\d+)").unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Preserve comments and empty lines as-is
        if trimmed.starts_with('#') || trimmed.is_empty() {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Check for PORT variables (e.g., PORT=3000)
        if let Some(caps) = port_re.captures(trimmed) {
            if let Some(var_name) = caps.get(1) {
                if let Some(assignment) = assignment_map.get(var_name.as_str()) {
                    // Replace the port value
                    let new_line = port_re.replace(line, |caps: &regex::Captures| {
                        format!("{}={}", &caps[1], assignment.assigned_value)
                    });
                    result.push_str(&new_line);
                    result.push('\n');
                    replacements.push(PortReplacement {
                        variable_name: var_name.as_str().to_string(),
                        original_value: assignment.original_value,
                        new_value: assignment.assigned_value,
                        line_number: (line_num + 1) as u32,
                    });
                    continue;
                }
            }
        }
        // Check for URL variables with ports (e.g., REDIS_URL=redis://localhost:6379)
        else if let Some(caps) = url_re.captures(trimmed) {
            // Extract variable name from the prefix (VAR_URL=...)
            if let Some(prefix) = caps.get(1) {
                let prefix_str = prefix.as_str();
                if let Some(eq_pos) = prefix_str.find('=') {
                    let var_name = &prefix_str[..eq_pos];
                    if let Some(assignment) = assignment_map.get(var_name) {
                        // Replace the port in the URL
                        let new_line = url_re.replace(line, |caps: &regex::Captures| {
                            format!("{}:{}", &caps[1], assignment.assigned_value)
                        });
                        result.push_str(&new_line);
                        result.push('\n');
                        replacements.push(PortReplacement {
                            variable_name: var_name.to_string(),
                            original_value: assignment.original_value,
                            new_value: assignment.assigned_value,
                            line_number: (line_num + 1) as u32,
                        });
                        continue;
                    }
                }
            }
        }

        // No match, keep line as-is
        result.push_str(line);
        result.push('\n');
    }

    // Remove trailing newline if original didn't have one
    if !content.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    TransformResult {
        content: result,
        replacements,
    }
}

/// Transform generic file content by replacing port numbers
/// This is a simple find-and-replace approach for non-.env files
pub fn transform_generic_content(content: &str, assignments: &[PortAssignment]) -> String {
    let mut result = content.to_string();

    // Sort by original_value descending to replace longer numbers first
    // This prevents issues like 3000 being replaced before 30001
    let mut sorted_assignments: Vec<&PortAssignment> = assignments.iter().collect();
    sorted_assignments.sort_by(|a, b| b.original_value.cmp(&a.original_value));

    for assignment in sorted_assignments {
        let original = assignment.original_value.to_string();
        let new_value = assignment.assigned_value.to_string();

        // Use word boundary matching to avoid partial replacements
        // Match port numbers surrounded by non-digit characters
        // We need to run multiple passes because Rust regex doesn't support lookbehind
        // and matches can overlap (e.g., "3000:3000" needs two replacements)
        let pattern = format!(
            r"(?P<before>^|[^0-9]){}(?P<after>[^0-9]|$)",
            regex::escape(&original)
        );

        if let Ok(re) = Regex::new(&pattern) {
            // Run multiple passes until no more replacements occur
            loop {
                let new_result = re
                    .replace_all(&result, format!("${{before}}{}${{after}}", new_value))
                    .to_string();
                if new_result == result {
                    break;
                }
                result = new_result;
            }
        }
    }

    result
}

/// Copy files with port transformation
pub fn copy_files_with_port_transformation(
    source_path: String,
    target_path: String,
    patterns: Vec<String>,
    assignments: Vec<PortAssignment>,
) -> Result<super::git_worktree::CopyResult, String> {
    let source = Path::new(&source_path);
    let target = Path::new(&target_path);

    if !source.exists() {
        return Err(format!("Source path does not exist: {}", source_path));
    }

    if !target.exists() {
        return Err(format!("Target path does not exist: {}", target_path));
    }

    let mut copied_files = Vec::new();
    let mut skipped_files = Vec::new();
    let mut errors = Vec::new();

    // Helper to check if file is an .env file
    fn is_env_file(path: &Path) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with(".env"))
            .unwrap_or(false)
    }

    // Helper to copy a single file with optional transformation
    fn copy_file_with_transform(
        path: &Path,
        source: &Path,
        target: &Path,
        assignments: &[PortAssignment],
        copied_files: &mut Vec<String>,
        skipped_files: &mut Vec<String>,
        errors: &mut Vec<String>,
    ) {
        let relative = match path.strip_prefix(source) {
            Ok(rel) => rel,
            Err(_) => {
                errors.push(format!(
                    "Failed to calculate relative path: {}",
                    path.display()
                ));
                return;
            }
        };

        let target_file = target.join(relative);

        if target_file.exists() {
            skipped_files.push(relative.to_string_lossy().to_string());
            return;
        }

        if let Some(parent) = target_file.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    errors.push(format!(
                        "Failed to create directory {}: {}",
                        parent.display(),
                        e
                    ));
                    return;
                }
            }
        }

        // Transform file content based on file type
        if !assignments.is_empty() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    let transformed = if is_env_file(path) {
                        // Use specialized .env transformation (preserves variable names)
                        transform_env_content(&content, assignments).content
                    } else {
                        // Use generic port number replacement
                        transform_generic_content(&content, assignments)
                    };

                    if let Err(e) = fs::write(&target_file, transformed) {
                        errors.push(format!(
                            "Failed to write transformed file {}: {}",
                            target_file.display(),
                            e
                        ));
                        return;
                    }
                    copied_files.push(relative.to_string_lossy().to_string());
                }
                Err(e) => {
                    errors.push(format!("Failed to read file {}: {}", path.display(), e));
                }
            }
        } else {
            // No assignments, just copy
            match fs::copy(path, &target_file) {
                Ok(_) => {
                    copied_files.push(relative.to_string_lossy().to_string());
                }
                Err(e) => {
                    errors.push(format!(
                        "Failed to copy {} to {}: {}",
                        path.display(),
                        target_file.display(),
                        e
                    ));
                }
            }
        }
    }

    // Helper to recursively copy directory
    fn copy_directory_recursive(
        dir: &Path,
        source: &Path,
        target: &Path,
        assignments: &[PortAssignment],
        copied_files: &mut Vec<String>,
        skipped_files: &mut Vec<String>,
        errors: &mut Vec<String>,
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    copy_directory_recursive(
                        &path,
                        source,
                        target,
                        assignments,
                        copied_files,
                        skipped_files,
                        errors,
                    );
                } else if path.is_file() {
                    copy_file_with_transform(
                        &path,
                        source,
                        target,
                        assignments,
                        copied_files,
                        skipped_files,
                        errors,
                    );
                }
            }
        }
    }

    for pattern in patterns {
        let full_pattern = source.join(&pattern);
        let pattern_str = full_pattern.to_string_lossy().to_string();

        match glob(&pattern_str) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(path) => {
                            if path.is_dir() {
                                copy_directory_recursive(
                                    &path,
                                    source,
                                    target,
                                    &assignments,
                                    &mut copied_files,
                                    &mut skipped_files,
                                    &mut errors,
                                );
                            } else if path.is_file() {
                                copy_file_with_transform(
                                    &path,
                                    source,
                                    target,
                                    &assignments,
                                    &mut copied_files,
                                    &mut skipped_files,
                                    &mut errors,
                                );
                            }
                        }
                        Err(e) => {
                            errors.push(format!("Glob error for pattern '{}': {}", pattern, e));
                        }
                    }
                }
            }
            Err(e) => {
                errors.push(format!("Invalid glob pattern '{}': {}", pattern, e));
            }
        }
    }

    Ok(super::git_worktree::CopyResult {
        copied_files,
        skipped_files,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_detect_ports_in_env_file_basic() {
        let content = r#"
# Database config
DB_PORT=5432
PORT=3000
API_PORT=8080
NOT_A_PORT=hello
"#;

        let ports = detect_ports_in_env_file(content, ".env");
        assert_eq!(ports.len(), 3);

        assert_eq!(ports[0].variable_name, "DB_PORT");
        assert_eq!(ports[0].port_value, 5432);
        assert_eq!(ports[0].line_number, 3);

        assert_eq!(ports[1].variable_name, "PORT");
        assert_eq!(ports[1].port_value, 3000);
        assert_eq!(ports[1].line_number, 4);

        assert_eq!(ports[2].variable_name, "API_PORT");
        assert_eq!(ports[2].port_value, 8080);
        assert_eq!(ports[2].line_number, 5);
    }

    #[test]
    fn test_detect_ports_in_env_file_empty() {
        let content = "";
        let ports = detect_ports_in_env_file(content, ".env");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_env_file_comments_only() {
        let content = r#"
# This is a comment
# PORT=3000
"#;
        let ports = detect_ports_in_env_file(content, ".env");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_dockerfile() {
        let content = r#"
FROM node:18
WORKDIR /app
COPY . .
EXPOSE 3000
EXPOSE 8080
# EXPOSE 9999
CMD ["node", "index.js"]
"#;

        let ports = detect_ports_in_dockerfile(content, "Dockerfile");
        assert_eq!(ports.len(), 2);

        assert_eq!(ports[0].variable_name, "EXPOSE");
        assert_eq!(ports[0].port_value, 3000);
        assert_eq!(ports[0].line_number, 5);

        assert_eq!(ports[1].variable_name, "EXPOSE");
        assert_eq!(ports[1].port_value, 8080);
        assert_eq!(ports[1].line_number, 6);
    }

    #[test]
    fn test_detect_ports_in_compose() {
        let content = r#"
version: "3"
services:
  web:
    ports:
      - "3000:3000"
      - "8080:8080"
  db:
    ports:
      - "5432:5432"
"#;

        let ports = detect_ports_in_compose(content, "docker-compose.yml");
        assert_eq!(ports.len(), 3);

        assert_eq!(ports[0].variable_name, "COMPOSE:3000");
        assert_eq!(ports[0].port_value, 3000);
        assert_eq!(ports[1].variable_name, "COMPOSE:8080");
        assert_eq!(ports[1].port_value, 8080);
        assert_eq!(ports[2].variable_name, "COMPOSE:5432");
        assert_eq!(ports[2].port_value, 5432);
    }

    #[test]
    fn test_detect_ports_in_compose_single_quotes() {
        let content = r#"
version: "3"
services:
  db:
    ports:
      - '5433:5432'
  redis:
    ports:
      - '6380:6379'
"#;

        let ports = detect_ports_in_compose(content, "docker-compose.test.yml");
        assert_eq!(ports.len(), 2);

        assert_eq!(ports[0].variable_name, "COMPOSE:5433");
        assert_eq!(ports[0].port_value, 5433);
        assert_eq!(ports[1].variable_name, "COMPOSE:6380");
        assert_eq!(ports[1].port_value, 6380);
    }

    #[test]
    fn test_detect_ports_in_compose_no_quotes() {
        let content = r#"
services:
  web:
    ports:
      - 3000:3000
      - 8080:80
"#;

        let ports = detect_ports_in_compose(content, "docker-compose.yml");
        assert_eq!(ports.len(), 2);

        assert_eq!(ports[0].variable_name, "COMPOSE:3000");
        assert_eq!(ports[0].port_value, 3000);
        assert_eq!(ports[1].variable_name, "COMPOSE:8080");
        assert_eq!(ports[1].port_value, 8080);
    }

    #[test]
    fn test_allocate_ports_compose_deduplication() {
        let ports = vec![
            PortSource {
                file_path: "docker-compose.yml".to_string(),
                variable_name: "COMPOSE:5433".to_string(),
                port_value: 5433,
                line_number: 5,
            },
            PortSource {
                file_path: "docker-compose.dev.yml".to_string(),
                variable_name: "COMPOSE:5433".to_string(),
                port_value: 5433,
                line_number: 3,
            },
            PortSource {
                file_path: "docker-compose.yml".to_string(),
                variable_name: "COMPOSE:8080".to_string(),
                port_value: 8080,
                line_number: 8,
            },
        ];

        let result = allocate_ports(&ports, 20000).unwrap();
        assert_eq!(result.assignments.len(), 3);
        // Only 2 unique variable names, so next_port should be 20002
        assert_eq!(result.next_port, 20002);

        // Both COMPOSE:5433 entries should get the same assigned value
        assert_eq!(result.assignments[0].assigned_value, 20000);
        assert_eq!(result.assignments[1].assigned_value, 20000);

        // COMPOSE:8080 gets a different assigned value
        assert_eq!(result.assignments[2].assigned_value, 20001);
    }

    #[test]
    fn test_allocate_ports_basic() {
        let ports = vec![
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "PORT".to_string(),
                port_value: 3000,
                line_number: 1,
            },
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "DB_PORT".to_string(),
                port_value: 5432,
                line_number: 2,
            },
        ];

        let result = allocate_ports(&ports, 20000).unwrap();
        assert_eq!(result.assignments.len(), 2);
        assert_eq!(result.next_port, 20002);

        assert_eq!(result.assignments[0].variable_name, "PORT");
        assert_eq!(result.assignments[0].original_value, 3000);
        assert_eq!(result.assignments[0].assigned_value, 20000);

        assert_eq!(result.assignments[1].variable_name, "DB_PORT");
        assert_eq!(result.assignments[1].original_value, 5432);
        assert_eq!(result.assignments[1].assigned_value, 20001);
    }

    #[test]
    fn test_allocate_ports_duplicate_variable() {
        let ports = vec![
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "PORT".to_string(),
                port_value: 3000,
                line_number: 1,
            },
            PortSource {
                file_path: ".env.local".to_string(),
                variable_name: "PORT".to_string(),
                port_value: 3000,
                line_number: 1,
            },
        ];

        let result = allocate_ports(&ports, 20000).unwrap();
        assert_eq!(result.assignments.len(), 2);
        assert_eq!(result.next_port, 20001); // Only one unique port allocated

        // Both should get the same assigned value
        assert_eq!(result.assignments[0].assigned_value, 20000);
        assert_eq!(result.assignments[1].assigned_value, 20000);
    }

    #[test]
    fn test_allocate_ports_invalid_start() {
        let ports = vec![];
        let result = allocate_ports(&ports, 100);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be between"));
    }

    #[test]
    fn test_transform_env_content() {
        let content = r#"# Config
PORT=3000
DB_PORT=5432
API_URL=http://localhost:8080
"#;

        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 20000,
            },
            PortAssignment {
                variable_name: "DB_PORT".to_string(),
                original_value: 5432,
                assigned_value: 20001,
            },
        ];

        let result = transform_env_content(content, &assignments);

        assert!(result.content.contains("PORT=20000"));
        assert!(result.content.contains("DB_PORT=20001"));
        assert!(result.content.contains("API_URL=http://localhost:8080")); // unchanged
        assert!(result.content.contains("# Config")); // comment preserved

        assert_eq!(result.replacements.len(), 2);
    }

    #[test]
    fn test_transform_env_content_no_trailing_newline() {
        let content = "PORT=3000";

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 20000,
        }];

        let result = transform_env_content(content, &assignments);
        assert_eq!(result.content, "PORT=20000");
        assert!(!result.content.ends_with('\n'));
    }

    #[test]
    fn test_transform_generic_content_docker_compose() {
        let content = r#"services:
  web:
    ports:
      - "3000:3000"
      - "5432:5432"
  db:
    ports:
      - "6379:6379"
"#;

        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 20000,
            },
            PortAssignment {
                variable_name: "DB_PORT".to_string(),
                original_value: 5432,
                assigned_value: 20001,
            },
        ];

        let result = transform_generic_content(content, &assignments);

        // Both host and container port should be replaced
        assert!(result.contains("20000:20000"));
        assert!(result.contains("20001:20001"));
        // Unchanged ports should remain
        assert!(result.contains("6379:6379"));
    }

    #[test]
    fn test_transform_generic_content_json_config() {
        let content = r#"{
  "port": 3000,
  "database": {
    "port": 5432
  }
}"#;

        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 20000,
            },
            PortAssignment {
                variable_name: "DB_PORT".to_string(),
                original_value: 5432,
                assigned_value: 20001,
            },
        ];

        let result = transform_generic_content(content, &assignments);

        assert!(result.contains("\"port\": 20000"));
        assert!(result.contains("\"port\": 20001"));
    }

    #[test]
    fn test_transform_generic_content_dockerfile() {
        let content = r#"FROM node:18
WORKDIR /app
COPY . .
EXPOSE 3000
CMD ["node", "index.js"]
"#;

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 20000,
        }];

        let result = transform_generic_content(content, &assignments);

        assert!(result.contains("EXPOSE 20000"));
    }

    #[test]
    fn test_transform_generic_content_avoids_partial_match() {
        // Ensure 3000 doesn't match inside 30001
        let content = "PORT=30001\nOTHER_PORT=3000";

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 20000,
        }];

        let result = transform_generic_content(content, &assignments);

        // 30001 should remain unchanged (not become 200001)
        assert!(result.contains("30001"));
        // 3000 should be replaced
        assert!(result.contains("20000"));
        // Should not contain "200001"
        assert!(!result.contains("200001"));
    }

    #[test]
    fn test_scan_env_files_for_ports() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".env"), "PORT=3000\n").unwrap();
        fs::write(dir.path().join(".env.local"), "DB_PORT=5432\n").unwrap();
        fs::write(dir.path().join("config.txt"), "PORT=9999\n").unwrap(); // Should not be scanned

        let ports = scan_env_files_for_ports(dir.path());
        assert_eq!(ports.len(), 2);

        let vars: Vec<&str> = ports.iter().map(|p| p.variable_name.as_str()).collect();
        assert!(vars.contains(&"PORT"));
        assert!(vars.contains(&"DB_PORT"));
    }

    #[test]
    fn test_detect_all_ports() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".env"), "PORT=3000\n").unwrap();
        fs::write(dir.path().join("Dockerfile"), "EXPOSE 3000\n").unwrap();
        fs::write(
            dir.path().join("docker-compose.yml"),
            "services:\n  web:\n    ports:\n      - \"3000:3000\"\n",
        )
        .unwrap();

        let result = detect_all_ports(dir.path().to_string_lossy().to_string()).unwrap();

        assert_eq!(result.env_ports.len(), 1);
        assert_eq!(result.dockerfile_ports.len(), 1);
        assert_eq!(result.compose_ports.len(), 1);
    }

    #[test]
    fn test_detect_all_ports_nonexistent_dir() {
        let result = detect_all_ports("/nonexistent/path".to_string());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_copy_files_with_port_transformation() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create source files
        fs::write(source_dir.path().join(".env"), "PORT=3000\nDB_PORT=5432\n").unwrap();
        fs::write(source_dir.path().join(".env.local"), "API_PORT=8080\n").unwrap();
        fs::write(source_dir.path().join("config.json"), "{}\n").unwrap();

        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 20000,
            },
            PortAssignment {
                variable_name: "DB_PORT".to_string(),
                original_value: 5432,
                assigned_value: 20001,
            },
            PortAssignment {
                variable_name: "API_PORT".to_string(),
                original_value: 8080,
                assigned_value: 20002,
            },
        ];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 2);
        assert!(result.errors.is_empty());

        // Verify .env was transformed
        let env_content = fs::read_to_string(target_dir.path().join(".env")).unwrap();
        assert!(env_content.contains("PORT=20000"));
        assert!(env_content.contains("DB_PORT=20001"));

        // Verify .env.local was transformed
        let env_local_content = fs::read_to_string(target_dir.path().join(".env.local")).unwrap();
        assert!(env_local_content.contains("API_PORT=20002"));
    }

    #[test]
    fn test_copy_files_with_port_transformation_no_assignments() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(source_dir.path().join(".env"), "PORT=3000\n").unwrap();

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
            vec![], // No assignments
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);

        // Content should be unchanged
        let env_content = fs::read_to_string(target_dir.path().join(".env")).unwrap();
        assert!(env_content.contains("PORT=3000"));
    }

    #[test]
    fn test_port_source_serialization() {
        let source = PortSource {
            file_path: ".env".to_string(),
            variable_name: "PORT".to_string(),
            port_value: 3000,
            line_number: 1,
        };

        let json = serde_json::to_string(&source).unwrap();
        let parsed: PortSource = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.file_path, ".env");
        assert_eq!(parsed.variable_name, "PORT");
        assert_eq!(parsed.port_value, 3000);
        assert_eq!(parsed.line_number, 1);
    }

    #[test]
    fn test_detected_ports_serialization() {
        let detected = DetectedPorts {
            env_ports: vec![PortSource {
                file_path: ".env".to_string(),
                variable_name: "PORT".to_string(),
                port_value: 3000,
                line_number: 1,
            }],
            dockerfile_ports: vec![],
            compose_ports: vec![],
        };

        let json = serde_json::to_string(&detected).unwrap();
        let parsed: DetectedPorts = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.env_ports.len(), 1);
        assert!(parsed.dockerfile_ports.is_empty());
        assert!(parsed.compose_ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_url_variables() {
        let content = r#"
# Database URLs
REDIS_URL=redis://localhost:6379
DATABASE_URL=postgres://user:pass@localhost:5432/mydb
MONGO_URL=mongodb://localhost:27017/test
# Simple port
PORT=3000
"#;

        let ports = detect_ports_in_env_file(content, ".env");
        assert_eq!(ports.len(), 4);

        // Check URL-based ports
        assert_eq!(ports[0].variable_name, "REDIS_URL");
        assert_eq!(ports[0].port_value, 6379);

        assert_eq!(ports[1].variable_name, "DATABASE_URL");
        assert_eq!(ports[1].port_value, 5432);

        assert_eq!(ports[2].variable_name, "MONGO_URL");
        assert_eq!(ports[2].port_value, 27017);

        // Check simple port
        assert_eq!(ports[3].variable_name, "PORT");
        assert_eq!(ports[3].port_value, 3000);
    }

    #[test]
    fn test_transform_env_content_with_urls() {
        let content = r#"# Config
PORT=3000
REDIS_URL=redis://localhost:6379
DATABASE_URL=postgres://user:pass@localhost:5432/mydb
"#;

        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 20000,
            },
            PortAssignment {
                variable_name: "REDIS_URL".to_string(),
                original_value: 6379,
                assigned_value: 20001,
            },
            PortAssignment {
                variable_name: "DATABASE_URL".to_string(),
                original_value: 5432,
                assigned_value: 20002,
            },
        ];

        let result = transform_env_content(content, &assignments);

        assert!(result.content.contains("PORT=20000"));
        assert!(result.content.contains("REDIS_URL=redis://localhost:20001"));
        assert!(result.content.contains("DATABASE_URL=postgres://user:pass@localhost:20002/mydb"));
        assert!(result.content.contains("# Config")); // comment preserved

        assert_eq!(result.replacements.len(), 3);
    }

    #[test]
    fn test_scan_env_files_for_ports_subdirectories() {
        let dir = tempdir().unwrap();

        // Root .env
        fs::write(dir.path().join(".env"), "PORT=3000\n").unwrap();

        // Subdirectory .env files
        let sub1 = dir.path().join("packages/api");
        fs::create_dir_all(&sub1).unwrap();
        fs::write(sub1.join(".env"), "API_PORT=8080\n").unwrap();

        let sub2 = dir.path().join("services/worker");
        fs::create_dir_all(&sub2).unwrap();
        fs::write(sub2.join(".env.local"), "WORKER_PORT=9090\n").unwrap();

        // Non-.env file should not be scanned
        fs::write(sub1.join("config.txt"), "PORT=9999\n").unwrap();

        let ports = scan_env_files_for_ports(dir.path());
        assert_eq!(ports.len(), 3);

        let vars: Vec<&str> = ports.iter().map(|p| p.variable_name.as_str()).collect();
        assert!(vars.contains(&"PORT"));
        assert!(vars.contains(&"API_PORT"));
        assert!(vars.contains(&"WORKER_PORT"));
    }

    #[test]
    fn test_copy_files_with_port_transformation_subdirectories() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Root .env
        fs::write(source_dir.path().join(".env"), "PORT=3000\n").unwrap();

        // Subdirectory .env
        let sub = source_dir.path().join("packages/api");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join(".env"), "API_PORT=8080\n").unwrap();

        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 20000,
            },
            PortAssignment {
                variable_name: "API_PORT".to_string(),
                original_value: 8080,
                assigned_value: 20001,
            },
        ];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["**/.env*".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 2);
        assert!(result.errors.is_empty());

        // Verify root .env was transformed
        let root_env = fs::read_to_string(target_dir.path().join(".env")).unwrap();
        assert!(root_env.contains("PORT=20000"));

        // Verify subdirectory .env was transformed
        let sub_env = fs::read_to_string(target_dir.path().join("packages/api/.env")).unwrap();
        assert!(sub_env.contains("API_PORT=20001"));
    }

    #[test]
    fn test_scan_compose_for_ports_subdirectories() {
        let dir = tempdir().unwrap();

        // Root docker-compose.yml
        fs::write(
            dir.path().join("docker-compose.yml"),
            "services:\n  web:\n    ports:\n      - \"3000:3000\"\n",
        )
        .unwrap();

        // Subdirectory docker-compose.yml
        let sub = dir.path().join("services/backend");
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            sub.join("docker-compose.yml"),
            "services:\n  api:\n    ports:\n      - \"8080:8080\"\n",
        )
        .unwrap();

        let ports = scan_compose_for_ports(dir.path());
        assert_eq!(ports.len(), 2);

        let values: Vec<u16> = ports.iter().map(|p| p.port_value).collect();
        assert!(values.contains(&3000));
        assert!(values.contains(&8080));
    }

    #[test]
    fn test_scan_compose_for_ports_dotted_filenames() {
        let dir = tempdir().unwrap();

        // Root docker-compose.yml
        fs::write(
            dir.path().join("docker-compose.yml"),
            "services:\n  web:\n    ports:\n      - \"3000:3000\"\n",
        )
        .unwrap();

        // docker-compose.test.yml (dotted variant)
        let sub = dir.path().join("backend");
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            sub.join("docker-compose.test.yml"),
            "services:\n  db:\n    ports:\n      - \"5433:5432\"\n",
        )
        .unwrap();

        // docker-compose.override.yaml (dotted variant with .yaml)
        fs::write(
            dir.path().join("docker-compose.override.yaml"),
            "services:\n  web:\n    ports:\n      - \"8080:8080\"\n",
        )
        .unwrap();

        let ports = scan_compose_for_ports(dir.path());
        assert_eq!(ports.len(), 3);

        let values: Vec<u16> = ports.iter().map(|p| p.port_value).collect();
        assert!(values.contains(&3000));
        assert!(values.contains(&5433));
        assert!(values.contains(&8080));
    }

    #[test]
    fn test_scan_dockerfile_for_ports_subdirectories() {
        let dir = tempdir().unwrap();

        // Root Dockerfile
        fs::write(dir.path().join("Dockerfile"), "EXPOSE 3000\n").unwrap();

        // Subdirectory Dockerfile
        let sub = dir.path().join("services/api");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("Dockerfile"), "EXPOSE 8080\n").unwrap();

        let ports = scan_dockerfile_for_ports(dir.path());
        assert_eq!(ports.len(), 2);

        let values: Vec<u16> = ports.iter().map(|p| p.port_value).collect();
        assert!(values.contains(&3000));
        assert!(values.contains(&8080));
    }
}
