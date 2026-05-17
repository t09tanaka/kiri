//! Tauri command wrappers for terminal functionality
//! These are thin wrappers that delegate to the core logic in terminal.rs

use super::cli_install;
use super::terminal::{
    create_pty_size, find_utf8_boundary, get_process_cwd, open_pty_with_shell, resolve_cwd,
    resolve_terminal_size, CliEnv, PtyCleanupGuard, PtyInstance, TerminalOutput,
    TerminalOutputBusState, TerminalState,
};
use lazy_static::lazy_static;
use serde::Serialize;
use std::io::{Read, Write};
use std::str;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

const PROCESS_SNAPSHOT_TTL: Duration = Duration::from_millis(1500);

/// Process info returned by get_terminal_process_info
#[derive(Debug, Clone, Serialize)]
pub struct TerminalProcessInfo {
    pub name: String,
    pub memory_bytes: u64,
}

#[derive(Clone)]
struct ProcessRecord {
    pid: u32,
    parent_pid: Option<u32>,
    name: String,
    memory_bytes: u64,
}

#[derive(Default)]
struct ProcessSnapshotCache {
    refreshed_at: Option<Instant>,
    records: Vec<ProcessRecord>,
}

lazy_static! {
    static ref PROCESS_SNAPSHOT_CACHE: Mutex<ProcessSnapshotCache> =
        Mutex::new(ProcessSnapshotCache::default());
}

fn process_snapshot_records() -> Vec<ProcessRecord> {
    let mut cache = PROCESS_SNAPSHOT_CACHE
        .lock()
        .expect("process snapshot cache mutex poisoned");
    if cache
        .refreshed_at
        .map(|t| t.elapsed() < PROCESS_SNAPSHOT_TTL)
        .unwrap_or(false)
    {
        return cache.records.clone();
    }

    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All);
    let records = sys
        .processes()
        .iter()
        .map(|(pid, proc)| ProcessRecord {
            pid: pid.as_u32(),
            parent_pid: proc.parent().map(|p| p.as_u32()),
            name: proc.name().to_string_lossy().to_string(),
            memory_bytes: proc.memory(),
        })
        .collect::<Vec<_>>();

    cache.refreshed_at = Some(Instant::now());
    cache.records = records.clone();
    records
}

pub fn process_info_for_shell_pid(shell_pid: u32) -> TerminalProcessInfo {
    let records = process_snapshot_records();
    let shell = records.iter().find(|p| p.pid == shell_pid);
    let children = records
        .iter()
        .filter(|p| p.parent_pid == Some(shell_pid))
        .collect::<Vec<_>>();

    let memory_bytes = shell.map(|p| p.memory_bytes).unwrap_or(0)
        + children.iter().map(|p| p.memory_bytes).sum::<u64>();
    let name = children
        .first()
        .map(|p| p.name.clone())
        .or_else(|| shell.map(|p| p.name.clone()))
        .unwrap_or_else(|| "Terminal".to_string());

    TerminalProcessInfo { name, memory_bytes }
}

pub fn shell_has_child_process(shell_pid: u32) -> bool {
    process_snapshot_records()
        .iter()
        .any(|p| p.parent_pid == Some(shell_pid))
}

/// Build the per-PTY CLI env (PATH + KIRI_SOCKET + KIRI_WINDOW_LABEL) for
/// the given window label. Returns `None` when the home directory can't
/// be located (no `~/.kiri/bin` to point PATH at).
fn cli_env_for(window_label: Option<&str>) -> Option<CliEnv> {
    let label = window_label?;
    let bin_dir = cli_install::kiri_bin_dir()?;
    let socket = cli_install::socket_path_for(label)?;
    Some(CliEnv {
        bin_dir,
        socket,
        window_label: label.to_string(),
    })
}

