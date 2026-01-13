//! Integration tests for PTY/terminal functionality
//!
//! These tests verify that the PTY operations work correctly
//! without the Tauri command wrapper.

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::Write;

#[test]
fn test_pty_creation() {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY");

    // Verify we can get a reader and writer
    let _reader = pair.master.try_clone_reader().expect("Failed to get reader");
    let _writer = pair.master.take_writer().expect("Failed to get writer");
}

#[test]
fn test_pty_spawn_simple_command() {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY");

    // Spawn a simple command that exits immediately
    let cmd = CommandBuilder::new("true");
    let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn command");

    // Wait for command to complete
    let status = child.wait().expect("Failed to wait for child");
    assert!(status.success());
}

#[test]
fn test_pty_spawn_false_command() {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY");

    // Spawn a command that fails
    let cmd = CommandBuilder::new("false");
    let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn command");

    // Wait for command to complete
    let status = child.wait().expect("Failed to wait for child");
    assert!(!status.success());
}

#[test]
fn test_pty_write_input() {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY");

    let mut writer = pair.master.take_writer().expect("Failed to get writer");

    // Write some data - should not fail
    let result = writer.write_all(b"test data\n");
    assert!(result.is_ok());

    let result = writer.flush();
    assert!(result.is_ok());
}

#[test]
fn test_pty_resize() {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY");

    // Resize the PTY
    let result = pair.master.resize(PtySize {
        rows: 48,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    });

    assert!(result.is_ok());
}

#[test]
fn test_pty_resize_multiple_times() {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY");

    // Resize multiple times
    for (rows, cols) in [(30, 100), (40, 120), (24, 80)] {
        let result = pair.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
        assert!(result.is_ok());
    }
}

#[test]
fn test_pty_with_cwd() {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY");

    // Use a simple command with cwd set
    let mut cmd = CommandBuilder::new("true");
    cmd.cwd("/tmp");

    let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn command");
    let status = child.wait().expect("Failed to wait for child");
    assert!(status.success());
}

#[test]
fn test_pty_with_env() {
    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("Failed to create PTY");

    // Use a simple command with env set
    let mut cmd = CommandBuilder::new("true");
    cmd.env("TEST_VAR", "test_value_123");
    cmd.env("TERM", "xterm-256color");

    let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn command");
    let status = child.wait().expect("Failed to wait for child");
    assert!(status.success());
}

#[test]
fn test_pty_different_sizes() {
    let pty_system = native_pty_system();

    // Test various sizes
    for (rows, cols) in [(24, 80), (50, 200), (10, 40), (100, 300)] {
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("Failed to create PTY");

        let cmd = CommandBuilder::new("true");
        let mut child = pair.slave.spawn_command(cmd).expect("Failed to spawn command");
        let status = child.wait().expect("Failed to wait for child");
        assert!(status.success());
    }
}
