use super::compose_isolation::{
    ComposeNameReplacement, ComposeTransformResult, DetectedComposeFiles,
};

#[tauri::command]
pub fn detect_compose_files(dir_path: String) -> Result<DetectedComposeFiles, String> {
    super::compose_isolation::scan_compose_files(&dir_path)
}

#[tauri::command]
pub fn apply_compose_isolation(
    worktree_path: String,
    replacements: Vec<ComposeNameReplacement>,
) -> Result<ComposeTransformResult, String> {
    Ok(super::compose_isolation::apply_compose_name_isolation(
        &worktree_path,
        &replacements,
    ))
}
