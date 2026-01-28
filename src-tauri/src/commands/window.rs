use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder};

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(1);

#[tauri::command]
pub fn create_window(
    app: AppHandle,
    x: Option<i32>,
    y: Option<i32>,
    width: Option<f64>,
    height: Option<f64>,
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

    let mut builder = WebviewWindowBuilder::new(&app, &label, WebviewUrl::default())
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
