use super::git_worktree::CopyResult;
use super::port_isolation::{DetectedPorts, PortAllocationResult, PortAssignment, PortSource};

#[tauri::command]
pub fn detect_ports(dir_path: String) -> Result<DetectedPorts, String> {
    super::port_isolation::detect_all_ports(dir_path)
}

#[tauri::command]
pub fn allocate_worktree_ports(
    ports: Vec<PortSource>,
    worktree_index: u16,
) -> Result<PortAllocationResult, String> {
    super::port_isolation::allocate_ports(&ports, worktree_index)
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
