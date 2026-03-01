//! Performance measurement module (debug builds only)
//!
//! Provides memory metrics and command timing tracking for development.
//! All functionality is compiled out in release builds.

use serde::Serialize;

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
    use std::sync::Mutex;
    use std::time::Instant;
    use sysinfo::{Pid, System};

    lazy_static::lazy_static! {
        static ref PERF_TRACKER: Mutex<PerformanceTracker> = Mutex::new(PerformanceTracker::new());
    }

    /// Performance tracker state
    pub struct PerformanceTracker {
        start_time: Instant,
        timings: Vec<CommandTiming>,
    }

    impl Default for PerformanceTracker {
        fn default() -> Self {
            Self::new()
        }
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

    /// Get memory usage for a specific process by PID
    pub fn get_memory_usage_for_pid(pid: Pid) -> MemoryMetrics {
        let mut sys = System::new();

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

    /// Get current memory usage for this process
    pub fn get_memory_usage() -> MemoryMetrics {
        let pid = Pid::from_u32(std::process::id());
        get_memory_usage_for_pid(pid)
    }

    /// Record a command timing
    pub fn record_timing(command: &str, duration_ms: f64) {
        if let Ok(mut tracker) = PERF_TRACKER.lock() {
            tracker.record(command, duration_ms);
        }
    }

    /// Build a performance report from optional tracker data.
    /// Returns a report with empty timings and zero uptime if tracker data is None.
    pub fn build_report(
        tracker_data: Option<(Vec<CommandTiming>, u64)>,
    ) -> PerformanceReport {
        let (timings, uptime) = tracker_data.unwrap_or_else(|| (Vec::new(), 0));

        PerformanceReport {
            memory: get_memory_usage(),
            command_timings: timings,
            app_uptime_ms: uptime,
        }
    }

    /// Get full performance report
    pub fn get_report() -> PerformanceReport {
        let tracker_data = if let Ok(tracker) = PERF_TRACKER.lock() {
            Some((tracker.get_timings(), tracker.uptime_ms()))
        } else {
            None
        };

        build_report(tracker_data)
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

    #[cfg(debug_assertions)]
    #[test]
    fn test_performance_tracker_record_limit() {
        use super::debug_impl::PerformanceTracker;

        let mut tracker = PerformanceTracker::new();

        // Record more than 1000 entries
        for i in 0..1005 {
            tracker.record(&format!("cmd_{}", i), i as f64);
        }

        let timings = tracker.get_timings();
        // Should keep only the last 1000 entries
        assert_eq!(timings.len(), 1000);
        // The oldest entries (0-4) should have been removed
        assert_eq!(timings[0].command, "cmd_5");
        assert_eq!(timings[999].command, "cmd_1004");
    }

    #[test]
    fn test_performance_report_serialization() {
        let report = PerformanceReport {
            memory: MemoryMetrics {
                rss: 1024,
                vms: 2048,
                platform: "macos".to_string(),
            },
            command_timings: vec![
                CommandTiming {
                    command: "open_file".to_string(),
                    duration_ms: 15.5,
                    timestamp_ms: 100,
                },
                CommandTiming {
                    command: "save_file".to_string(),
                    duration_ms: 8.2,
                    timestamp_ms: 200,
                },
            ],
            app_uptime_ms: 5000,
        };

        let json = serde_json::to_string(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["memory"]["rss"], 1024);
        assert_eq!(parsed["memory"]["vms"], 2048);
        assert_eq!(parsed["memory"]["platform"], "macos");
        assert_eq!(parsed["command_timings"].as_array().unwrap().len(), 2);
        assert_eq!(parsed["command_timings"][0]["command"], "open_file");
        assert_eq!(parsed["command_timings"][1]["command"], "save_file");
        assert_eq!(parsed["app_uptime_ms"], 5000);
    }

    #[test]
    fn test_memory_metrics_serialization() {
        let metrics = MemoryMetrics {
            rss: 4096,
            vms: 8192,
            platform: "linux".to_string(),
        };

        let json = serde_json::to_string(&metrics).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["rss"], 4096);
        assert_eq!(parsed["vms"], 8192);
        assert_eq!(parsed["platform"], "linux");
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_clear_timings_empties_all() {
        // Record some timings
        record_timing("cmd_a", 1.0);
        record_timing("cmd_b", 2.0);

        // Clear
        clear_timings();

        // Report should have no timings
        let report = get_report();
        assert!(
            report.command_timings.is_empty(),
            "Timings should be empty after clear"
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_performance_tracker_new() {
        use super::debug_impl::PerformanceTracker;

        let tracker = PerformanceTracker::new();

        // Initial state: no timings
        let timings = tracker.get_timings();
        assert!(timings.is_empty(), "New tracker should have no timings");

        // Uptime should be very small (just created)
        let uptime = tracker.uptime_ms();
        assert!(uptime < 1000, "New tracker uptime should be very small");
    }

    #[test]
    fn test_command_timing_serialization() {
        let timing = CommandTiming {
            command: "read_dir".to_string(),
            duration_ms: 3.14,
            timestamp_ms: 42,
        };

        let json = serde_json::to_string(&timing).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["command"], "read_dir");
        assert!((parsed["duration_ms"].as_f64().unwrap() - 3.14).abs() < f64::EPSILON);
        assert_eq!(parsed["timestamp_ms"], 42);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_get_memory_usage_for_nonexistent_pid_returns_zeros() {
        use sysinfo::Pid;
        use super::debug_impl::get_memory_usage_for_pid;

        // Use a very high PID that is extremely unlikely to exist
        let fake_pid = Pid::from_u32(u32::MAX - 1);
        let metrics = get_memory_usage_for_pid(fake_pid);

        assert_eq!(metrics.rss, 0, "RSS should be 0 for nonexistent process");
        assert_eq!(metrics.vms, 0, "VMS should be 0 for nonexistent process");
        assert_eq!(
            metrics.platform,
            std::env::consts::OS,
            "Platform should still be set correctly"
        );
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_build_report_with_none_returns_empty_timings() {
        use super::debug_impl::build_report;

        // Simulate the fallback path (tracker data unavailable)
        let report = build_report(None);

        assert!(
            report.command_timings.is_empty(),
            "Timings should be empty when tracker data is None"
        );
        assert_eq!(
            report.app_uptime_ms, 0,
            "Uptime should be 0 when tracker data is None"
        );
        // Memory should still be populated from the current process
        assert!(!report.memory.platform.is_empty(), "Platform should be set");
    }

    #[cfg(debug_assertions)]
    #[test]
    fn test_build_report_with_some_uses_provided_data() {
        use super::debug_impl::build_report;

        let timings = vec![
            CommandTiming {
                command: "test_cmd".to_string(),
                duration_ms: 5.0,
                timestamp_ms: 100,
            },
        ];
        let report = build_report(Some((timings, 9999)));

        assert_eq!(report.command_timings.len(), 1);
        assert_eq!(report.command_timings[0].command, "test_cmd");
        assert_eq!(report.app_uptime_ms, 9999);
    }
}
