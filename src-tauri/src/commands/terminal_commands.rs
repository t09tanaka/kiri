//! Tauri command wrappers for terminal functionality
//! These are thin wrappers that delegate to the core logic in terminal.rs

use super::terminal::{
    create_pty_size, find_utf8_boundary, open_pty_with_shell, resolve_cwd, resolve_terminal_size,
    PtyInstance, TerminalOutput, TerminalState,
};
use serde::Serialize;
use std::io::{Read, Write};
use std::str;
use std::thread;
use tauri::{AppHandle, Emitter};

#[tauri::command]
pub fn create_terminal(
    app: AppHandle,
    state: tauri::State<'_, TerminalState>,
    cwd: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<u32, String> {
    let (initial_cols, initial_rows) = resolve_terminal_size(cols, rows);
    let resolved_cwd = resolve_cwd(cwd);

    // Use extracted function for PTY creation
    let pty_with_shell = open_pty_with_shell(initial_cols, initial_rows, resolved_cwd.as_deref())?;

    let mut reader = pty_with_shell
        .pair
        .master
        .try_clone_reader()
        .map_err(|e| e.to_string())?;
    let writer = pty_with_shell
        .pair
        .master
        .take_writer()
        .map_err(|e| e.to_string())?;

    let mut manager = state.lock().map_err(|e| e.to_string())?;
    let id = manager.next_id;
    manager.next_id += 1;

    // Get shell PID for foreground process checking
    let shell_pid = pty_with_shell.child.process_id();

    // Send bindkey -e to enable emacs mode for keyboard navigation
    // This ensures Option+Arrow and Cmd+Arrow work correctly regardless of user's shell config
    // Use clear to hide the command from the user
    let mut writer = writer;
    let _ = writer.write_all(b"bindkey -e && clear\n");
    let _ = writer.flush();

    manager.instances.insert(
        id,
        PtyInstance {
            master: pty_with_shell.pair.master,
            writer,
            child: pty_with_shell.child,
            shell_pid,
        },
    );

    // Spawn thread to read PTY output
    let terminal_id = id;
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        // Buffer for incomplete UTF-8 sequences from previous reads
        let mut pending: Vec<u8> = Vec::new();

        loop {
            // Calculate where to start reading (after any pending bytes)
            let read_start = pending.len();
            let read_len = buf.len() - read_start;

            if read_len == 0 {
                // Buffer is full of pending bytes, which shouldn't happen
                // Reset and continue
                pending.clear();
                continue;
            }

            // Copy pending bytes to the start of buffer
            buf[..read_start].copy_from_slice(&pending);

            match reader.read(&mut buf[read_start..]) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let total_len = read_start + n;
                    let data_slice = &buf[..total_len];

                    // Find the last valid UTF-8 boundary
                    let valid_len = find_utf8_boundary(data_slice);

                    if valid_len > 0 {
                        // Safety: we just validated this is valid UTF-8
                        let data = unsafe { str::from_utf8_unchecked(&data_slice[..valid_len]) };
                        let _ = app.emit(
                            "terminal-output",
                            TerminalOutput {
                                id: terminal_id,
                                data: data.to_string(),
                            },
                        );
                    }

                    // Save any incomplete bytes for the next read
                    pending.clear();
                    if valid_len < total_len {
                        pending.extend_from_slice(&data_slice[valid_len..]);
                    }
                }
                Err(_) => break,
            }
        }
    });

    Ok(id)
}

#[tauri::command]
pub fn write_terminal(
    state: tauri::State<'_, TerminalState>,
    id: u32,
    data: String,
) -> Result<(), String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;

    if let Some(instance) = manager.instances.get_mut(&id) {
        instance
            .writer
            .write_all(data.as_bytes())
            .map_err(|e| e.to_string())?;
        instance.writer.flush().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("Terminal {} not found", id))
    }
}

#[tauri::command]
pub fn resize_terminal(
    state: tauri::State<'_, TerminalState>,
    id: u32,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    let manager = state.lock().map_err(|e| e.to_string())?;

    if let Some(instance) = manager.instances.get(&id) {
        instance
            .master
            .resize(create_pty_size(cols, rows))
            .map_err(|e| e.to_string())?;
        log::info!("Resized terminal {} to {}x{}", id, cols, rows);
        Ok(())
    } else {
        Err(format!("Terminal {} not found", id))
    }
}

