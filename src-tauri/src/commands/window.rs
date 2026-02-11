use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(1);

/// Registry to track which windows are associated with which project paths
#[derive(Default)]
pub struct WindowRegistry {
    /// Maps project paths to window labels
    path_to_label: HashMap<String, String>,
    /// Maps window labels to project paths
    label_to_path: HashMap<String, String>,
}

impl WindowRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a window with a project path
    pub fn register(&mut self, label: &str, path: &str) {
        self.path_to_label.insert(path.to_string(), label.to_string());
        self.label_to_path.insert(label.to_string(), path.to_string());
    }

    /// Unregister a window by its label
    pub fn unregister_by_label(&mut self, label: &str) {
        if let Some(path) = self.label_to_path.remove(label) {
            self.path_to_label.remove(&path);
        }
    }

    /// Get the window label for a project path
    pub fn get_label_for_path(&self, path: &str) -> Option<&String> {
        self.path_to_label.get(path)
    }
}

pub type WindowRegistryState = Arc<Mutex<WindowRegistry>>;

/// Internal implementation of window creation (used by both command and menu)
pub fn create_window_impl(
    app: &AppHandle,
    registry: Option<&WindowRegistryState>,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<f64>,
    height: Option<f64>,
    project_path: Option<String>,
    window_index: Option<i32>,
) -> Result<(), String> {
    let id = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
    let label = format!("window-{}", id);

    // Use provided size or try to get the size of an existing window, or use default
    let (win_width, win_height) = match (width, height) {
        (Some(w), Some(h)) => (w, h),
        _ => app
            .webview_windows()
            .values()
            .next()
            .and_then(|w| w.inner_size().ok())
            .map(|size| (size.width as f64, size.height as f64))
            .unwrap_or((1200.0, 800.0)),
    };

    // Build URL with optional parameters
    let mut params = Vec::new();
    if let Some(path) = &project_path {
        params.push(format!("project={}", urlencoding::encode(path)));
    }
    if let Some(idx) = window_index {
        params.push(format!("windowIndex={}", idx));
    }
    let url = if params.is_empty() {
        WebviewUrl::default()
    } else {
        WebviewUrl::App(format!("?{}", params.join("&")).into())
    };

    let mut builder = WebviewWindowBuilder::new(app, &label, url)
        .title("kiri")
        .inner_size(win_width, win_height)
        .min_inner_size(600.0, 400.0)
        .visible(true)
        .focused(true);

    // Set position if provided
    if let (Some(pos_x), Some(pos_y)) = (x, y) {
        builder = builder.position(pos_x as f64, pos_y as f64);
    }

    builder.build().map_err(|e| e.to_string())?;

    // Register the window with its project path
    if let (Some(path), Some(registry)) = (project_path, registry) {
        if let Ok(mut reg) = registry.lock() {
            reg.register(&label, &path);
        }
    }

    Ok(())
}

#[tauri::command]
pub fn create_window(
    app: AppHandle,
    registry: tauri::State<WindowRegistryState>,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<f64>,
    height: Option<f64>,
    project_path: Option<String>,
    window_index: Option<i32>,
) -> Result<(), String> {
    create_window_impl(&app, Some(&registry), x, y, width, height, project_path, window_index)
}

/// Focus an existing window for the given project path, or create a new one if not found
#[tauri::command]
pub fn focus_or_create_window(
    app: AppHandle,
    registry: tauri::State<WindowRegistryState>,
    project_path: String,
) -> Result<bool, String> {
    // Check if a window already exists for this path
    let existing_label = {
        let reg = registry.lock().map_err(|e| format!("Lock error: {}", e))?;
        reg.get_label_for_path(&project_path).cloned()
    };

    if let Some(label) = existing_label {
        // Check if the window still exists
        if let Some(window) = app.get_webview_window(&label) {
            // Window exists, focus it
            window.set_focus().map_err(|e| format!("Failed to focus window: {}", e))?;
            return Ok(true); // Indicates existing window was focused
        } else {
            // Window no longer exists, clean up registry
            if let Ok(mut reg) = registry.lock() {
                reg.unregister_by_label(&label);
            }
        }
    }

    // No existing window, create a new one
    create_window(app, registry, None, None, None, None, Some(project_path), None)?;
    Ok(false) // Indicates new window was created
}

/// Register a window with a project path (for windows not created via create_window)
#[tauri::command]
pub fn register_window(
    registry: tauri::State<WindowRegistryState>,
    label: String,
    project_path: String,
) -> Result<(), String> {
    if let Ok(mut reg) = registry.lock() {
        reg.register(&label, &project_path);
    }
    Ok(())
}

/// Unregister a window from the registry (called when window is closed)
#[tauri::command]
pub fn unregister_window(
    registry: tauri::State<WindowRegistryState>,
    label: String,
) -> Result<(), String> {
    if let Ok(mut reg) = registry.lock() {
        reg.unregister_by_label(&label);
    }
    Ok(())
}

/// Get window geometry (position and size) for the specified window label
/// Returns logical coordinates for cross-platform consistency
#[tauri::command]
pub fn get_window_geometry(
    app: AppHandle,
    label: String,
) -> Result<(f64, f64, f64, f64), String> {
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("Window '{}' not found", label))?;

    let scale_factor = window.scale_factor().unwrap_or(1.0);

    let position = window
        .outer_position()
        .map_err(|e| format!("Failed to get position: {}", e))?;
    let size = window
        .inner_size()
        .map_err(|e| format!("Failed to get size: {}", e))?;

    // Convert physical to logical coordinates
    let logical_x = position.x as f64 / scale_factor;
    let logical_y = position.y as f64 / scale_factor;
    let logical_width = size.width as f64 / scale_factor;
    let logical_height = size.height as f64 / scale_factor;

    Ok((logical_x, logical_y, logical_width, logical_height))
}

/// Set window geometry (position and size) for the specified window label
/// Accepts logical coordinates for cross-platform consistency
#[tauri::command]
pub fn set_window_geometry(
    app: AppHandle,
    label: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) -> Result<(), String> {
    let window = app
        .get_webview_window(&label)
        .ok_or_else(|| format!("Window '{}' not found", label))?;

    window
        .set_position(LogicalPosition::new(x, y))
        .map_err(|e| format!("Failed to set position: {}", e))?;

    window
        .set_size(LogicalSize::new(width, height))
        .map_err(|e| format!("Failed to set size: {}", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_window_counter_increments() {
        // fetch_add returns the previous value and atomically increments
        let prev = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
        let current = WINDOW_COUNTER.load(Ordering::SeqCst);
        // current should be at least prev + 1 (other tests may also increment)
        assert!(current > prev);
    }

    #[test]
    fn test_window_counter_is_atomic() {
        // Test atomicity by verifying all fetch_add calls return unique values
        // This proves no race conditions occur
        let results = Arc::new(std::sync::Mutex::new(Vec::new()));

        let handles: Vec<_> = (0..10)
            .map(|_| {
                let results = Arc::clone(&results);
                thread::spawn(move || {
                    let prev = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
                    results.lock().unwrap().push(prev);
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        // All returned values should be unique (no two threads got same value)
        let results = results.lock().unwrap();
        let unique: HashSet<_> = results.iter().collect();
        assert_eq!(unique.len(), 10, "All fetch_add results should be unique");
    }
}
