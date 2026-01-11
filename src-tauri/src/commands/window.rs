use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

static WINDOW_COUNTER: AtomicU32 = AtomicU32::new(1);

#[tauri::command]
pub fn create_window(app: AppHandle) -> Result<(), String> {
    let id = WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
    let label = format!("window-{}", id);

    // Try to get the size of an existing window, or use default
    let (width, height) = app
        .webview_windows()
        .values()
        .next()
        .and_then(|w| w.inner_size().ok())
        .map(|size| (size.width as f64, size.height as f64))
        .unwrap_or((1200.0, 800.0));

    WebviewWindowBuilder::new(&app, &label, WebviewUrl::default())
        .title("Kiri")
        .inner_size(width, height)
        .min_inner_size(600.0, 400.0)
        .visible(true)
        .focused(true)
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_counter_increments() {
        let initial = WINDOW_COUNTER.load(Ordering::SeqCst);
        WINDOW_COUNTER.fetch_add(1, Ordering::SeqCst);
        let next = WINDOW_COUNTER.load(Ordering::SeqCst);
        assert_eq!(next, initial + 1);
    }
}
