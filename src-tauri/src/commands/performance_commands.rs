//! Tauri command wrappers for performance measurement
//!
//! These commands are only available in debug builds.

use super::performance::{self, MemoryMetrics, PerformanceReport};

/// Get current memory metrics
///
/// Returns memory usage information for the current process.
#[tauri::command]
pub fn get_memory_metrics() -> Result<MemoryMetrics, String> {
    Ok(performance::get_memory_usage())
}

/// Get full performance report
///
/// Returns memory metrics, command timings, and app uptime.
#[tauri::command]
pub fn get_performance_report() -> Result<PerformanceReport, String> {
    Ok(performance::get_report())
}

/// Record a command timing from the frontend
///
/// This allows the frontend to report operation timings to the backend
/// for centralized performance tracking.
#[tauri::command]
pub fn record_command_timing(command: String, duration_ms: f64) -> Result<(), String> {
    performance::record_timing(&command, duration_ms);
    Ok(())
}

/// Clear all recorded timings
///
/// Useful for resetting performance tracking between sessions.
#[tauri::command]
pub fn clear_performance_timings() -> Result<(), String> {
    performance::clear_timings();
    Ok(())
}