#[tauri::command]
pub fn create_terminal(
    app: AppHandle,
    state: tauri::State<'_, TerminalState>,
    bus: tauri::State<'_, TerminalOutputBusState>,
    cwd: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
    window_label: Option<String>,
) -> Result<u32, String> {
    let (initial_cols, initial_rows) = resolve_terminal_size(cols, rows);
    let resolved_cwd = resolve_cwd(cwd);
    let cli_env = cli_env_for(window_label.as_deref());

    // Wrap the freshly-spawned PTY in a cleanup guard so that any
    // early-return below (reader/writer extraction, state lock failure)
    // kills + reaps the shell instead of leaking the FD and process.
    // The guard is `commit()`-ed once the PtyInstance is safely owned
    // by the manager.
    let mut pty_guard = PtyCleanupGuard::new(open_pty_with_shell(
        initial_cols,
        initial_rows,
        resolved_cwd.as_deref(),
        cli_env.as_ref(),
    )?);

    let mut reader = pty_guard
        .as_mut()
        .pair
        .master
        .try_clone_reader()
        .map_err(|e| e.to_string())?;
    let writer = pty_guard
        .as_mut()
        .pair
        .master
        .take_writer()
        .map_err(|e| e.to_string())?;

    let mut manager = state.lock().map_err(|e| e.to_string())?;
    let id = manager.next_id;
    manager.next_id += 1;

    // Get shell PID for foreground process checking
    let shell_pid = pty_guard.as_mut().child.process_id();

    // Send bindkey -e to enable emacs mode for keyboard navigation
    // This ensures Option+Arrow and Cmd+Arrow work correctly regardless of user's shell config
    // Use clear to hide the command from the user
    let mut writer = writer;
    let _ = writer.write_all(b"bindkey -e && clear\n");
    let _ = writer.flush();

    // Take ownership out of the guard now that nothing else can fail.
    let pty_with_shell = pty_guard.commit();
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
    let bus_for_task: TerminalOutputBusState = bus.inner().clone();
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
                        let raw_chunk = &data_slice[..valid_len];
                        // Publish to in-process bus first so cli_server
                        // sentinel detection sees the same bytes the
                        // frontend receives.
                        bus_for_task.publish(terminal_id, raw_chunk);

                        // Safety: we just validated this is valid UTF-8
                        let data = unsafe { str::from_utf8_unchecked(raw_chunk) };
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
        bus_for_task.close(terminal_id);
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
pub fn close_terminal(
    state: tauri::State<'_, TerminalState>,
    bus: tauri::State<'_, TerminalOutputBusState>,
    id: u32,
) -> Result<(), String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;

    if let Some(mut instance) = manager.instances.remove(&id) {
        bus.close(id);
        drop(manager);
        thread::spawn(move || {
            let _ = instance.child.kill();
            let _ = instance.child.wait();
        });
        Ok(())
    } else {
        Err(format!("Terminal {} not found", id))
    }
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

    let shell_pid = {
        let mut manager = state.lock().map_err(|e| e.to_string())?;
        let Some(instance) = manager.instances.get_mut(&id) else {
            return Ok(default_info);
        };
        match instance.child.try_wait() {
            Ok(Some(_)) => return Ok(default_info),
            Ok(None) => {}
            Err(_) => return Ok(default_info),
        }
        instance.shell_pid
    };

    Ok(shell_pid
        .map(process_info_for_shell_pid)
        .unwrap_or(default_info))
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

/// Get the current working directory for a terminal's shell process
#[tauri::command]
pub fn get_terminal_cwd(
    state: tauri::State<'_, TerminalState>,
    id: u32,
) -> Result<Option<String>, String> {
    let mut manager = state.lock().map_err(|e| e.to_string())?;
    if let Some(instance) = manager.instances.get_mut(&id) {
        match instance.child.try_wait() {
            Ok(Some(_)) => return Ok(None),
            Ok(None) => {}
            Err(_) => return Ok(None),
        }
        if let Some(shell_pid) = instance.shell_pid {
            Ok(get_process_cwd(shell_pid))
        } else {
            Ok(None)
        }
    } else {
        Ok(None)
    }
}

/// Check if a terminal has a foreground process running (command in execution)
/// Returns true if there's a child process of the shell (command running),
/// false if the shell is idle (waiting at prompt)
#[tauri::command]
pub fn is_terminal_alive(state: tauri::State<'_, TerminalState>, id: u32) -> Result<bool, String> {
    let shell_pid = {
        let mut manager = state.lock().map_err(|e| e.to_string())?;
        let Some(instance) = manager.instances.get_mut(&id) else {
            return Ok(false);
        };
        match instance.child.try_wait() {
            Ok(Some(_)) => return Ok(false),
            Ok(None) => {}
            Err(e) => return Err(format!("Failed to check terminal status: {}", e)),
        }
        instance.shell_pid
    };

    Ok(shell_pid.map(shell_has_child_process).unwrap_or(false))
}
