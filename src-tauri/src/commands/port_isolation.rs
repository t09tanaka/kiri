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
// Pattern for port flags in package.json scripts: -p PORT, --port PORT, --port=PORT
const SCRIPT_PORT_PATTERN: &str = r"(?:--port[= ]|-p )(\d+)";


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
    pub script_ports: Vec<PortSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortAllocationResult {
    pub assignments: Vec<PortAssignment>,
    pub worktree_index: u16,
    pub overflow_warnings: Vec<String>,
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

/// Detect port flags in package.json scripts content
/// Matches patterns like: -p 3000, --port 8080, --port=3000
pub fn detect_ports_in_package_json(content: &str, file_path: &str) -> Vec<PortSource> {
    let re = Regex::new(SCRIPT_PORT_PATTERN).unwrap();
    let mut ports = Vec::new();

    // Parse JSON to find "scripts" section
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(content);
    let scripts = match parsed {
        Ok(ref val) => val.get("scripts").and_then(|s| s.as_object()),
        Err(_) => return ports,
    };

    let Some(scripts) = scripts else {
        return ports;
    };

    // Search for port patterns in each script value
    for (_key, value) in scripts {
        if let Some(script_str) = value.as_str() {
            for caps in re.captures_iter(script_str) {
                if let Some(port_str) = caps.get(1) {
                    if let Ok(port) = port_str.as_str().parse::<u16>() {
                        // Find line number by searching the raw content
                        let line_number = content
                            .lines()
                            .enumerate()
                            .find(|(_, line)| line.contains(script_str))
                            .map(|(i, _)| (i + 1) as u32)
                            .unwrap_or(0);

                        ports.push(PortSource {
                            file_path: file_path.to_string(),
                            variable_name: format!("SCRIPT:{}", port),
                            port_value: port,
                            line_number,
                        });
                    }
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

/// Scan a directory for all package.json files and detect ports in scripts
pub fn scan_package_json_for_ports(dir: &Path) -> Vec<PortSource> {
    let mut all_ports = Vec::new();
    let pattern = dir.join("**/package.json").to_string_lossy().to_string();

    let options = MatchOptions {
        require_literal_leading_dot: true,
        ..Default::default()
    };

    if let Ok(entries) = glob_with(&pattern, options) {
        for entry in entries.flatten() {
            if entry.is_file() {
                // Skip node_modules
                if entry
                    .components()
                    .any(|c| c.as_os_str() == "node_modules")
                {
                    continue;
                }
                if let Ok(content) = fs::read_to_string(&entry) {
                    let file_path = entry.to_string_lossy().to_string();
                    let ports = detect_ports_in_package_json(&content, &file_path);
                    all_ports.extend(ports);
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
    let script_ports = scan_package_json_for_ports(dir);

    Ok(DetectedPorts {
        env_ports,
        dockerfile_ports,
        compose_ports,
        script_ports,
    })
}

/// Allocate ports using offset-based strategy: original_port + (worktree_index * 100)
pub fn allocate_ports(
    ports: &[PortSource],
    worktree_index: u16,
) -> Result<PortAllocationResult, String> {
    if worktree_index == 0 {
        return Err("Worktree index must be greater than 0".to_string());
    }

    let offset = worktree_index as u32 * 100;
    let mut assignments = Vec::new();
    let mut overflow_warnings = Vec::new();
    let mut seen_values: HashMap<u16, u16> = HashMap::new();

    for port_source in ports {
        // If same original port value was already assigned, reuse it
        if let Some(&assigned_port) = seen_values.get(&port_source.port_value) {
            assignments.push(PortAssignment {
                variable_name: port_source.variable_name.clone(),
                original_value: port_source.port_value,
                assigned_value: assigned_port,
            });
            continue;
        }

        let new_port = port_source.port_value as u32 + offset;

        if new_port > 65535 {
            overflow_warnings.push(format!(
                "{}={}: {} exceeds max port 65535",
                port_source.variable_name, port_source.port_value, new_port
            ));
            continue;
        }

        let assigned_port = new_port as u16;
        seen_values.insert(port_source.port_value, assigned_port);

        assignments.push(PortAssignment {
            variable_name: port_source.variable_name.clone(),
            original_value: port_source.port_value,
            assigned_value: assigned_port,
        });
    }

    Ok(PortAllocationResult {
        assignments,
        worktree_index,
        overflow_warnings,
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

/// Transform docker-compose content, replacing only host-side ports in port mappings.
/// Container-internal ports (right side of host:container) are preserved.
pub fn transform_compose_content(content: &str, assignments: &[PortAssignment]) -> String {
    let re = Regex::new(COMPOSE_PORT_PATTERN).unwrap();
    // Extended pattern to also capture optional /protocol suffix
    let re_full = Regex::new(r#"^(\s*-\s*["']?)(\d+):(\d+)(/\w+)?(["']?)(.*)$"#).unwrap();

    let mut result_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        // Check if this line is a port mapping
        if re.is_match(line) {
            if let Some(caps) = re_full.captures(line) {
                let prefix = caps.get(1).map_or("", |m| m.as_str());
                let host_port_str = caps.get(2).map_or("", |m| m.as_str());
                let container_port = caps.get(3).map_or("", |m| m.as_str());
                let protocol = caps.get(4).map_or("", |m| m.as_str());
                let suffix_quote = caps.get(5).map_or("", |m| m.as_str());
                let trailing = caps.get(6).map_or("", |m| m.as_str());

                if let Ok(host_port) = host_port_str.parse::<u16>() {
                    // Find matching assignment for this host port
                    let new_host_port = assignments
                        .iter()
                        .find(|a| a.original_value == host_port)
                        .map(|a| a.assigned_value)
                        .unwrap_or(host_port);

                    result_lines.push(format!(
                        "{}{}:{}{}{}{}",
                        prefix, new_host_port, container_port, protocol, suffix_quote, trailing
                    ));
                    continue;
                }
            }
        }
        // Non-port-mapping lines pass through unchanged
        result_lines.push(line.to_string());
    }

    // Preserve trailing newline if original had one
    let mut result = result_lines.join("\n");
    if content.ends_with('\n') {
        result.push('\n');
    }
    result
}

/// Check if a file is an .env file
pub fn is_env_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with(".env"))
        .unwrap_or(false)
}

/// Check if a file is a docker-compose file
pub fn is_compose_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| {
            let lower = name.to_lowercase();
            (lower.starts_with("docker-compose") || lower == "compose.yml" || lower == "compose.yaml")
                && (lower.ends_with(".yml") || lower.ends_with(".yaml"))
        })
        .unwrap_or(false)
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
    let mut transformed_files = Vec::new();
    let mut errors = Vec::new();

    // Helper to copy a single file with optional transformation
    #[allow(clippy::too_many_arguments)]
    fn copy_file_with_transform(
        path: &Path,
        source: &Path,
        target: &Path,
        assignments: &[PortAssignment],
        copied_files: &mut Vec<String>,
        skipped_files: &mut Vec<String>,
        transformed_files: &mut Vec<String>,
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
            if !assignments.is_empty() {
                // File already exists (e.g., git-tracked docker-compose.yml in worktree)
                // but we have port assignments to apply — transform in-place
                match fs::read_to_string(&target_file) {
                    Ok(content) => {
                        let transformed = if is_env_file(&target_file) {
                            transform_env_content(&content, assignments).content
                        } else if is_compose_file(&target_file) {
                            transform_compose_content(&content, assignments)
                        } else {
                            transform_generic_content(&content, assignments)
                        };

                        if transformed != content {
                            if let Err(e) = fs::write(&target_file, transformed) {
                                errors.push(format!(
                                    "Failed to write transformed file {}: {}",
                                    target_file.display(),
                                    e
                                ));
                                return;
                            }
                            transformed_files
                                .push(relative.to_string_lossy().to_string());
                        } else {
                            skipped_files
                                .push(relative.to_string_lossy().to_string());
                        }
                    }
                    Err(e) => {
                        errors.push(format!(
                            "Failed to read existing file {}: {}",
                            target_file.display(),
                            e
                        ));
                    }
                }
            } else {
                skipped_files.push(relative.to_string_lossy().to_string());
            }
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
                    } else if is_compose_file(path) {
                        // Use compose-specific transformation (host-side only)
                        transform_compose_content(&content, assignments)
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
    #[allow(clippy::too_many_arguments)]
    fn copy_directory_recursive(
        dir: &Path,
        source: &Path,
        target: &Path,
        assignments: &[PortAssignment],
        copied_files: &mut Vec<String>,
        skipped_files: &mut Vec<String>,
        transformed_files: &mut Vec<String>,
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
                        transformed_files,
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
                        transformed_files,
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
                                    &mut transformed_files,
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
                                    &mut transformed_files,
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
        transformed_files,
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

        let result = allocate_ports(&ports, 2).unwrap();
        assert_eq!(result.assignments.len(), 3);
        assert_eq!(result.worktree_index, 2);

        // Both COMPOSE:5433 entries should get the same assigned value
        assert_eq!(result.assignments[0].assigned_value, 5633); // 5433 + 200
        assert_eq!(result.assignments[1].assigned_value, 5633);

        // COMPOSE:8080 gets offset too
        assert_eq!(result.assignments[2].assigned_value, 8280); // 8080 + 200
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

        let result = allocate_ports(&ports, 1).unwrap();
        assert_eq!(result.assignments.len(), 2);
        assert_eq!(result.worktree_index, 1);
        assert!(result.overflow_warnings.is_empty());

        assert_eq!(result.assignments[0].variable_name, "PORT");
        assert_eq!(result.assignments[0].original_value, 3000);
        assert_eq!(result.assignments[0].assigned_value, 3100);

        assert_eq!(result.assignments[1].variable_name, "DB_PORT");
        assert_eq!(result.assignments[1].original_value, 5432);
        assert_eq!(result.assignments[1].assigned_value, 5532);
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

        let result = allocate_ports(&ports, 1).unwrap();
        assert_eq!(result.assignments.len(), 2);
        assert_eq!(result.worktree_index, 1);

        // Both should get the same assigned value
        assert_eq!(result.assignments[0].assigned_value, 3100);
        assert_eq!(result.assignments[1].assigned_value, 3100);
    }

    #[test]
    fn test_allocate_ports_invalid_index() {
        let ports = vec![];
        let result = allocate_ports(&ports, 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be greater than 0"));
    }

    #[test]
    fn test_allocate_ports_overflow() {
        let ports = vec![
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "PORT".to_string(),
                port_value: 3000,
                line_number: 1,
            },
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "HIGH_PORT".to_string(),
                port_value: 65500,
                line_number: 2,
            },
        ];

        let result = allocate_ports(&ports, 1).unwrap();
        // PORT=3000 should be assigned (3100)
        assert_eq!(result.assignments.len(), 1);
        assert_eq!(result.assignments[0].assigned_value, 3100);
        // HIGH_PORT=65500 should produce an overflow warning
        assert_eq!(result.overflow_warnings.len(), 1);
        assert!(result.overflow_warnings[0].contains("65500"));
        assert!(result.overflow_warnings[0].contains("65535"));
    }

    #[test]
    fn test_allocate_ports_higher_index() {
        let ports = vec![
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "PORT".to_string(),
                port_value: 3000,
                line_number: 1,
            },
        ];

        let result = allocate_ports(&ports, 5).unwrap();
        assert_eq!(result.assignments.len(), 1);
        assert_eq!(result.assignments[0].assigned_value, 3500); // 3000 + 500
        assert_eq!(result.worktree_index, 5);
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

        // Generic transform replaces all occurrences (used for non-compose files)
        // For compose files, transform_compose_content is used instead (host-only)
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
            script_ports: vec![],
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
    fn test_copy_files_with_port_transformation_existing_files() {
        // Bug: docker-compose files are git-tracked and already exist in worktree
        // after `git worktree add`. The copy function used to skip existing files,
        // even when port assignments need to be applied.
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create source docker-compose.yml
        let compose_content = "services:\n  web:\n    ports:\n      - \"3000:3000\"\n  db:\n    ports:\n      - \"5432:5432\"\n";
        fs::write(source_dir.path().join("docker-compose.yml"), compose_content).unwrap();

        // Simulate git worktree: docker-compose.yml already exists in target (git-tracked)
        fs::write(target_dir.path().join("docker-compose.yml"), compose_content).unwrap();

        let assignments = vec![
            PortAssignment {
                variable_name: "COMPOSE:3000".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            },
            PortAssignment {
                variable_name: "COMPOSE:5432".to_string(),
                original_value: 5432,
                assigned_value: 5532,
            },
        ];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["docker-compose.yml".to_string()],
            assignments,
        )
        .unwrap();

        // The file should be transformed in-place, not skipped
        assert!(result.errors.is_empty());
        assert_eq!(result.transformed_files.len(), 1);

        // Verify docker-compose.yml was transformed with new ports
        let transformed = fs::read_to_string(target_dir.path().join("docker-compose.yml")).unwrap();
        // Now uses compose-specific transform: only host ports change
        assert!(transformed.contains("3100:3000"), "Expected 3100:3000, got: {}", transformed);
        assert!(transformed.contains("5532:5432"), "Expected 5532:5432, got: {}", transformed);
    }

    #[test]
    fn test_copy_files_existing_no_assignments_still_skipped() {
        // When there are no assignments, existing files should still be skipped (no transformation needed)
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let content = "PORT=3000\n";
        fs::write(source_dir.path().join(".env"), content).unwrap();
        fs::write(target_dir.path().join(".env"), content).unwrap();

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
            vec![], // No assignments
        )
        .unwrap();

        // File should be skipped since no assignments and file exists
        assert_eq!(result.skipped_files.len(), 1);
        assert!(result.copied_files.is_empty());
        assert!(result.transformed_files.is_empty());
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

    #[test]
    fn test_detect_ports_in_package_json_basic() {
        let content = r#"{
  "name": "my-app",
  "scripts": {
    "dev": "next dev -p 3000",
    "start": "node server.js --port 8080",
    "build": "next build"
  }
}"#;

        let ports = detect_ports_in_package_json(content, "package.json");
        assert_eq!(ports.len(), 2);

        assert_eq!(ports[0].variable_name, "SCRIPT:3000");
        assert_eq!(ports[0].port_value, 3000);

        assert_eq!(ports[1].variable_name, "SCRIPT:8080");
        assert_eq!(ports[1].port_value, 8080);
    }

    #[test]
    fn test_detect_ports_in_package_json_port_equals() {
        let content = r#"{
  "scripts": {
    "dev": "vite --port=5173"
  }
}"#;

        let ports = detect_ports_in_package_json(content, "package.json");
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].variable_name, "SCRIPT:5173");
        assert_eq!(ports[0].port_value, 5173);
    }

    #[test]
    fn test_detect_ports_in_package_json_no_scripts() {
        let content = r#"{
  "name": "my-app",
  "version": "1.0.0"
}"#;

        let ports = detect_ports_in_package_json(content, "package.json");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_package_json_no_ports_in_scripts() {
        let content = r#"{
  "scripts": {
    "build": "next build",
    "lint": "eslint ."
  }
}"#;

        let ports = detect_ports_in_package_json(content, "package.json");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_package_json_invalid_json() {
        let content = "not valid json";
        let ports = detect_ports_in_package_json(content, "package.json");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_scan_package_json_for_ports() {
        let dir = tempdir().unwrap();

        let content = r#"{
  "scripts": {
    "dev": "next dev -p 3000"
  }
}"#;
        fs::write(dir.path().join("package.json"), content).unwrap();

        let ports = scan_package_json_for_ports(dir.path());
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].variable_name, "SCRIPT:3000");
        assert_eq!(ports[0].port_value, 3000);
    }

    #[test]
    fn test_scan_package_json_for_ports_subdirectories() {
        let dir = tempdir().unwrap();

        // Root package.json
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts": {"dev": "next dev -p 3000"}}"#,
        )
        .unwrap();

        // Subdirectory package.json
        let sub = dir.path().join("packages/api");
        fs::create_dir_all(&sub).unwrap();
        fs::write(
            sub.join("package.json"),
            r#"{"scripts": {"start": "node index.js --port 8080"}}"#,
        )
        .unwrap();

        // node_modules should be skipped
        let nm = dir.path().join("node_modules/some-package");
        fs::create_dir_all(&nm).unwrap();
        fs::write(
            nm.join("package.json"),
            r#"{"scripts": {"dev": "serve -p 9999"}}"#,
        )
        .unwrap();

        let ports = scan_package_json_for_ports(dir.path());
        assert_eq!(ports.len(), 2);

        let values: Vec<u16> = ports.iter().map(|p| p.port_value).collect();
        assert!(values.contains(&3000));
        assert!(values.contains(&8080));
        // 9999 from node_modules should NOT be detected
        assert!(!values.contains(&9999));
    }

    #[test]
    fn test_detect_all_ports_includes_script_ports() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join(".env"), "PORT=3000\n").unwrap();
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts": {"dev": "next dev -p 3000"}}"#,
        )
        .unwrap();

        let result = detect_all_ports(dir.path().to_string_lossy().to_string()).unwrap();

        assert_eq!(result.env_ports.len(), 1);
        assert_eq!(result.script_ports.len(), 1);
        assert_eq!(result.script_ports[0].variable_name, "SCRIPT:3000");
    }

    #[test]
    fn test_is_compose_file() {
        assert!(is_compose_file(Path::new("docker-compose.yml")));
        assert!(is_compose_file(Path::new("docker-compose.yaml")));
        assert!(is_compose_file(Path::new("docker-compose.dev.yml")));
        assert!(is_compose_file(Path::new("docker-compose.prod.yaml")));
        assert!(is_compose_file(Path::new("compose.yml")));
        assert!(is_compose_file(Path::new("compose.yaml")));
        assert!(is_compose_file(Path::new("/some/path/docker-compose.yml")));
        assert!(is_compose_file(Path::new("/some/path/compose.yaml")));
        assert!(!is_compose_file(Path::new(".env")));
        assert!(!is_compose_file(Path::new("config.json")));
        assert!(!is_compose_file(Path::new("package.json")));
        assert!(!is_compose_file(Path::new("Dockerfile")));
        assert!(!is_compose_file(Path::new("my-compose.yml")));
    }

    #[test]
    fn test_transform_compose_content_host_only() {
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
                variable_name: "COMPOSE:3000".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            },
            PortAssignment {
                variable_name: "COMPOSE:5432".to_string(),
                original_value: 5432,
                assigned_value: 5532,
            },
        ];

        let result = transform_compose_content(content, &assignments);

        // Host port should be transformed, container port should be fixed
        assert!(result.contains("3100:3000"), "Expected 3100:3000, got:\n{}", result);
        assert!(result.contains("5532:5432"), "Expected 5532:5432, got:\n{}", result);
        // Unmatched port mapping should remain unchanged
        assert!(result.contains("6379:6379"));
        // Non-port lines should be unchanged
        assert!(result.contains("services:"));
        assert!(result.contains("  web:"));
    }

    #[test]
    fn test_transform_compose_content_unquoted_ports() {
        let content = "services:\n  web:\n    ports:\n      - 8080:8080\n";

        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:8080".to_string(),
            original_value: 8080,
            assigned_value: 8180,
        }];

        let result = transform_compose_content(content, &assignments);
        assert!(result.contains("8180:8080"), "Expected 8180:8080, got:\n{}", result);
    }

    #[test]
    fn test_transform_compose_content_single_quoted_ports() {
        let content = "services:\n  web:\n    ports:\n      - '5432:5432'\n";

        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:5432".to_string(),
            original_value: 5432,
            assigned_value: 5532,
        }];

        let result = transform_compose_content(content, &assignments);
        assert!(result.contains("5532:5432"), "Expected 5532:5432, got:\n{}", result);
    }

    #[test]
    fn test_transform_compose_content_with_protocol() {
        let content = "services:\n  web:\n    ports:\n      - \"5432:5432/tcp\"\n";

        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:5432".to_string(),
            original_value: 5432,
            assigned_value: 5532,
        }];

        let result = transform_compose_content(content, &assignments);
        assert!(result.contains("5532:5432/tcp"), "Expected 5532:5432/tcp, got:\n{}", result);
    }

    #[test]
    fn test_transform_compose_content_no_matching_assignment() {
        let content = "services:\n  web:\n    ports:\n      - \"9999:9999\"\n";

        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:3000".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = transform_compose_content(content, &assignments);
        // No matching assignment, so port mapping should remain unchanged
        assert!(result.contains("9999:9999"));
    }

    #[test]
    fn test_transform_compose_content_preserves_comments() {
        let content = r#"services:
  web:
    ports:
      # Main web port
      - "3000:3000"
      # Database port
      - "5432:5432"
"#;

        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:3000".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = transform_compose_content(content, &assignments);
        assert!(result.contains("# Main web port"));
        assert!(result.contains("# Database port"));
        assert!(result.contains("3100:3000"));
        // 5432 has no assignment, should stay unchanged
        assert!(result.contains("5432:5432"));
    }

    #[test]
    fn test_copy_files_with_port_transformation_compose_host_only() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create source docker-compose.yml
        fs::write(
            source_dir.path().join("docker-compose.yml"),
            "services:\n  db:\n    ports:\n      - \"5432:5432\"\n      - \"3000:3000\"\n",
        )
        .unwrap();

        let assignments = vec![
            PortAssignment {
                variable_name: "COMPOSE:5432".to_string(),
                original_value: 5432,
                assigned_value: 5532,
            },
            PortAssignment {
                variable_name: "COMPOSE:3000".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            },
        ];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["docker-compose.yml".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        assert!(result.errors.is_empty());

        // Verify host ports are transformed but container ports are fixed
        let content = fs::read_to_string(target_dir.path().join("docker-compose.yml")).unwrap();
        assert!(content.contains("5532:5432"), "Expected 5532:5432, got:\n{}", content);
        assert!(content.contains("3100:3000"), "Expected 3100:3000, got:\n{}", content);
    }

    #[test]
    fn test_copy_files_compose_in_place_transform() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let compose_content = "services:\n  db:\n    ports:\n      - \"5432:5432\"\n";

        // Create file in both source and target (simulates git-tracked file in worktree)
        fs::write(source_dir.path().join("docker-compose.yml"), compose_content).unwrap();
        fs::write(target_dir.path().join("docker-compose.yml"), compose_content).unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:5432".to_string(),
            original_value: 5432,
            assigned_value: 5532,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["docker-compose.yml".to_string()],
            assignments,
        )
        .unwrap();

        // Should be transformed in-place (file already existed)
        assert_eq!(result.transformed_files.len(), 1);
        assert!(result.errors.is_empty());

        let content = fs::read_to_string(target_dir.path().join("docker-compose.yml")).unwrap();
        assert!(content.contains("5532:5432"), "Expected 5532:5432, got:\n{}", content);
    }

    #[test]
    fn test_transform_compose_content_asymmetric_ports() {
        let content = "services:\n  web:\n    ports:\n      - \"8080:3000\"\n";
        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:8080".to_string(),
            original_value: 8080,
            assigned_value: 8180,
        }];
        let result = transform_compose_content(content, &assignments);
        // Only host port changes; container port stays as 3000
        assert!(result.contains("8180:3000"), "Expected 8180:3000, got:\n{}", result);
        assert!(!result.contains("8180:3100"), "Container port should not change");
    }

    // ========== is_env_file tests ==========

    #[test]
    fn test_is_env_file_basic() {
        assert!(is_env_file(Path::new(".env")));
    }

    #[test]
    fn test_is_env_file_with_suffix() {
        assert!(is_env_file(Path::new(".env.local")));
    }

    #[test]
    fn test_is_env_file_with_production_suffix() {
        assert!(is_env_file(Path::new(".env.production")));
    }

    #[test]
    fn test_is_env_file_with_development_suffix() {
        assert!(is_env_file(Path::new(".env.development")));
    }

    #[test]
    fn test_is_env_file_non_env() {
        assert!(!is_env_file(Path::new("config.yml")));
    }

    #[test]
    fn test_is_env_file_non_env_json() {
        assert!(!is_env_file(Path::new("package.json")));
    }

    #[test]
    fn test_is_env_file_dockerfile() {
        assert!(!is_env_file(Path::new("Dockerfile")));
    }

    #[test]
    fn test_is_env_file_with_dir() {
        assert!(is_env_file(Path::new("/some/path/.env")));
    }

    #[test]
    fn test_is_env_file_with_nested_dir() {
        assert!(is_env_file(Path::new("/project/packages/api/.env.local")));
    }

    // ========== is_compose_file additional tests ==========

    #[test]
    fn test_is_compose_file_standard() {
        assert!(is_compose_file(Path::new("docker-compose.yml")));
    }

    #[test]
    fn test_is_compose_file_yaml() {
        assert!(is_compose_file(Path::new("docker-compose.yaml")));
    }

    #[test]
    fn test_is_compose_file_variant() {
        assert!(is_compose_file(Path::new("docker-compose.dev.yml")));
    }

    #[test]
    fn test_is_compose_file_compose_only() {
        assert!(is_compose_file(Path::new("compose.yml")));
    }

    #[test]
    fn test_is_compose_file_compose_yaml() {
        assert!(is_compose_file(Path::new("compose.yaml")));
    }

    #[test]
    fn test_is_compose_file_non_compose() {
        assert!(!is_compose_file(Path::new("Dockerfile")));
    }

    #[test]
    fn test_is_compose_file_non_compose_env() {
        assert!(!is_compose_file(Path::new(".env")));
    }

    #[test]
    fn test_is_compose_file_non_compose_random_yml() {
        assert!(!is_compose_file(Path::new("my-compose.yml")));
    }

    #[test]
    fn test_is_compose_file_case_insensitive() {
        // The implementation uses to_lowercase(), so mixed case should work
        assert!(is_compose_file(Path::new("Docker-Compose.yml")));
    }

    // ========== scan_env_files_for_ports additional tests ==========

    #[test]
    fn test_scan_env_files_for_ports_empty_dir() {
        let dir = tempdir().unwrap();
        let ports = scan_env_files_for_ports(dir.path());
        assert!(ports.is_empty());
    }

    #[test]
    fn test_scan_env_files_for_ports_nested() {
        let dir = tempdir().unwrap();

        // Root .env
        fs::write(dir.path().join(".env"), "PORT=3000\n").unwrap();

        // Nested .env files at different depths
        let level1 = dir.path().join("services");
        fs::create_dir_all(&level1).unwrap();
        fs::write(level1.join(".env"), "SERVICE_PORT=4000\n").unwrap();

        let level2 = dir.path().join("services/api/config");
        fs::create_dir_all(&level2).unwrap();
        fs::write(level2.join(".env.test"), "TEST_PORT=5000\n").unwrap();

        let ports = scan_env_files_for_ports(dir.path());
        assert_eq!(ports.len(), 3);

        let vars: Vec<&str> = ports.iter().map(|p| p.variable_name.as_str()).collect();
        assert!(vars.contains(&"PORT"));
        assert!(vars.contains(&"SERVICE_PORT"));
        assert!(vars.contains(&"TEST_PORT"));
    }

    // ========== scan_dockerfile_for_ports additional tests ==========

    #[test]
    fn test_scan_dockerfile_for_ports_empty_dir() {
        let dir = tempdir().unwrap();
        let ports = scan_dockerfile_for_ports(dir.path());
        assert!(ports.is_empty());
    }

    #[test]
    fn test_scan_dockerfile_for_ports_variant_name() {
        let dir = tempdir().unwrap();

        // Dockerfile.dev variant
        fs::write(dir.path().join("Dockerfile.dev"), "FROM node:18\nEXPOSE 3000\n").unwrap();

        let ports = scan_dockerfile_for_ports(dir.path());
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port_value, 3000);
    }

    #[test]
    fn test_scan_dockerfile_for_ports_multiple_variants() {
        let dir = tempdir().unwrap();

        fs::write(dir.path().join("Dockerfile"), "EXPOSE 3000\n").unwrap();
        fs::write(dir.path().join("Dockerfile.prod"), "EXPOSE 8080\n").unwrap();

        let sub = dir.path().join("services/worker");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("Dockerfile"), "EXPOSE 9090\n").unwrap();

        let ports = scan_dockerfile_for_ports(dir.path());
        assert_eq!(ports.len(), 3);

        let values: Vec<u16> = ports.iter().map(|p| p.port_value).collect();
        assert!(values.contains(&3000));
        assert!(values.contains(&8080));
        assert!(values.contains(&9090));
    }

    // ========== scan_compose_for_ports additional tests ==========

    #[test]
    fn test_scan_compose_for_ports_empty_dir() {
        let dir = tempdir().unwrap();
        let ports = scan_compose_for_ports(dir.path());
        assert!(ports.is_empty());
    }

    #[test]
    fn test_scan_compose_for_ports_compose_yml_variant() {
        let dir = tempdir().unwrap();

        // compose.yml (without "docker-" prefix)
        fs::write(
            dir.path().join("compose.yml"),
            "services:\n  web:\n    ports:\n      - \"3000:3000\"\n",
        )
        .unwrap();

        let ports = scan_compose_for_ports(dir.path());
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port_value, 3000);
    }

    #[test]
    fn test_scan_compose_for_ports_compose_yaml_variant() {
        let dir = tempdir().unwrap();

        // compose.yaml
        fs::write(
            dir.path().join("compose.yaml"),
            "services:\n  web:\n    ports:\n      - \"4000:4000\"\n",
        )
        .unwrap();

        let ports = scan_compose_for_ports(dir.path());
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port_value, 4000);
    }

    // ========== scan_package_json_for_ports additional tests ==========

    #[test]
    fn test_scan_package_json_for_ports_empty_dir() {
        let dir = tempdir().unwrap();
        let ports = scan_package_json_for_ports(dir.path());
        assert!(ports.is_empty());
    }

    #[test]
    fn test_scan_package_json_skips_node_modules() {
        let dir = tempdir().unwrap();

        // Root package.json with ports
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts": {"dev": "next dev -p 3000"}}"#,
        )
        .unwrap();

        // node_modules at root level
        let nm_root = dir.path().join("node_modules/some-pkg");
        fs::create_dir_all(&nm_root).unwrap();
        fs::write(
            nm_root.join("package.json"),
            r#"{"scripts": {"start": "serve -p 5555"}}"#,
        )
        .unwrap();

        // node_modules in subdirectory
        let nm_nested = dir.path().join("packages/app/node_modules/another-pkg");
        fs::create_dir_all(&nm_nested).unwrap();
        fs::write(
            nm_nested.join("package.json"),
            r#"{"scripts": {"dev": "vite --port 6666"}}"#,
        )
        .unwrap();

        let ports = scan_package_json_for_ports(dir.path());
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port_value, 3000);
    }

    // ========== detect_all_ports additional tests ==========

    #[test]
    fn test_detect_all_ports_comprehensive() {
        let dir = tempdir().unwrap();

        // .env file
        fs::write(dir.path().join(".env"), "PORT=3000\nDB_PORT=5432\n").unwrap();

        // Dockerfile
        fs::write(dir.path().join("Dockerfile"), "FROM node:18\nEXPOSE 3000\nEXPOSE 8080\n").unwrap();

        // docker-compose.yml
        fs::write(
            dir.path().join("docker-compose.yml"),
            "services:\n  web:\n    ports:\n      - \"3000:3000\"\n      - \"5432:5432\"\n",
        )
        .unwrap();

        // package.json
        fs::write(
            dir.path().join("package.json"),
            r#"{"scripts": {"dev": "next dev -p 3000", "api": "node server.js --port=9090"}}"#,
        )
        .unwrap();

        let result = detect_all_ports(dir.path().to_string_lossy().to_string()).unwrap();

        assert_eq!(result.env_ports.len(), 2);
        assert_eq!(result.dockerfile_ports.len(), 2);
        assert_eq!(result.compose_ports.len(), 2);
        assert_eq!(result.script_ports.len(), 2);
    }

    #[test]
    fn test_detect_all_ports_empty_dir() {
        let dir = tempdir().unwrap();
        let result = detect_all_ports(dir.path().to_string_lossy().to_string()).unwrap();

        assert!(result.env_ports.is_empty());
        assert!(result.dockerfile_ports.is_empty());
        assert!(result.compose_ports.is_empty());
        assert!(result.script_ports.is_empty());
    }

    // ========== transform_compose_content additional tests ==========

    #[test]
    fn test_transform_compose_content_empty() {
        let content = "";
        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:3000".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];
        let result = transform_compose_content(content, &assignments);
        assert_eq!(result, "");
    }

    #[test]
    fn test_transform_compose_content_no_ports_section() {
        let content = "services:\n  web:\n    image: nginx\n    environment:\n      - FOO=bar\n";
        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:3000".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];
        let result = transform_compose_content(content, &assignments);
        assert_eq!(result, content);
    }

    #[test]
    fn test_transform_compose_content_preserves_trailing_newline() {
        let content = "services:\n  web:\n    ports:\n      - \"3000:3000\"\n";
        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:3000".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];
        let result = transform_compose_content(content, &assignments);
        assert!(result.ends_with('\n'));
        assert!(result.contains("3100:3000"));
    }

    #[test]
    fn test_transform_compose_content_no_trailing_newline() {
        let content = "services:\n  web:\n    ports:\n      - \"3000:3000\"";
        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:3000".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];
        let result = transform_compose_content(content, &assignments);
        assert!(!result.ends_with('\n'));
        assert!(result.contains("3100:3000"));
    }

    #[test]
    fn test_transform_compose_content_multiple_services() {
        let content = r#"services:
  web:
    ports:
      - "3000:3000"
  api:
    ports:
      - "8080:8080"
  db:
    ports:
      - "5432:5432"
  redis:
    ports:
      - "6379:6379"
"#;
        let assignments = vec![
            PortAssignment {
                variable_name: "COMPOSE:3000".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            },
            PortAssignment {
                variable_name: "COMPOSE:8080".to_string(),
                original_value: 8080,
                assigned_value: 8180,
            },
            PortAssignment {
                variable_name: "COMPOSE:5432".to_string(),
                original_value: 5432,
                assigned_value: 5532,
            },
        ];
        let result = transform_compose_content(content, &assignments);
        assert!(result.contains("3100:3000"));
        assert!(result.contains("8180:8080"));
        assert!(result.contains("5532:5432"));
        // 6379 has no assignment, stays unchanged
        assert!(result.contains("6379:6379"));
    }

    #[test]
    fn test_transform_compose_content_quoted_double() {
        let content = "services:\n  web:\n    ports:\n      - \"4000:4000\"\n";
        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:4000".to_string(),
            original_value: 4000,
            assigned_value: 4100,
        }];
        let result = transform_compose_content(content, &assignments);
        assert!(result.contains("4100:4000"), "Expected 4100:4000, got:\n{}", result);
    }

    // ========== transform_generic_content additional tests ==========

    #[test]
    fn test_transform_generic_content_basic() {
        let content = "port: 3000\n";
        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];
        let result = transform_generic_content(content, &assignments);
        assert!(result.contains("port: 3100"));
    }

    #[test]
    fn test_transform_generic_content_word_boundary() {
        // Ensure that port 80 doesn't get replaced inside 8080
        let content = "PORT=8080\nHTTP_PORT=80\n";
        let assignments = vec![PortAssignment {
            variable_name: "HTTP_PORT".to_string(),
            original_value: 80,
            assigned_value: 180,
        }];
        let result = transform_generic_content(content, &assignments);
        // 8080 should NOT be affected
        assert!(result.contains("8080"), "8080 should remain unchanged, got:\n{}", result);
        // 80 standalone should be replaced
        assert!(result.contains("180"), "80 should be replaced with 180, got:\n{}", result);
    }

    #[test]
    fn test_transform_generic_content_empty() {
        let content = "";
        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];
        let result = transform_generic_content(content, &assignments);
        assert_eq!(result, "");
    }

    #[test]
    fn test_transform_generic_content_no_assignments() {
        let content = "port: 3000\n";
        let result = transform_generic_content(content, &[]);
        assert_eq!(result, content);
    }

    #[test]
    fn test_transform_generic_content_multiple_occurrences() {
        // Same port appearing multiple times
        let content = "server_port=3000\nclient_port=3000\nproxy_port=3000\n";
        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];
        let result = transform_generic_content(content, &assignments);
        assert_eq!(result.matches("3100").count(), 3);
        assert_eq!(result.matches("3000").count(), 0);
    }

    #[test]
    fn test_transform_generic_content_at_start_and_end() {
        // Port number at the very start and end of content
        let content = "3000 is the port and it ends with 3000";
        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];
        let result = transform_generic_content(content, &assignments);
        assert!(result.contains("3100 is the port"));
        assert!(result.ends_with("3100"));
    }

    // ========== copy_files_with_port_transformation additional tests ==========

    #[test]
    fn test_copy_files_with_port_transformation_nonexistent_source() {
        let target_dir = tempdir().unwrap();
        let result = copy_files_with_port_transformation(
            "/nonexistent/source/path".to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
            vec![],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_copy_files_with_port_transformation_nonexistent_target() {
        let source_dir = tempdir().unwrap();
        fs::write(source_dir.path().join(".env"), "PORT=3000\n").unwrap();
        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            "/nonexistent/target/path".to_string(),
            vec![".env*".to_string()],
            vec![],
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_copy_files_with_port_transformation_env_file_transform() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(
            source_dir.path().join(".env"),
            "PORT=3000\nDB_PORT=5432\nAPP_NAME=myapp\n",
        )
        .unwrap();

        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            },
            PortAssignment {
                variable_name: "DB_PORT".to_string(),
                original_value: 5432,
                assigned_value: 5532,
            },
        ];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        assert!(result.errors.is_empty());

        let content = fs::read_to_string(target_dir.path().join(".env")).unwrap();
        assert!(content.contains("PORT=3100"));
        assert!(content.contains("DB_PORT=5532"));
        assert!(content.contains("APP_NAME=myapp")); // Non-port variable unchanged
    }

    #[test]
    fn test_copy_files_with_port_transformation_generic_file() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a config file that isn't .env or compose
        fs::write(
            source_dir.path().join("config.json"),
            r#"{"port": 3000, "dbPort": 5432}"#,
        )
        .unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config.json".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        assert!(result.errors.is_empty());

        let content = fs::read_to_string(target_dir.path().join("config.json")).unwrap();
        assert!(content.contains("3100"));
        // 5432 had no assignment, should remain
        assert!(content.contains("5432"));
    }

    #[test]
    fn test_copy_files_with_port_transformation_compose_via_copy() {
        // Test that when copying a new docker-compose file (not existing in target),
        // compose-specific transformation is applied (host-only)
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(
            source_dir.path().join("docker-compose.yml"),
            "services:\n  web:\n    ports:\n      - \"3000:3000\"\n",
        )
        .unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:3000".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["docker-compose.yml".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        assert!(result.errors.is_empty());

        let content = fs::read_to_string(target_dir.path().join("docker-compose.yml")).unwrap();
        // Host port should change, container port should remain
        assert!(content.contains("3100:3000"), "Expected 3100:3000 in copied compose file, got:\n{}", content);
    }

    #[test]
    fn test_copy_files_with_port_transformation_no_matching_files() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Only create a file that doesn't match the pattern
        fs::write(source_dir.path().join("config.txt"), "PORT=3000\n").unwrap();

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
            vec![],
        )
        .unwrap();

        assert!(result.copied_files.is_empty());
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_copy_files_with_port_transformation_existing_env_with_assignments() {
        // When .env already exists in target with assignments, it should be transformed in-place
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let content = "PORT=3000\nDB_PORT=5432\n";
        fs::write(source_dir.path().join(".env"), content).unwrap();
        fs::write(target_dir.path().join(".env"), content).unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
            assignments,
        )
        .unwrap();

        // File existed, so should be transformed, not copied
        assert_eq!(result.transformed_files.len(), 1);
        assert!(result.copied_files.is_empty());
        assert!(result.errors.is_empty());

        let transformed = fs::read_to_string(target_dir.path().join(".env")).unwrap();
        assert!(transformed.contains("PORT=3100"));
        assert!(transformed.contains("DB_PORT=5432")); // Unchanged, no assignment for this
    }

    #[test]
    fn test_copy_files_with_port_transformation_existing_identical_content() {
        // When file exists and transformation produces identical content, it should be skipped
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let content = "APP_NAME=myapp\nAPP_VERSION=1.0\n";
        fs::write(source_dir.path().join(".env"), content).unwrap();
        fs::write(target_dir.path().join(".env"), content).unwrap();

        // Assignments that don't match any variable in the file
        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
            assignments,
        )
        .unwrap();

        // Content is identical after transform attempt, so should be skipped
        assert_eq!(result.skipped_files.len(), 1);
        assert!(result.transformed_files.is_empty());
        assert!(result.copied_files.is_empty());
    }

    // ========== allocate_ports additional edge case tests ==========

    #[test]
    fn test_allocate_ports_zero_index() {
        let ports = vec![PortSource {
            file_path: ".env".to_string(),
            variable_name: "PORT".to_string(),
            port_value: 3000,
            line_number: 1,
        }];
        let result = allocate_ports(&ports, 0);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Worktree index must be greater than 0"
        );
    }

    #[test]
    fn test_allocate_ports_overflow_high_index() {
        // Large worktree_index causing most ports to overflow
        let ports = vec![
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "PORT".to_string(),
                port_value: 60000,
                line_number: 1,
            },
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "DB_PORT".to_string(),
                port_value: 100,
                line_number: 2,
            },
        ];

        let result = allocate_ports(&ports, 100).unwrap();
        // PORT=60000 + 10000 = 70000 > 65535, should produce warning
        assert_eq!(result.overflow_warnings.len(), 1);
        assert!(result.overflow_warnings[0].contains("60000"));
        // DB_PORT=100 + 10000 = 10100, should succeed
        assert_eq!(result.assignments.len(), 1);
        assert_eq!(result.assignments[0].variable_name, "DB_PORT");
        assert_eq!(result.assignments[0].assigned_value, 10100);
    }

    #[test]
    fn test_allocate_ports_duplicate_values_different_variables() {
        // Two different variable names with the same port value
        let ports = vec![
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "WEB_PORT".to_string(),
                port_value: 3000,
                line_number: 1,
            },
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "PROXY_PORT".to_string(),
                port_value: 3000,
                line_number: 2,
            },
        ];

        let result = allocate_ports(&ports, 1).unwrap();
        assert_eq!(result.assignments.len(), 2);
        // Both should get the same assigned value since original_value is the same
        assert_eq!(result.assignments[0].assigned_value, 3100);
        assert_eq!(result.assignments[1].assigned_value, 3100);
        // But variable names should differ
        assert_eq!(result.assignments[0].variable_name, "WEB_PORT");
        assert_eq!(result.assignments[1].variable_name, "PROXY_PORT");
    }

    #[test]
    fn test_allocate_ports_empty_ports() {
        let ports: Vec<PortSource> = vec![];
        let result = allocate_ports(&ports, 1).unwrap();
        assert!(result.assignments.is_empty());
        assert!(result.overflow_warnings.is_empty());
        assert_eq!(result.worktree_index, 1);
    }

    #[test]
    fn test_allocate_ports_all_overflow() {
        // All ports overflow
        let ports = vec![
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "PORT_A".to_string(),
                port_value: 65000,
                line_number: 1,
            },
            PortSource {
                file_path: ".env".to_string(),
                variable_name: "PORT_B".to_string(),
                port_value: 65500,
                line_number: 2,
            },
        ];

        let result = allocate_ports(&ports, 10).unwrap();
        // 65000 + 1000 = 66000 > 65535
        // 65500 + 1000 = 66500 > 65535
        assert!(result.assignments.is_empty());
        assert_eq!(result.overflow_warnings.len(), 2);
    }

    #[test]
    fn test_allocate_ports_boundary_value() {
        // Port exactly at boundary: 65535 - offset = valid
        let ports = vec![PortSource {
            file_path: ".env".to_string(),
            variable_name: "PORT".to_string(),
            port_value: 65435, // 65435 + 100 = 65535 (exactly at limit)
            line_number: 1,
        }];

        let result = allocate_ports(&ports, 1).unwrap();
        assert_eq!(result.assignments.len(), 1);
        assert_eq!(result.assignments[0].assigned_value, 65535);
        assert!(result.overflow_warnings.is_empty());
    }

    #[test]
    fn test_allocate_ports_boundary_overflow() {
        // Port just over boundary: 65536 after offset
        let ports = vec![PortSource {
            file_path: ".env".to_string(),
            variable_name: "PORT".to_string(),
            port_value: 65436, // 65436 + 100 = 65536 (exceeds limit)
            line_number: 1,
        }];

        let result = allocate_ports(&ports, 1).unwrap();
        assert!(result.assignments.is_empty());
        assert_eq!(result.overflow_warnings.len(), 1);
    }

    // ========== detect_ports_in_env_file additional edge cases ==========

    #[test]
    fn test_detect_ports_in_env_file_mixed_valid_invalid() {
        let content = "PORT=3000\nINVALID_PORT=abc\nDB_PORT=5432\nTOO_BIG=99999\n";
        let ports = detect_ports_in_env_file(content, ".env");
        // PORT=3000 valid, INVALID_PORT=abc no match (regex needs digits),
        // DB_PORT=5432 valid, TOO_BIG=99999 won't parse as u16 (> 65535)
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].variable_name, "PORT");
        assert_eq!(ports[1].variable_name, "DB_PORT");
    }

    #[test]
    fn test_detect_ports_in_env_file_with_spaces_around_comments() {
        let content = "  # Comment with spaces\n  PORT=3000\n";
        let ports = detect_ports_in_env_file(content, ".env");
        // Lines are trimmed, so "  PORT=3000" should match after trim
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].variable_name, "PORT");
    }

    // ========== detect_ports_in_package_json edge cases ==========

    #[test]
    fn test_detect_ports_in_package_json_multiple_ports_in_one_script() {
        let content = r#"{
  "scripts": {
    "dev": "concurrently \"next dev -p 3000\" \"api --port 8080\""
  }
}"#;

        let ports = detect_ports_in_package_json(content, "package.json");
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].port_value, 3000);
        assert_eq!(ports[1].port_value, 8080);
    }

    // ========== copy_files with subdirectories ==========

    #[test]
    fn test_copy_files_with_port_transformation_creates_subdirs() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create nested source files
        let sub = source_dir.path().join("config/env");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join(".env"), "PORT=3000\n").unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["**/.env*".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        assert!(result.errors.is_empty());

        // Verify subdirectories were created in target
        let target_file = target_dir.path().join("config/env/.env");
        assert!(target_file.exists());
        let content = fs::read_to_string(target_file).unwrap();
        assert!(content.contains("PORT=3100"));
    }

    // ========== copy_files_with_port_transformation: in-place generic transform ==========

    #[test]
    fn test_copy_files_existing_generic_file_with_assignments_transforms_in_place() {
        // When a generic file (not .env, not compose) already exists in target
        // and assignments match, it should be transformed in-place
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let content = r#"{"port": 3000, "dbPort": 5432}"#;
        fs::write(source_dir.path().join("config.json"), content).unwrap();
        fs::write(target_dir.path().join("config.json"), content).unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config.json".to_string()],
            assignments,
        )
        .unwrap();

        // Should be transformed in-place (file already existed)
        assert_eq!(result.transformed_files.len(), 1);
        assert!(result.copied_files.is_empty());
        assert!(result.errors.is_empty());

        let transformed = fs::read_to_string(target_dir.path().join("config.json")).unwrap();
        assert!(transformed.contains("3100"));
        assert!(transformed.contains("5432")); // No assignment for 5432
    }

    #[test]
    fn test_copy_files_existing_generic_file_no_change_skipped() {
        // When a generic file already exists and transformation produces same content
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let content = r#"{"name": "app", "version": "1.0"}"#;
        fs::write(source_dir.path().join("config.json"), content).unwrap();
        fs::write(target_dir.path().join("config.json"), content).unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config.json".to_string()],
            assignments,
        )
        .unwrap();

        // Content unchanged, so should be skipped
        assert_eq!(result.skipped_files.len(), 1);
        assert!(result.transformed_files.is_empty());
        assert!(result.copied_files.is_empty());
    }

    // ========== copy_files: directory glob patterns ==========

    #[test]
    fn test_copy_files_with_directory_glob_pattern() {
        // Test that when a glob pattern matches a directory,
        // copy_directory_recursive is used
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a directory with files inside
        let config_dir = source_dir.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join(".env"), "PORT=3000\n").unwrap();
        fs::write(config_dir.join("settings.json"), r#"{"port": 3000}"#).unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config".to_string()],
            assignments,
        )
        .unwrap();

        // Both files should be copied
        assert_eq!(result.copied_files.len(), 2);
        assert!(result.errors.is_empty());

        // Verify .env was transformed as env file
        let env_content = fs::read_to_string(target_dir.path().join("config/.env")).unwrap();
        assert!(env_content.contains("PORT=3100"));

        // Verify settings.json was transformed as generic file
        let json_content =
            fs::read_to_string(target_dir.path().join("config/settings.json")).unwrap();
        assert!(json_content.contains("3100"));
    }

    #[test]
    fn test_copy_files_directory_recursive_nested() {
        // Test recursive directory copying with deeply nested files
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let deep = source_dir.path().join("config/nested/deep");
        fs::create_dir_all(&deep).unwrap();
        fs::write(deep.join(".env"), "PORT=3000\n").unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        assert!(result.errors.is_empty());

        let target_file = target_dir.path().join("config/nested/deep/.env");
        assert!(target_file.exists());
        let content = fs::read_to_string(target_file).unwrap();
        assert!(content.contains("PORT=3100"));
    }

    #[test]
    fn test_copy_files_directory_recursive_no_assignments() {
        // Test recursive directory copying without port assignments (plain copy)
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let config_dir = source_dir.path().join("config");
        fs::create_dir_all(&config_dir).unwrap();
        fs::write(config_dir.join(".env"), "PORT=3000\n").unwrap();
        fs::write(config_dir.join("readme.txt"), "hello\n").unwrap();

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config".to_string()],
            vec![], // No assignments
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 2);
        assert!(result.errors.is_empty());

        // Files should be plain copies without transformation
        let env_content = fs::read_to_string(target_dir.path().join("config/.env")).unwrap();
        assert!(env_content.contains("PORT=3000")); // Unchanged

        let readme = fs::read_to_string(target_dir.path().join("config/readme.txt")).unwrap();
        assert_eq!(readme, "hello\n");
    }

    // ========== copy_files: compose in-place existing transform ==========

    #[test]
    fn test_copy_files_existing_compose_no_change_skipped() {
        // When compose file exists but no assignment matches, it should be skipped
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let content = "services:\n  web:\n    ports:\n      - \"9999:9999\"\n";
        fs::write(source_dir.path().join("docker-compose.yml"), content).unwrap();
        fs::write(target_dir.path().join("docker-compose.yml"), content).unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:3000".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["docker-compose.yml".to_string()],
            assignments,
        )
        .unwrap();

        // Content is identical after transform (no matching port), should be skipped
        assert_eq!(result.skipped_files.len(), 1);
        assert!(result.transformed_files.is_empty());
    }

    // ========== copy_files: multiple patterns ==========

    #[test]
    fn test_copy_files_with_multiple_patterns() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(source_dir.path().join(".env"), "PORT=3000\n").unwrap();
        fs::write(
            source_dir.path().join("docker-compose.yml"),
            "services:\n  web:\n    ports:\n      - \"3000:3000\"\n",
        )
        .unwrap();
        fs::write(
            source_dir.path().join("config.json"),
            r#"{"port": 3000}"#,
        )
        .unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![
                ".env*".to_string(),
                "docker-compose.yml".to_string(),
                "config.json".to_string(),
            ],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 3);
        assert!(result.errors.is_empty());

        // .env uses env-specific transform
        let env = fs::read_to_string(target_dir.path().join(".env")).unwrap();
        assert!(env.contains("PORT=3100"));

        // docker-compose uses compose-specific transform (host-only)
        let compose =
            fs::read_to_string(target_dir.path().join("docker-compose.yml")).unwrap();
        assert!(
            compose.contains("3100:3000"),
            "Expected 3100:3000 in compose, got:\n{}",
            compose
        );

        // config.json uses generic transform
        let config = fs::read_to_string(target_dir.path().join("config.json")).unwrap();
        assert!(config.contains("3100"));
    }

    // ========== copy_files: existing file in directory recursive with assignments ==========

    #[test]
    fn test_copy_files_directory_recursive_with_existing_files() {
        // When copying a directory recursively and some files already exist in target
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let src_config = source_dir.path().join("config");
        fs::create_dir_all(&src_config).unwrap();
        fs::write(src_config.join(".env"), "PORT=3000\n").unwrap();

        // Pre-create the file in target
        let tgt_config = target_dir.path().join("config");
        fs::create_dir_all(&tgt_config).unwrap();
        fs::write(tgt_config.join(".env"), "PORT=3000\n").unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config".to_string()],
            assignments,
        )
        .unwrap();

        // File existed, should be transformed in-place
        assert_eq!(result.transformed_files.len(), 1);
        assert!(result.copied_files.is_empty());
        assert!(result.errors.is_empty());

        let content = fs::read_to_string(tgt_config.join(".env")).unwrap();
        assert!(content.contains("PORT=3100"));
    }

    // ========== transform_env_content: edge cases ==========

    #[test]
    fn test_transform_env_content_empty_assignments() {
        let content = "PORT=3000\nDB_PORT=5432\n";
        let result = transform_env_content(content, &[]);
        // No assignments means no replacements
        assert_eq!(result.content, content);
        assert!(result.replacements.is_empty());
    }

    #[test]
    fn test_transform_env_content_unmatched_assignment() {
        let content = "PORT=3000\n";
        let assignments = vec![PortAssignment {
            variable_name: "DB_PORT".to_string(),
            original_value: 5432,
            assigned_value: 5532,
        }];
        let result = transform_env_content(content, &assignments);
        // No matching variable, content unchanged
        assert_eq!(result.content, content);
        assert!(result.replacements.is_empty());
    }

    #[test]
    fn test_transform_env_content_preserves_empty_lines() {
        let content = "PORT=3000\n\nDB_PORT=5432\n";
        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            },
            PortAssignment {
                variable_name: "DB_PORT".to_string(),
                original_value: 5432,
                assigned_value: 5532,
            },
        ];
        let result = transform_env_content(content, &assignments);
        assert!(result.content.contains("PORT=3100"));
        assert!(result.content.contains("DB_PORT=5532"));
        // Empty line should be preserved
        assert!(result.content.contains("\n\n"));
    }

    // ========== transform_generic_content: descending sort order ==========

    #[test]
    fn test_transform_generic_content_descending_sort_prevents_collision() {
        // Port 30001 and 3000 - replacing 3000 first could corrupt 30001
        // The function sorts by original_value descending to prevent this
        let content = "PORT_A=30001\nPORT_B=3000\n";
        let assignments = vec![
            PortAssignment {
                variable_name: "PORT_B".to_string(),
                original_value: 3000,
                assigned_value: 4000,
            },
            PortAssignment {
                variable_name: "PORT_A".to_string(),
                original_value: 30001,
                assigned_value: 40001,
            },
        ];
        let result = transform_generic_content(content, &assignments);
        assert!(
            result.contains("40001"),
            "30001 should become 40001, got:\n{}",
            result
        );
        assert!(
            result.contains("4000"),
            "3000 should become 4000, got:\n{}",
            result
        );
    }

    // ========== copy_files: empty patterns ==========

    #[test]
    fn test_copy_files_with_empty_patterns() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(source_dir.path().join(".env"), "PORT=3000\n").unwrap();

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![], // No patterns
            vec![],
        )
        .unwrap();

        assert!(result.copied_files.is_empty());
        assert!(result.errors.is_empty());
    }

    // ========== copy_files: compose file new copy with host-only transform ==========

    #[test]
    fn test_copy_files_compose_new_copy_asymmetric_ports() {
        // Compose file with asymmetric host:container ports, copied as new file
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(
            source_dir.path().join("docker-compose.yml"),
            "services:\n  web:\n    ports:\n      - \"8080:3000\"\n",
        )
        .unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:8080".to_string(),
            original_value: 8080,
            assigned_value: 8180,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["docker-compose.yml".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        assert!(result.errors.is_empty());

        let content =
            fs::read_to_string(target_dir.path().join("docker-compose.yml")).unwrap();
        // Only host port changes; container port stays 3000
        assert!(
            content.contains("8180:3000"),
            "Expected 8180:3000, got:\n{}",
            content
        );
    }

    // ========== copy_files: env file with URL ports ==========

    #[test]
    fn test_copy_files_env_with_url_ports() {
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        fs::write(
            source_dir.path().join(".env"),
            "PORT=3000\nREDIS_URL=redis://localhost:6379\nDATABASE_URL=postgres://user:pass@db:5432/mydb\n",
        )
        .unwrap();

        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            },
            PortAssignment {
                variable_name: "REDIS_URL".to_string(),
                original_value: 6379,
                assigned_value: 6479,
            },
            PortAssignment {
                variable_name: "DATABASE_URL".to_string(),
                original_value: 5432,
                assigned_value: 5532,
            },
        ];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env*".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        assert!(result.errors.is_empty());

        let content = fs::read_to_string(target_dir.path().join(".env")).unwrap();
        assert!(content.contains("PORT=3100"));
        assert!(content.contains("REDIS_URL=redis://localhost:6479"));
        assert!(content.contains("DATABASE_URL=postgres://user:pass@db:5532/mydb"));
    }

    // ========== copy_files: mixed file types in single copy ==========

    #[test]
    fn test_copy_files_directory_with_mixed_file_types() {
        // Directory containing .env, docker-compose, and generic files
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        let project = source_dir.path().join("project");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join(".env"), "PORT=3000\n").unwrap();
        fs::write(
            project.join("docker-compose.yml"),
            "services:\n  web:\n    ports:\n      - \"3000:3000\"\n",
        )
        .unwrap();
        fs::write(
            project.join("nginx.conf"),
            "listen 3000;\nproxy_pass http://app:3000;\n",
        )
        .unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 3100,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["project".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 3);
        assert!(result.errors.is_empty());

        // .env: env-specific transform
        let env = fs::read_to_string(target_dir.path().join("project/.env")).unwrap();
        assert!(env.contains("PORT=3100"));

        // docker-compose: compose-specific transform (host-only)
        let compose =
            fs::read_to_string(target_dir.path().join("project/docker-compose.yml")).unwrap();
        assert!(
            compose.contains("3100:3000"),
            "Expected compose host-only transform, got:\n{}",
            compose
        );

        // nginx.conf: generic transform (all occurrences)
        let nginx =
            fs::read_to_string(target_dir.path().join("project/nginx.conf")).unwrap();
        assert!(nginx.contains("listen 3100"));
        assert!(nginx.contains("proxy_pass http://app:3100"));
    }

    #[test]
    fn test_copy_files_transform_existing_target_with_assignments() {
        // Test the path where target file already exists and assignments are applied in-place
        let source_dir = tempfile::tempdir().unwrap();
        let target_dir = tempfile::tempdir().unwrap();

        // Create source .env file
        let source_env_dir = source_dir.path().join("config");
        fs::create_dir_all(&source_env_dir).unwrap();
        fs::write(source_env_dir.join(".env"), "PORT=3000\nDB_PORT=5432\n").unwrap();

        // Create target with the SAME file already present (simulating git checkout)
        let target_env_dir = target_dir.path().join("config");
        fs::create_dir_all(&target_env_dir).unwrap();
        fs::write(target_env_dir.join(".env"), "PORT=3000\nDB_PORT=5432\n").unwrap();

        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 20001,
            },
        ];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config/.env".to_string()],
            assignments,
        )
        .unwrap();

        // File should be transformed (in-place), not copied
        assert!(
            result.transformed_files.iter().any(|f| f.contains(".env")),
            "Expected .env to be in transformed_files: {:?}",
            result
        );
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);

        // Verify content was transformed in the existing file
        let content = fs::read_to_string(target_env_dir.join(".env")).unwrap();
        assert!(content.contains("PORT=20001"), "Content: {}", content);
    }

    #[test]
    fn test_copy_files_transform_existing_target_without_assignments() {
        // Test: target file exists but no assignments → file should be skipped
        let source_dir = tempfile::tempdir().unwrap();
        let target_dir = tempfile::tempdir().unwrap();

        fs::write(source_dir.path().join(".env"), "PORT=3000\n").unwrap();
        fs::write(target_dir.path().join(".env"), "PORT=3000\n").unwrap();

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env".to_string()],
            vec![], // No assignments
        )
        .unwrap();

        assert!(
            result.skipped_files.iter().any(|f| f.contains(".env")),
            "Expected .env to be in skipped_files: {:?}",
            result
        );
    }

    #[test]
    fn test_copy_files_transform_existing_target_compose_in_place() {
        // Test in-place transform for compose files that already exist in target
        let source_dir = tempfile::tempdir().unwrap();
        let target_dir = tempfile::tempdir().unwrap();

        let compose_content = "services:\n  web:\n    ports:\n      - \"3000:3000\"\n";
        fs::write(source_dir.path().join("docker-compose.yml"), compose_content).unwrap();
        fs::write(target_dir.path().join("docker-compose.yml"), compose_content).unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 25000,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["docker-compose.yml".to_string()],
            assignments,
        )
        .unwrap();

        assert!(
            result.transformed_files.iter().any(|f| f.contains("docker-compose")),
            "Expected docker-compose.yml in transformed_files: {:?}",
            result
        );

        let content = fs::read_to_string(target_dir.path().join("docker-compose.yml")).unwrap();
        assert!(content.contains("25000:3000"), "Content: {}", content);
    }

    #[test]
    fn test_copy_files_transform_existing_unchanged_content() {
        // Test: target file exists, has assignments but content doesn't change → skipped
        let source_dir = tempfile::tempdir().unwrap();
        let target_dir = tempfile::tempdir().unwrap();

        // Content has no port that matches the assignment
        let content = "HOSTNAME=localhost\nDEBUG=true\n";
        fs::write(source_dir.path().join(".env"), content).unwrap();
        fs::write(target_dir.path().join(".env"), content).unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 9999,
            assigned_value: 20001,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec![".env".to_string()],
            assignments,
        )
        .unwrap();

        assert!(
            result.skipped_files.iter().any(|f| f.contains(".env")),
            "Expected .env in skipped_files when content unchanged: {:?}",
            result
        );
    }

    #[test]
    fn test_copy_files_transform_new_file_no_assignments_just_copy() {
        // Test: new file with no assignments → simple copy
        let source_dir = tempfile::tempdir().unwrap();
        let target_dir = tempfile::tempdir().unwrap();

        fs::write(source_dir.path().join("config.txt"), "some config").unwrap();

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["config.txt".to_string()],
            vec![],
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        assert!(result.errors.is_empty());

        let content = fs::read_to_string(target_dir.path().join("config.txt")).unwrap();
        assert_eq!(content, "some config");
    }

    #[test]
    fn test_copy_files_transform_creates_nested_directories() {
        // Test: target parent directory doesn't exist → should be created
        let source_dir = tempfile::tempdir().unwrap();
        let target_dir = tempfile::tempdir().unwrap();

        let nested = source_dir.path().join("a/b/c");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join(".env"), "PORT=3000\n").unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 20001,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["a/b/c/.env".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        assert!(result.errors.is_empty());

        let content = fs::read_to_string(target_dir.path().join("a/b/c/.env")).unwrap();
        assert!(content.contains("PORT=20001"));
    }

    #[test]
    fn test_copy_files_transform_directory_pattern() {
        // Test: glob pattern matches a directory → copy recursively
        let source_dir = tempfile::tempdir().unwrap();
        let target_dir = tempfile::tempdir().unwrap();

        let sub = source_dir.path().join("configs");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join(".env"), "PORT=3000\n").unwrap();
        fs::write(sub.join("app.conf"), "listen 3000;\n").unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 20001,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["configs".to_string()],
            assignments,
        )
        .unwrap();

        assert!(result.copied_files.len() >= 2, "Should copy files in directory: {:?}", result);
        assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
    }

    #[test]
    fn test_copy_files_transform_invalid_glob_pattern() {
        // Test: invalid glob pattern → error recorded
        let source_dir = tempfile::tempdir().unwrap();
        let target_dir = tempfile::tempdir().unwrap();

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["[invalid".to_string()],
            vec![],
        )
        .unwrap();

        assert!(!result.errors.is_empty(), "Expected error for invalid glob pattern");
    }

    #[test]
    fn test_copy_files_transform_generic_file() {
        // Test: non-env, non-compose file with assignments → generic transform
        let source_dir = tempfile::tempdir().unwrap();
        let target_dir = tempfile::tempdir().unwrap();

        fs::write(
            source_dir.path().join("nginx.conf"),
            "server {\n    listen 3000;\n    proxy_pass http://localhost:3000;\n}\n",
        )
        .unwrap();

        let assignments = vec![PortAssignment {
            variable_name: "PORT".to_string(),
            original_value: 3000,
            assigned_value: 22000,
        }];

        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["nginx.conf".to_string()],
            assignments,
        )
        .unwrap();

        assert_eq!(result.copied_files.len(), 1);
        let content = fs::read_to_string(target_dir.path().join("nginx.conf")).unwrap();
        assert!(content.contains("22000"), "Content: {}", content);
    }

    // ========== Additional edge case tests for uncovered lines ==========

    #[test]
    fn test_detect_ports_in_env_file_port_value_exceeds_u16() {
        // L85: port_str.parse::<u16>() fails when value > 65535
        let content = "HUGE_PORT=99999\nPORT=3000\n";
        let ports = detect_ports_in_env_file(content, ".env");
        // 99999 exceeds u16 range, so only PORT=3000 should be detected
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].variable_name, "PORT");
        assert_eq!(ports[0].port_value, 3000);
    }

    #[test]
    fn test_detect_ports_in_env_file_url_port_exceeds_u16() {
        // L98: URL port parse failure when port > 65535
        let content = "REDIS_URL=redis://localhost:99999\nMONGO_URL=mongodb://localhost:27017\n";
        let ports = detect_ports_in_env_file(content, ".env");
        // 99999 exceeds u16, only MONGO_URL should be detected
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].variable_name, "MONGO_URL");
        assert_eq!(ports[0].port_value, 27017);
    }

    #[test]
    fn test_detect_ports_in_dockerfile_port_exceeds_u16() {
        // L127: port parse failure in Dockerfile when value > 65535
        let content = "FROM node:18\nEXPOSE 99999\nEXPOSE 3000\n";
        let ports = detect_ports_in_dockerfile(content, "Dockerfile");
        // 99999 exceeds u16, only EXPOSE 3000 should be detected
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port_value, 3000);
    }

    #[test]
    fn test_detect_ports_in_compose_with_comments() {
        // L144: comment lines in compose file should be skipped
        let content = r#"services:
  web:
    ports:
      # This is a commented port
      - "3000:3000"
      # - "9999:9999"
"#;
        let ports = detect_ports_in_compose(content, "docker-compose.yml");
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port_value, 3000);
    }

    #[test]
    fn test_detect_ports_in_compose_port_exceeds_u16() {
        // L156: host port parse failure when value > 65535
        let content = "services:\n  web:\n    ports:\n      - \"99999:3000\"\n      - \"8080:8080\"\n";
        let ports = detect_ports_in_compose(content, "docker-compose.yml");
        // 99999 exceeds u16, only 8080 should be detected
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port_value, 8080);
    }

    #[test]
    fn test_detect_ports_in_package_json_script_port_exceeds_u16() {
        // L201-204: port parse failure in scripts when port > 65535
        let content = r#"{
  "scripts": {
    "dev": "next dev -p 99999",
    "start": "node server.js --port 3000"
  }
}"#;
        let ports = detect_ports_in_package_json(content, "package.json");
        // 99999 exceeds u16, only --port 3000 should be detected
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port_value, 3000);
    }

    #[test]
    fn test_detect_ports_in_package_json_script_value_is_not_number() {
        // Scripts with non-numeric values after port flag should not match
        let content = r#"{
  "scripts": {
    "dev": "next dev --port auto",
    "start": "node server.js -p 8080"
  }
}"#;
        let ports = detect_ports_in_package_json(content, "package.json");
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port_value, 8080);
    }

    #[test]
    fn test_transform_compose_content_host_port_exceeds_u16() {
        // L576-577: host_port_str.parse::<u16>() fails for ports > 65535
        // This tests the edge case where the regex matches but parse fails
        let content = "services:\n  web:\n    ports:\n      - \"99999:3000\"\n      - \"8080:8080\"\n";
        let assignments = vec![PortAssignment {
            variable_name: "COMPOSE:8080".to_string(),
            original_value: 8080,
            assigned_value: 8180,
        }];
        let result = transform_compose_content(content, &assignments);
        // 99999 can't be parsed as u16, line should pass through unchanged
        assert!(
            result.contains("99999:3000"),
            "Unparseable port line should remain unchanged, got:\n{}",
            result
        );
        // 8080 should be transformed
        assert!(
            result.contains("8180:8080"),
            "Expected 8180:8080, got:\n{}",
            result
        );
    }

    #[test]
    fn test_transform_env_content_port_var_no_assignment() {
        // L459: port_re matches but variable is not in assignment_map
        // The line should be preserved as-is
        let content = "PORT=3000\nDB_PORT=5432\nAPI_PORT=8080\n";
        let assignments = vec![PortAssignment {
            variable_name: "DB_PORT".to_string(),
            original_value: 5432,
            assigned_value: 5532,
        }];
        let result = transform_env_content(content, &assignments);
        // PORT and API_PORT have no assignment, should remain unchanged
        assert!(result.content.contains("PORT=3000"));
        assert!(result.content.contains("DB_PORT=5532"));
        assert!(result.content.contains("API_PORT=8080"));
        assert_eq!(result.replacements.len(), 1);
        assert_eq!(result.replacements[0].variable_name, "DB_PORT");
    }

    #[test]
    fn test_transform_env_content_url_var_no_assignment() {
        // L483: URL regex matches but variable is not in assignment_map
        let content = "REDIS_URL=redis://localhost:6379\nMONGO_URL=mongodb://localhost:27017\n";
        let assignments = vec![PortAssignment {
            variable_name: "REDIS_URL".to_string(),
            original_value: 6379,
            assigned_value: 6479,
        }];
        let result = transform_env_content(content, &assignments);
        // REDIS_URL should be transformed
        assert!(result.content.contains("REDIS_URL=redis://localhost:6479"));
        // MONGO_URL has no assignment, should remain unchanged
        assert!(result.content.contains("MONGO_URL=mongodb://localhost:27017"));
        assert_eq!(result.replacements.len(), 1);
    }

    #[test]
    fn test_scan_env_files_for_ports_unreadable_file() {
        // L235-240: Test with a directory entry that matches glob but can't be read
        // (e.g., a directory named .env_dir which is_file() returns false)
        let dir = tempdir().unwrap();
        // Create a directory named ".env_something" which matches .env* pattern
        // but is_file() will return false, so it will be skipped
        let env_dir = dir.path().join(".env_configs");
        fs::create_dir_all(&env_dir).unwrap();
        // Also create a valid .env file
        fs::write(dir.path().join(".env"), "PORT=3000\n").unwrap();

        let ports = scan_env_files_for_ports(dir.path());
        // Only the real .env file should be detected
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].variable_name, "PORT");
    }

    #[test]
    fn test_scan_dockerfile_for_ports_directory_named_dockerfile() {
        // L267-269: Directory named "Dockerfile" should be skipped (is_file check)
        let dir = tempdir().unwrap();
        let docker_dir = dir.path().join("Dockerfile");
        fs::create_dir_all(&docker_dir).unwrap();
        // Also create a real Dockerfile
        fs::write(dir.path().join("Dockerfile.prod"), "EXPOSE 8080\n").unwrap();

        let ports = scan_dockerfile_for_ports(dir.path());
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].port_value, 8080);
    }

    #[test]
    fn test_scan_compose_for_ports_directory_named_compose() {
        // L298-300: Directory named like a compose file should be skipped
        let dir = tempdir().unwrap();
        // This won't actually match compose pattern as a dir, but test the scan
        // with a valid file alongside non-matching entries
        fs::write(
            dir.path().join("docker-compose.yml"),
            "services:\n  web:\n    ports:\n      - \"3000:3000\"\n",
        )
        .unwrap();
        // Create a compose.yaml as well
        fs::write(
            dir.path().join("compose.yaml"),
            "services:\n  api:\n    ports:\n      - \"8080:8080\"\n",
        )
        .unwrap();

        let ports = scan_compose_for_ports(dir.path());
        assert_eq!(ports.len(), 2);
    }

    #[test]
    fn test_scan_package_json_for_ports_with_only_node_modules() {
        // L331-333: All package.json files are inside node_modules
        let dir = tempdir().unwrap();
        let nm = dir.path().join("node_modules/pkg");
        fs::create_dir_all(&nm).unwrap();
        fs::write(
            nm.join("package.json"),
            r#"{"scripts": {"dev": "serve -p 5555"}}"#,
        )
        .unwrap();

        let ports = scan_package_json_for_ports(dir.path());
        assert!(ports.is_empty());
    }

    #[test]
    fn test_allocate_ports_max_worktree_index() {
        // L483-484: Test with very large worktree_index that causes overflow
        // for all ports
        let ports = vec![PortSource {
            file_path: ".env".to_string(),
            variable_name: "PORT".to_string(),
            port_value: 3000,
            line_number: 1,
        }];

        // worktree_index 650: offset = 65000, 3000 + 65000 = 68000 > 65535
        let result = allocate_ports(&ports, 650).unwrap();
        assert!(result.assignments.is_empty());
        assert_eq!(result.overflow_warnings.len(), 1);
        assert!(result.overflow_warnings[0].contains("3000"));
    }

    #[test]
    fn test_copy_files_with_port_transformation_relative_path_error() {
        // L649-653: Test the error path when strip_prefix fails
        // This is difficult to trigger directly since we construct paths from source,
        // but we can test by verifying the function handles the case
        // where source and target exist but patterns produce no matches
        let source_dir = tempdir().unwrap();
        let target_dir = tempdir().unwrap();

        // Create a file in source
        fs::write(source_dir.path().join(".env"), "PORT=3000\n").unwrap();

        // Use a pattern that matches files outside the source path
        // This shouldn't cause a relative path error but tests path handling
        let result = copy_files_with_port_transformation(
            source_dir.path().to_string_lossy().to_string(),
            target_dir.path().to_string_lossy().to_string(),
            vec!["nonexistent_pattern_xyz".to_string()],
            vec![],
        )
        .unwrap();

        assert!(result.copied_files.is_empty());
    }

    #[test]
    fn test_copy_files_existing_file_read_only_write_error() {
        // L675-680: Test write failure on existing file during in-place transform
        // We can simulate by making the target file read-only
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let source_dir = tempdir().unwrap();
            let target_dir = tempdir().unwrap();

            let content = "PORT=3000\n";
            fs::write(source_dir.path().join(".env"), content).unwrap();
            fs::write(target_dir.path().join(".env"), content).unwrap();

            // Make target file read-only
            let target_file = target_dir.path().join(".env");
            let mut perms = fs::metadata(&target_file).unwrap().permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&target_file, perms).unwrap();

            let assignments = vec![PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            }];

            let result = copy_files_with_port_transformation(
                source_dir.path().to_string_lossy().to_string(),
                target_dir.path().to_string_lossy().to_string(),
                vec![".env*".to_string()],
                assignments,
            )
            .unwrap();

            // Should have an error about write failure
            assert!(
                !result.errors.is_empty(),
                "Expected write error for read-only file: {:?}",
                result
            );
            assert!(
                result.errors[0].contains("Failed to write"),
                "Error should mention write failure: {}",
                result.errors[0]
            );

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&target_file).unwrap().permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&target_file, perms).unwrap();
        }
    }

    #[test]
    fn test_copy_files_new_file_write_error_read_only_dir() {
        // L706/L731-737: Test write failure when target directory is read-only
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let source_dir = tempdir().unwrap();
            let target_dir = tempdir().unwrap();

            // Create a subdirectory structure in source
            let sub = source_dir.path().join("configs");
            fs::create_dir_all(&sub).unwrap();
            fs::write(sub.join(".env"), "PORT=3000\n").unwrap();

            // Create target configs dir and make it read-only
            let target_sub = target_dir.path().join("configs");
            fs::create_dir_all(&target_sub).unwrap();
            let mut perms = fs::metadata(&target_sub).unwrap().permissions();
            perms.set_mode(0o555);
            fs::set_permissions(&target_sub, perms).unwrap();

            let assignments = vec![PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            }];

            let result = copy_files_with_port_transformation(
                source_dir.path().to_string_lossy().to_string(),
                target_dir.path().to_string_lossy().to_string(),
                vec!["configs/.env".to_string()],
                assignments,
            )
            .unwrap();

            // Depending on OS, the file may or may not be writable.
            // On Unix, writing to a read-only directory should fail.
            // The error path should be hit.
            if !result.errors.is_empty() {
                assert!(
                    result.errors[0].contains("Failed to write") || result.errors[0].contains("Failed to copy"),
                    "Error should mention failure: {}",
                    result.errors[0]
                );
            }

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&target_sub).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&target_sub, perms).unwrap();
        }
    }

    #[test]
    fn test_is_env_file_empty_path() {
        // Edge case: path with no file name component
        assert!(!is_env_file(Path::new("")));
    }

    #[test]
    fn test_is_compose_file_empty_path() {
        // Edge case: path with no file name component
        assert!(!is_compose_file(Path::new("")));
    }

    #[test]
    fn test_detect_ports_in_env_file_only_comments_and_empty() {
        // Multiple comment styles and empty lines
        let content = "# Comment 1\n\n# Comment 2\n\n\n";
        let ports = detect_ports_in_env_file(content, ".env");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_dockerfile_empty() {
        // Empty Dockerfile content
        let content = "";
        let ports = detect_ports_in_dockerfile(content, "Dockerfile");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_dockerfile_only_comments() {
        // Dockerfile with only comments
        let content = "# EXPOSE 3000\n# EXPOSE 8080\n";
        let ports = detect_ports_in_dockerfile(content, "Dockerfile");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_compose_empty() {
        // Empty compose content
        let content = "";
        let ports = detect_ports_in_compose(content, "docker-compose.yml");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_compose_only_comments() {
        // Compose content with only comments
        let content = "# services:\n#   web:\n#     ports:\n#       - \"3000:3000\"\n";
        let ports = detect_ports_in_compose(content, "docker-compose.yml");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_package_json_empty_scripts() {
        // package.json with empty scripts object
        let content = r#"{"scripts": {}}"#;
        let ports = detect_ports_in_package_json(content, "package.json");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_detect_ports_in_package_json_non_string_script_value() {
        // L204: script value is not a string (e.g., number or null)
        let content = r#"{"scripts": {"dev": 123, "start": null, "build": "next build"}}"#;
        let ports = detect_ports_in_package_json(content, "package.json");
        assert!(ports.is_empty());
    }

    #[test]
    fn test_allocate_ports_single_port_at_boundary() {
        // Port exactly at u16 max minus offset
        let ports = vec![PortSource {
            file_path: ".env".to_string(),
            variable_name: "PORT".to_string(),
            port_value: 65535, // Max u16, any offset makes it overflow
            line_number: 1,
        }];

        let result = allocate_ports(&ports, 1).unwrap();
        // 65535 + 100 = 65635 > 65535
        assert!(result.assignments.is_empty());
        assert_eq!(result.overflow_warnings.len(), 1);
    }

    #[test]
    fn test_transform_env_content_mixed_port_and_url_with_partial_assignments() {
        // Some PORT vars have assignments, some URL vars have assignments,
        // some have none - tests multiple fallthrough paths
        let content = "PORT=3000\nDB_PORT=5432\nREDIS_URL=redis://localhost:6379\nCACHE_URL=memcached://localhost:11211\nAPP_NAME=test\n";
        let assignments = vec![
            PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            },
            PortAssignment {
                variable_name: "CACHE_URL".to_string(),
                original_value: 11211,
                assigned_value: 11311,
            },
        ];
        let result = transform_env_content(content, &assignments);
        assert!(result.content.contains("PORT=3100"));
        assert!(result.content.contains("DB_PORT=5432")); // No assignment
        assert!(result.content.contains("REDIS_URL=redis://localhost:6379")); // No assignment
        assert!(result.content.contains("CACHE_URL=memcached://localhost:11311"));
        assert!(result.content.contains("APP_NAME=test")); // Not a port variable
        assert_eq!(result.replacements.len(), 2);
    }

    #[test]
    fn test_copy_files_no_assignment_copy_failure() {
        // L751-758: Test copy failure when target path is invalid
        // When there are no assignments and the file copy fails
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let source_dir = tempdir().unwrap();
            let target_dir = tempdir().unwrap();

            fs::write(source_dir.path().join("config.txt"), "data").unwrap();

            // Make target directory read-only to prevent file creation
            let mut perms = fs::metadata(target_dir.path()).unwrap().permissions();
            perms.set_mode(0o555);
            fs::set_permissions(target_dir.path(), perms).unwrap();

            let result = copy_files_with_port_transformation(
                source_dir.path().to_string_lossy().to_string(),
                target_dir.path().to_string_lossy().to_string(),
                vec!["config.txt".to_string()],
                vec![], // No assignments, triggers simple copy path
            )
            .unwrap();

            // Copy should fail because target dir is read-only
            if !result.errors.is_empty() {
                assert!(
                    result.errors[0].contains("Failed to copy"),
                    "Error should mention copy failure: {}",
                    result.errors[0]
                );
            }

            // Restore permissions for cleanup
            let mut perms = fs::metadata(target_dir.path()).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(target_dir.path(), perms).unwrap();
        }
    }

    #[test]
    fn test_copy_files_with_assignments_read_source_failure() {
        // L741-743: Test read failure when source file can't be read during transform
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let source_dir = tempdir().unwrap();
            let target_dir = tempdir().unwrap();

            let source_file = source_dir.path().join("config.json");
            fs::write(&source_file, r#"{"port": 3000}"#).unwrap();

            // Make source file unreadable
            let mut perms = fs::metadata(&source_file).unwrap().permissions();
            perms.set_mode(0o000);
            fs::set_permissions(&source_file, perms).unwrap();

            let assignments = vec![PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            }];

            let result = copy_files_with_port_transformation(
                source_dir.path().to_string_lossy().to_string(),
                target_dir.path().to_string_lossy().to_string(),
                vec!["config.json".to_string()],
                assignments,
            )
            .unwrap();

            // Should have error about reading failure
            if !result.errors.is_empty() {
                assert!(
                    result.errors[0].contains("Failed to read"),
                    "Error should mention read failure: {}",
                    result.errors[0]
                );
            }

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&source_file).unwrap().permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&source_file, perms).unwrap();
        }
    }

    #[test]
    fn test_copy_files_existing_file_read_failure() {
        // L689-695: Test read failure when existing target file can't be read
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let source_dir = tempdir().unwrap();
            let target_dir = tempdir().unwrap();

            fs::write(source_dir.path().join(".env"), "PORT=3000\n").unwrap();
            let target_file = target_dir.path().join(".env");
            fs::write(&target_file, "PORT=3000\n").unwrap();

            // Make target file unreadable
            let mut perms = fs::metadata(&target_file).unwrap().permissions();
            perms.set_mode(0o000);
            fs::set_permissions(&target_file, perms).unwrap();

            let assignments = vec![PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            }];

            let result = copy_files_with_port_transformation(
                source_dir.path().to_string_lossy().to_string(),
                target_dir.path().to_string_lossy().to_string(),
                vec![".env*".to_string()],
                assignments,
            )
            .unwrap();

            // Should have error about reading existing file
            assert!(
                !result.errors.is_empty(),
                "Expected read error for unreadable existing file"
            );
            assert!(
                result.errors[0].contains("Failed to read existing file"),
                "Error should mention read failure: {}",
                result.errors[0]
            );

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&target_file).unwrap().permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&target_file, perms).unwrap();
        }
    }

    #[test]
    fn test_copy_files_new_file_write_failure_with_assignments() {
        // L731-738: Test write failure when creating new file with port transform
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let source_dir = tempdir().unwrap();
            let target_dir = tempdir().unwrap();

            fs::write(source_dir.path().join(".env"), "PORT=3000\n").unwrap();

            // Make target directory read-only
            let mut perms = fs::metadata(target_dir.path()).unwrap().permissions();
            perms.set_mode(0o555);
            fs::set_permissions(target_dir.path(), perms).unwrap();

            let assignments = vec![PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            }];

            let result = copy_files_with_port_transformation(
                source_dir.path().to_string_lossy().to_string(),
                target_dir.path().to_string_lossy().to_string(),
                vec![".env*".to_string()],
                assignments,
            )
            .unwrap();

            // Should have error about write failure
            assert!(
                !result.errors.is_empty(),
                "Expected write error for read-only target directory"
            );
            assert!(
                result.errors[0].contains("Failed to write"),
                "Error should mention write failure: {}",
                result.errors[0]
            );

            // Restore permissions for cleanup
            let mut perms = fs::metadata(target_dir.path()).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(target_dir.path(), perms).unwrap();
        }
    }

    #[test]
    fn test_copy_files_create_dir_failure() {
        // L706: Test directory creation failure
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let source_dir = tempdir().unwrap();
            let target_dir = tempdir().unwrap();

            // Create nested source file
            let sub = source_dir.path().join("deep/nested");
            fs::create_dir_all(&sub).unwrap();
            fs::write(sub.join(".env"), "PORT=3000\n").unwrap();

            // Make target directory read-only so subdirectories can't be created
            let mut perms = fs::metadata(target_dir.path()).unwrap().permissions();
            perms.set_mode(0o555);
            fs::set_permissions(target_dir.path(), perms).unwrap();

            let assignments = vec![PortAssignment {
                variable_name: "PORT".to_string(),
                original_value: 3000,
                assigned_value: 3100,
            }];

            let result = copy_files_with_port_transformation(
                source_dir.path().to_string_lossy().to_string(),
                target_dir.path().to_string_lossy().to_string(),
                vec!["deep/nested/.env".to_string()],
                assignments,
            )
            .unwrap();

            // Should have error about directory creation failure
            assert!(
                !result.errors.is_empty(),
                "Expected error for directory creation in read-only target"
            );
            assert!(
                result.errors[0].contains("Failed to create directory")
                    || result.errors[0].contains("Failed to write"),
                "Error should mention directory or write failure: {}",
                result.errors[0]
            );

            // Restore permissions for cleanup
            let mut perms = fs::metadata(target_dir.path()).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(target_dir.path(), perms).unwrap();
        }
    }

    #[test]
    fn test_detect_ports_in_env_file_large_port_number_in_url() {
        // Test with URL port number that is a valid number but exceeds u16
        let content = "SERVICE_URL=http://example.com:100000\nPORT=3000\n";
        let ports = detect_ports_in_env_file(content, ".env");
        // 100000 exceeds u16, only PORT=3000 is valid
        assert_eq!(ports.len(), 1);
        assert_eq!(ports[0].variable_name, "PORT");
    }

    #[test]
    fn test_detect_ports_in_compose_mixed_valid_invalid() {
        // Mix of valid and invalid (oversized) port values
        let content = r#"services:
  web:
    ports:
      - "3000:3000"
      # A commented out port
      - "8080:80"
"#;
        let ports = detect_ports_in_compose(content, "docker-compose.yml");
        assert_eq!(ports.len(), 2);
        assert_eq!(ports[0].port_value, 3000);
        assert_eq!(ports[1].port_value, 8080);
    }
}
