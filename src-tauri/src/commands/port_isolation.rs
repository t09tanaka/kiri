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
const COMPOSE_PORT_PATTERN: &str = r#"^\s*-\s*"?(\d+):(\d+)"?"#;

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
pub struct CustomPortRule {
    pub id: String,
    pub file_pattern: String,
    pub search_pattern: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRuleReplacement {
    pub file_path: String,
    pub original_value: u16,
    pub new_value: u16,
    pub line_number: u32,
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
                        variable_name: "ports".to_string(),
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
    let pattern = dir.join(".env*").to_string_lossy().to_string();

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

/// Scan a directory for Dockerfile and detect ports
pub fn scan_dockerfile_for_ports(dir: &Path) -> Vec<PortSource> {
    let mut all_ports = Vec::new();

    // Check for Dockerfile
    let dockerfile_path = dir.join("Dockerfile");
    if dockerfile_path.is_file() {
        if let Ok(content) = fs::read_to_string(&dockerfile_path) {
            let file_path = dockerfile_path.to_string_lossy().to_string();
            let ports = detect_ports_in_dockerfile(&content, &file_path);
            all_ports.extend(ports);
        }
    }

    // Also check for Dockerfile.* variants
    let pattern = dir.join("Dockerfile.*").to_string_lossy().to_string();
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

    all_ports
}

/// Scan a directory for docker-compose files and detect ports
pub fn scan_compose_for_ports(dir: &Path) -> Vec<PortSource> {
    let mut all_ports = Vec::new();

    let compose_files = [
        "docker-compose.yml",
        "docker-compose.yaml",
        "compose.yml",
        "compose.yaml",
    ];

    for filename in compose_files {
        let compose_path = dir.join(filename);
        if compose_path.is_file() {
            if let Ok(content) = fs::read_to_string(&compose_path) {
                let file_path = compose_path.to_string_lossy().to_string();
                let ports = detect_ports_in_compose(&content, &file_path);
                all_ports.extend(ports);
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

        // Check if this is an .env file that needs transformation
        if is_env_file(path) && !assignments.is_empty() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    let transformed = transform_env_content(&content, assignments);
                    if let Err(e) = fs::write(&target_file, transformed.content) {
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
            // Regular copy
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

/// Apply custom rules to files in source directory and write to target
pub fn apply_custom_rules(
    source_path: String,
    target_path: String,
    rules: Vec<CustomPortRule>,
    port_offset: u16,
) -> Result<Vec<CustomRuleReplacement>, String> {
    let source = Path::new(&source_path);
    let target = Path::new(&target_path);

    if !source.exists() {
        return Err(format!("Source path does not exist: {}", source_path));
    }

    if !target.exists() {
        return Err(format!("Target path does not exist: {}", target_path));
    }

    let mut all_replacements = Vec::new();

    for rule in rules {
        if !rule.enabled {
            continue;
        }

        let pattern = source.join(&rule.file_pattern).to_string_lossy().to_string();
        let search_re = match Regex::new(&rule.search_pattern) {
            Ok(re) => re,
            Err(e) => {
                return Err(format!(
                    "Invalid regex pattern '{}': {}",
                    rule.search_pattern, e
                ));
            }
        };

        if let Ok(entries) = glob(&pattern) {
            for entry in entries.flatten() {
                if !entry.is_file() {
                    continue;
                }

                let content = match fs::read_to_string(&entry) {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                let relative = entry.strip_prefix(source).unwrap_or(&entry);
                let target_file = target.join(relative);

                // Create parent directories
                if let Some(parent) = target_file.parent() {
                    if !parent.exists() {
                        let _ = fs::create_dir_all(parent);
                    }
                }

                let mut new_content = String::new();
                let mut replacements_for_file = Vec::new();

                for (line_num, line) in content.lines().enumerate() {
                    if let Some(caps) = search_re.captures(line) {
                        if let Some(port_match) = caps.get(1) {
                            if let Ok(original_port) = port_match.as_str().parse::<u16>() {
                                let new_port = original_port.saturating_add(port_offset);
                                let new_line = search_re.replace(line, |caps: &regex::Captures| {
                                    let full_match = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                                    let port_match = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                                    full_match.replace(port_match, &new_port.to_string())
                                });
                                new_content.push_str(&new_line);
                                replacements_for_file.push(CustomRuleReplacement {
                                    file_path: relative.to_string_lossy().to_string(),
                                    original_value: original_port,
                                    new_value: new_port,
                                    line_number: (line_num + 1) as u32,
                                });
                            } else {
                                new_content.push_str(line);
                            }
                        } else {
                            new_content.push_str(line);
                        }
                    } else {
                        new_content.push_str(line);
                    }
                    new_content.push('\n');
                }

                // Remove trailing newline if original didn't have one
                if !content.ends_with('\n') && new_content.ends_with('\n') {
                    new_content.pop();
                }

                // Write the transformed file
                if !replacements_for_file.is_empty() {
                    if let Err(e) = fs::write(&target_file, &new_content) {
                        return Err(format!(
                            "Failed to write file {}: {}",
                            target_file.display(),
                            e
                        ));
                    }
                    all_replacements.extend(replacements_for_file);
                }
            }
        }
    }

    Ok(all_replacements)
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

        assert_eq!(ports[0].port_value, 3000);
        assert_eq!(ports[1].port_value, 8080);
        assert_eq!(ports[2].port_value, 5432);
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
    fn test_apply_custom_rules_basic() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a JSON config file
        fs::write(
            source_dir.path().join("config.json"),
            r#"{"port": 3000, "name": "test"}"#,
        )
        .unwrap();

        let rules = vec![CustomPortRule {
            id: "json-port".to_string(),
            file_pattern: "config.json".to_string(),
            search_pattern: r#""port":\s*(\d+)"#.to_string(),
            enabled: true,
        }];

        let result = apply_custom_rules(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            rules,
            17000, // offset
        )
        .unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].original_value, 3000);
        assert_eq!(result[0].new_value, 20000); // 3000 + 17000

        // Verify file was written
        let content = fs::read_to_string(target_dir.path().join("config.json")).unwrap();
        assert!(content.contains("20000"));
    }

    #[test]
    fn test_apply_custom_rules_disabled() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(
            source_dir.path().join("config.json"),
            r#"{"port": 3000}"#,
        )
        .unwrap();

        let rules = vec![CustomPortRule {
            id: "json-port".to_string(),
            file_pattern: "config.json".to_string(),
            search_pattern: r#""port":\s*(\d+)"#.to_string(),
            enabled: false, // disabled
        }];

        let result = apply_custom_rules(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            rules,
            17000,
        )
        .unwrap();

        assert!(result.is_empty());
        // File should not be created
        assert!(!target_dir.path().join("config.json").exists());
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
}
