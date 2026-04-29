//! Per-window CLI server.
//!
//! Each kiri window gets its own Unix Domain Socket at
//! `~/.kiri/instances/<label>.sock`. The socket accepts newline-
//! delimited JSON requests, dispatches them to handlers, and replies
//! with newline-delimited JSON responses.

pub mod dispatch;
pub mod frontend_bridge;
pub mod handlers;
pub mod pane_map;
pub mod ring_buffer;
pub mod run_logic;

use crate::commands::cli_install;
use crate::commands::terminal::{TerminalOutputBusState, TerminalState};
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::{GenericFilePath, ListenerOptions, ToFsName};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

pub struct CliServerHandle {
    pub socket_path: PathBuf,
    pub label: String,
    #[allow(dead_code)]
    join: JoinHandle<()>,
    stop: Mutex<Option<oneshot::Sender<()>>>,
    pub pending: Arc<frontend_bridge::PendingReplies>,
    pub pane_map: Arc<pane_map::PaneMap>,
}

impl CliServerHandle {
    pub fn stop(&self) {
        if let Some(tx) = self.stop.lock().expect("stop mutex poisoned").take() {
            let _ = tx.send(());
        }
    }
}

#[derive(Default)]
pub struct CliServerRegistry {
    pub handles: Mutex<HashMap<String, Arc<CliServerHandle>>>,
}

impl CliServerRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a handle keyed by window label. If a previous handle
    /// existed for that label, stop it first.
    pub fn insert(&self, label: String, handle: Arc<CliServerHandle>) {
        let mut map = self.handles.lock().expect("registry mutex poisoned");
        if let Some(prev) = map.insert(label, handle) {
            prev.stop();
        }
    }

    pub fn stop_and_remove(&self, label: &str) {
        let mut map = self.handles.lock().expect("registry mutex poisoned");
        if let Some(h) = map.remove(label) {
            h.stop();
        }
    }
}

pub type CliServerRegistryState = Arc<CliServerRegistry>;

/// Spawn a listener for one window. The caller is expected to store the
/// returned handle in the registry so it can be stopped on window close.
pub fn spawn_for_window(
    label: String,
    app: tauri::AppHandle,
    terminals: TerminalState,
    bus: TerminalOutputBusState,
) -> std::io::Result<CliServerHandle> {
    let socket_path = cli_install::socket_path_for(&label)
        .ok_or_else(|| std::io::Error::other("no home dir for socket path"))?;
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
        // Tighten the parent dir to 0700 so that even if the socket file
        // is briefly created with wider permissions (between bind and the
        // explicit chmod below), no other local user can list or open it.
        // This is the primary defence; the chmod is belt-and-suspenders.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
    }
    // Best-effort: ensure no stale socket file blocks bind.
    let _ = std::fs::remove_file(&socket_path);

    let name = socket_path.as_os_str().to_fs_name::<GenericFilePath>()?;
    let listener = ListenerOptions::new().name(name).create_tokio()?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))?;
    }

    let pane_map = Arc::new(pane_map::PaneMap::new());
    let pending = Arc::new(frontend_bridge::PendingReplies::new());
    let buffers = Arc::new(dispatch::TerminalBuffers::new());

    let ctx = dispatch::DispatchContext {
        label: label.clone(),
        app: Some(app),
        terminals,
        bus,
        pane_map: pane_map.clone(),
        pending: pending.clone(),
        buffers,
    };

    let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
    let socket_for_cleanup = socket_path.clone();
    let label_for_task = label.clone();
    let ctx_for_task = ctx.clone();

    let join = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                conn = listener.accept() => {
                    match conn {
                        Ok(stream) => {
                            let ctx = ctx_for_task.clone();
                            let label_for_conn = label_for_task.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, ctx).await {
                                    log::warn!(
                                        "cli_server[{label_for_conn}] connection error: {e}"
                                    );
                                }
                            });
                        }
                        Err(e) => {
                            log::warn!("cli_server[{label_for_task}] accept failed: {e}");
                        }
                    }
                }
            }
        }
        let _ = std::fs::remove_file(&socket_for_cleanup);
    });

    Ok(CliServerHandle {
        socket_path,
        label,
        join,
        stop: Mutex::new(Some(stop_tx)),
        pending,
        pane_map,
    })
}

async fn handle_connection(
    stream: interprocess::local_socket::tokio::Stream,
    ctx: dispatch::DispatchContext,
) -> std::io::Result<()> {
    use interprocess::local_socket::traits::tokio::Stream as _;
    let (reader, mut writer) = stream.split();
    let mut lines = BufReader::new(reader).lines();
    while let Some(line) = lines.next_line().await? {
        if line.is_empty() {
            continue;
        }
        let responses = dispatch::dispatch_line(&ctx, &line).await;
        for resp in responses {
            let mut bytes = serde_json::to_vec(&resp).unwrap_or_else(|_| {
                serde_json::to_vec(&kiri_cli_proto::Response::Error {
                    code: kiri_cli_proto::ErrorCode::InternalError,
                    message: "serialization failed".into(),
                    detail: None,
                })
                .expect("error variant always serializes")
            });
            bytes.push(b'\n');
            writer.write_all(&bytes).await?;
        }
    }
    Ok(())
}

#[tauri::command]
pub fn cli_resolve_pending(
    registry: tauri::State<'_, CliServerRegistryState>,
    label: String,
    request_id: String,
    payload: serde_json::Value,
) -> Result<bool, String> {
    let map = registry.handles.lock().map_err(|e| e.to_string())?;
    let handle = map
        .get(&label)
        .ok_or_else(|| format!("no server for {label}"))?;
    Ok(handle.pending.resolve(&request_id, payload))
}

#[tauri::command]
pub fn cli_update_pane_map(
    registry: tauri::State<'_, CliServerRegistryState>,
    label: String,
    panes: Vec<pane_map::PaneEntry>,
) -> Result<(), String> {
    let map = registry.handles.lock().map_err(|e| e.to_string())?;
    let handle = map
        .get(&label)
        .ok_or_else(|| format!("no server for {label}"))?;
    handle.pane_map.replace(panes);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_constructs_empty() {
        let r = CliServerRegistry::new();
        assert!(r.handles.lock().unwrap().is_empty());
    }

    #[test]
    fn registry_stop_and_remove_unknown_is_noop() {
        let r = CliServerRegistry::new();
        r.stop_and_remove("missing");
        assert!(r.handles.lock().unwrap().is_empty());
    }
}
