use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct TerminalOutput {
    pub id: u32,
    pub data: String,
}

struct PtyInstance {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    _child: Box<dyn portable_pty::Child + Send + Sync>,
}

pub struct TerminalManager {
    instances: HashMap<u32, PtyInstance>,
    next_id: u32,
}

impl TerminalManager {
    pub fn new() -> Self {
        Self {
            instances: HashMap::new(),
            next_id: 1,
        }
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

pub type TerminalState = Arc<Mutex<TerminalManager>>;

#[tauri::command]
pub fn create_terminal(
    app: AppHandle,
    state: tauri::State<'_, TerminalState>,
    cwd: Option<String>,
    cols: Option<u16>,
    rows: Option<u16>,
) -> Result<u32, String> {
    let pty_system = native_pty_system();

    // Use provided size or fallback to reasonable defaults
    // Getting the correct initial size is critical for Ink-based apps like Claude Code
    let initial_cols = cols.unwrap_or(120);
    let initial_rows = rows.unwrap_or(30);

    let pair = pty_system
        .openpty(PtySize {
            rows: initial_rows,
            cols: initial_cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| e.to_string())?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.arg("-l"); // Login shell

    // Set TERM to enable proper ANSI escape sequence handling
    // This is critical for CLI tools like Claude Code that use cursor movement
    // and line clearing for progress indicators
    cmd.env("TERM", "xterm-256color");

    if let Some(dir) = cwd {
        cmd.cwd(dir);
    } else if let Some(home) = dirs::home_dir() {
        cmd.cwd(home);
    }

    let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;

    let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
    let writer = pair.master.take_writer().map_err(|e| e.to_string())?;

    let mut manager = state.lock().map_err(|e| e.to_string())?;
    let id = manager.next_id;
    manager.next_id += 1;

    manager.instances.insert(
        id,
        PtyInstance {
            master: pair.master,
            writer,
            _child: child,
        },
    );

    // Spawn thread to read PTY output
    let terminal_id = id;
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app.emit(
                        "terminal-output",
                        TerminalOutput {
                            id: terminal_id,
                            data,
                        },
                    );
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
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
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