#[tauri::command]
pub fn close_terminal(state: tauri::State<'_, TerminalState>, id: u32) -> Result<(), String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;

    if manager.instances.remove(&id).is_some() {
        Ok(())
    } else {
        Err(format!("Terminal {} not found", id))
    }
}

/// Process info returned by get_terminal_process_info
#[derive(Debug, Clone, Serialize)]
pub struct TerminalProcessInfo {
    pub name: String,
    pub memory_bytes: u64,
}

/// Get the foreground process name and total memory usage for a terminal
/// Returns the process name and combined memory of shell + all child processes
#[tauri::command]
pub fn get_terminal_process_info(
    state: tauri::State<'_, TerminalState>,
    id: u32,
) -> Result<TerminalProcessInfo, String> {
    let default_info = TerminalProcessInfo {
        name: "Terminal".to_string(),
        memory_bytes: 0,
    };

    let mut manager = state.lock().map_err(|e| e.to_string())?;

    if let Some(instance) = manager.instances.get_mut(&id) {
        // Check if shell has exited
        match instance.child.try_wait() {
            Ok(Some(_)) => return Ok(default_info),
            Ok(None) => {}
            Err(_) => return Ok(default_info),
        }

        if let Some(shell_pid) = instance.shell_pid {
            use sysinfo::System;

            let mut sys = System::new();
            sys.refresh_processes(sysinfo::ProcessesToUpdate::All);

            // Collect memory from shell + all descendants
            let mut total_memory: u64 = 0;

            // Add shell process memory
            if let Some(shell_proc) = sys.process(sysinfo::Pid::from_u32(shell_pid)) {
                total_memory += shell_proc.memory();
            }

            // Find child processes and sum their memory
            let child_processes: Vec<_> = sys
                .processes()
                .values()
                .filter(|proc| {
                    proc.parent()
                        .map(|parent_pid| parent_pid.as_u32() == shell_pid)
                        .unwrap_or(false)
                })
                .collect();

            for child in &child_processes {
                total_memory += child.memory();
            }

            // Determine the display name
            let name = if let Some(child) = child_processes.first() {
                child.name().to_string_lossy().to_string()
            } else if let Some(shell_proc) = sys.process(sysinfo::Pid::from_u32(shell_pid)) {
                shell_proc.name().to_string_lossy().to_string()
            } else {
                "Terminal".to_string()
            };

            Ok(TerminalProcessInfo {
                name,
                memory_bytes: total_memory,
            })
        } else {
            Ok(default_info)
        }
    } else {
        Ok(default_info)
    }
}

/// Get the name of the foreground process running in a terminal
/// Returns the child process name if a command is running (e.g., "vim", "cargo"),
/// the shell name if idle (e.g., "zsh"), or "Terminal" if unavailable
#[tauri::command]
pub fn get_foreground_process_name(
    state: tauri::State<'_, TerminalState>,
    id: u32,
) -> Result<String, String> {
    // Delegate to get_terminal_process_info for consistency
    let info = get_terminal_process_info(state, id)?;
    Ok(info.name)
}

/// Check if a terminal has a foreground process running (command in execution)
/// Returns true if there's a child process of the shell (command running),
/// false if the shell is idle (waiting at prompt)
#[tauri::command]
pub fn is_terminal_alive(state: tauri::State<'_, TerminalState>, id: u32) -> Result<bool, String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;

    if let Some(instance) = manager.instances.get_mut(&id) {
        // First check if shell itself has exited
        match instance.child.try_wait() {
            Ok(Some(_)) => return Ok(false), // Shell has exited, no foreground process
            Ok(None) => {}                   // Shell is running, continue to check children
            Err(e) => return Err(format!("Failed to check terminal status: {}", e)),
        }

        // Check if shell has any child processes (foreground process)
        if let Some(shell_pid) = instance.shell_pid {
            use sysinfo::System;

            let mut sys = System::new();
            sys.refresh_processes(sysinfo::ProcessesToUpdate::All);

            // Look for any process whose parent is the shell
            let has_child = sys.processes().values().any(|proc| {
                proc.parent()
                    .map(|parent_pid| parent_pid.as_u32() == shell_pid)
                    .unwrap_or(false)
            });

            Ok(has_child)
        } else {
            // No PID available, assume no foreground process
            Ok(false)
        }
    } else {
        // Terminal not found
        Ok(false)
    }
}
