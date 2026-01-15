//! Performance measurement module (debug builds only)
//!
//! Provides memory metrics and command timing tracking for development.
//! All functionality is compiled out in release builds.

use serde::Serialize;
use std::sync::Mutex;
use std::time::Instant;

/// Memory usage metrics
#[derive(Debug, Clone, Serialize, Default)]
pub struct MemoryMetrics {
    /// Resident Set Size in bytes
    pub rss: u64,
    /// Virtual memory size in bytes
    pub vms: u64,
    /// Platform identifier
    pub platform: String,
}

/// Single command timing entry
#[derive(Debug, Clone, Serialize)]
pub struct CommandTiming {
    /// Command name
    pub command: String,
    /// Duration in milliseconds
    pub duration_ms: f64,
    /// Timestamp (ms since tracker start)
    pub timestamp_ms: u64,
}

/// Full performance report
#[derive(Debug, Clone, Serialize)]
pub struct PerformanceReport {
    /// Memory metrics
    pub memory: MemoryMetrics,
    /// Command timings
    pub command_timings: Vec<CommandTiming>,
    /// App uptime in milliseconds
    pub app_uptime_ms: u64,
}

// ============================================================================
// Debug build implementation
// ============================================================================

#[cfg(debug_assertions)]
mod debug_impl {
    use super::*;
    use sysinfo::{Pid, System};

    lazy_static::lazy_static! {
        static ref PERF_TRACKER: Mutex<PerformanceTracker> = Mutex::new(PerformanceTracker::new());
    }

    /// Performance tracker state
    pub struct PerformanceTracker {
        start_time: Instant,
        timings: Vec<CommandTiming>,
    }

    impl PerformanceTracker {
        pub fn new() -> Self {
            Self {
                start_time: Instant::now(),
                timings: Vec::new(),
            }
        }

        pub fn record(&mut self, command: &str, duration_ms: f64) {
            let timestamp_ms = self.start_time.elapsed().as_millis() as u64;
            self.timings.push(CommandTiming {
                command: command.to_string(),
                duration_ms,
                timestamp_ms,
            });

            // Keep only last 1000 entries to prevent memory growth
            if self.timings.len() > 1000 {
                self.timings.remove(0);
            }
        }

        pub fn get_timings(&self) -> Vec<CommandTiming> {
            self.timings.clone()
        }

        pub fn uptime_ms(&self) -> u64 {
            self.start_time.elapsed().as_millis() as u64
        }

        pub fn clear(&mut self) {
            self.timings.clear();
        }
    }

    /// Get current memory usage for this process
    pub fn get_memory_usage() -> MemoryMetrics {
        let mut sys = System::new();
        let pid = Pid::from_u32(std::process::id());

        // Refresh only the specific process
        sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]));

        if let Some(process) = sys.process(pid) {
            MemoryMetrics {
                rss: process.memory(),
                vms: process.virtual_memory(),
                platform: std::env::consts::OS.to_string(),
            }
        } else {
            MemoryMetrics {
                rss: 0,
                vms: 0,
                platform: std::env::consts::OS.to_string(),
            }
        }
    }

    /// Record a command timing
    pub fn record_timing(command: &str, duration_ms: f64) {
        if let Ok(mut tracker) = PERF_TRACKER.lock() {
            tracker.record(command, duration_ms);
        }
    }

    /// Get full performance report
    pub fn get_report() -> PerformanceReport {
        let (timings, uptime) = if let Ok(tracker) = PERF_TRACKER.lock() {
            (tracker.get_timings(), tracker.uptime_ms())
        } else {
            (Vec::new(), 0)
        };

        PerformanceReport {
            memory: get_memory_usage(),
            command_timings: timings,
            app_uptime_ms: uptime,
        }
    }

    /// Clear recorded timings
    pub fn clear_timings() {
        if let Ok(mut tracker) = PERF_TRACKER.lock() {
            tracker.clear();
        }
    }
}

// ============================================================================
// Release build implementation (no-op)
// ============================================================================

#[cfg(not(debug_assertions))]
mod release_impl {
    use super::*;

    /// No-op in release builds
    pub fn get_memory_usage() -> MemoryMetrics {
        MemoryMetrics::default()
    }

    /// No-op in release builds
    pub fn record_timing(_command: &str, _duration_ms: f64) {}

    /// No-op in release builds
    pub fn get_report() -> PerformanceReport {
        PerformanceReport {
            memory: MemoryMetrics::default(),
            command_timings: Vec::new(),
            app_uptime_ms: 0,
        }
    }

    /// No-op in release builds
    pub fn clear_timings() {}
}

// ============================================================================
// Public API
// ============================================================================

#[cfg(debug_assertions)]
pub use debug_impl::*;

#[cfg(not(debug_assertions))]
pub use release_impl::*;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_metrics_default() {
        let metrics = MemoryMetrics::default();
        assert_eq!(metrics.rss, 0);
        assert_eq!(metrics.vms, 0);
        assert!(metrics.platform.is_empty());
    }

    #[test]
    fn test_command_timing_creation() {
        let timing = CommandTiming {
            command: "test_command".to_string(),
            duration_ms: 42.5,
            timestamp_ms: 1000,
        };
        assert_eq!(timing.command, "test_command");
        assert!((timing.duration_ms - 42.5).abs() < f64::EPSILON);
        assert_eq!(timing.timestamp_ms, 1000);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_get_memory_usage_returns_valid_data() {
        let metrics = get_memory_usage();
        // RSS should be > 0 for a running process
        assert!(metrics.rss > 0, "RSS should be greater than 0");
        assert!(!metrics.platform.is_empty(), "Platform should not be empty");
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_record_and_get_timings() {
        // Clear any existing timings
        clear_timings();

        // Record some timings
        record_timing("test_cmd_1", 10.0);
        record_timing("test_cmd_2", 20.0);

        // Get report and verify
        let report = get_report();
        assert!(report.command_timings.len() >= 2);

        // Find our timings
        let has_cmd_1 = report
            .command_timings
            .iter()
            .any(|t| t.command == "test_cmd_1");
        let has_cmd_2 = report
            .command_timings
            .iter()
            .any(|t| t.command == "test_cmd_2");

        assert!(has_cmd_1, "Should have test_cmd_1");
        assert!(has_cmd_2, "Should have test_cmd_2");

        // Clean up
        clear_timings();
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_uptime_increases() {
        let report1 = get_report();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let report2 = get_report();

        assert!(
            report2.app_uptime_ms >= report1.app_uptime_ms,
            "Uptime should increase over time"
        );
    }
}
