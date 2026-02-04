use super::git_worktree::CopyResult;
use super::port_isolation::{
    CustomPortRule, CustomRuleReplacement, DetectedPorts, PortAllocationResult, PortAssignment,
    PortSource,
};

#[tauri::command]
pub fn detect_ports(dir_path: String) -> Result<DetectedPorts, String> {
    super::port_isolation::detect_all_ports(dir_path)
}

#[tauri::command]
pub fn allocate_worktree_ports(
    ports: Vec<PortSource>,
    start_port: u16,
) -> Result<PortAllocationResult, String> {
    super::port_isolation::allocate_ports(&ports, start_port)
}

#[tauri::command]
pub fn copy_files_with_ports(
    source_path: String,
    target_path: String,
    patterns: Vec<String>,
    assignments: Vec<PortAssignment>,
) -> Result<CopyResult, String> {
    super::port_isolation::copy_files_with_port_transformation(
        source_path,
        target_path,
        patterns,
        assignments,
    )
}

#[tauri::command]
pub fn apply_port_custom_rules(
    source_path: String,
    target_path: String,
    rules: Vec<CustomPortRule>,
    port_offset: u16,
) -> Result<Vec<CustomRuleReplacement>, String> {
    super::port_isolation::apply_custom_rules(source_path, target_path, rules, port_offset)
}
