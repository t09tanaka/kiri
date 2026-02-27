//! Tauri command wrappers for remote access server
//!
//! These commands allow starting, stopping, and querying the state
//! of the embedded HTTP server from the frontend.

use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

/// Holds the runtime state of the remote access server.
pub struct RemoteServerState {
    /// Sender half of the shutdown channel. Sending a value triggers graceful shutdown.
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// The port the server is listening on.
    port: u16,
    /// Whether the server is currently running.
    is_running: bool,
}

impl RemoteServerState {
    pub fn new() -> Self {
        Self {
            shutdown_tx: None,
            port: 9876,
            is_running: false,
        }
    }
}

impl Default for RemoteServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for the shared remote server state, managed by Tauri.
pub type RemoteServerStateType = Arc<Mutex<RemoteServerState>>;

/// Start the remote access HTTP server on the specified port.
///
/// Returns an error if the server is already running.
#[tauri::command]
pub async fn start_remote_server(
    state: tauri::State<'_, RemoteServerStateType>,
    port: u16,
) -> Result<(), String> {
    let mut server = state.lock().await;
    if server.is_running {
        return Err("Server is already running".to_string());
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        if let Err(e) = super::remote_access::start_server(port, shutdown_rx).await {
            log::error!("Remote server error: {}", e);
        }
    });

    server.shutdown_tx = Some(shutdown_tx);
    server.port = port;
    server.is_running = true;
    Ok(())
}

/// Stop the remote access HTTP server gracefully.
///
/// If the server is not running, this is a no-op.
#[tauri::command]
pub async fn stop_remote_server(
    state: tauri::State<'_, RemoteServerStateType>,
) -> Result<(), String> {
    let mut server = state.lock().await;
    if !server.is_running {
        return Ok(());
    }

    if let Some(tx) = server.shutdown_tx.take() {
        let _ = tx.send(());
    }
    server.is_running = false;
    Ok(())
}

/// Check whether the remote access server is currently running.
#[tauri::command]
pub async fn is_remote_server_running(
    state: tauri::State<'_, RemoteServerStateType>,
) -> Result<bool, String> {
    let server = state.lock().await;
    Ok(server.is_running)
}
