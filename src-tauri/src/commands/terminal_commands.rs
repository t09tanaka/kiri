//! Tauri command wrappers for terminal functionality
//! These are thin wrappers that delegate to the core logic in terminal.rs

use super::terminal::{
    create_pty_size, open_pty_with_shell, resolve_cwd, resolve_terminal_size, PtyInstance,
    TerminalOutput, TerminalState,
};
use std::io::Read;
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

    manager.instances.insert(
        id,
        PtyInstance {
            master: pty_with_shell.pair.master,
            writer,
            _child: pty_with_shell.child,
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
