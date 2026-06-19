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
pub mod signals;

use crate::commands::cli_install;
use crate::commands::lock_ext::LockExt;
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
    pub buffers: Arc<dispatch::TerminalBuffers>,
    pub signals: Arc<signals::SignalRegistry>,
}

impl CliServerHandle {
    pub fn stop(&self) {
        if let Some(tx) = self.stop.lock_recover().take() {
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
        let mut map = self.handles.lock_recover();
        if let Some(prev) = map.insert(label, handle) {
            prev.stop();
        }
    }

    pub fn stop_and_remove(&self, label: &str) {
        let mut map = self.handles.lock_recover();
        if let Some(h) = map.remove(label) {
            h.stop();
            // Remove the socket file synchronously rather than relying on the
            // listener task's own cleanup. On window close the task removes it
            // anyway, but on abrupt app exit the Tokio runtime can be torn
            // down before that task runs — deleting here guarantees no stale
            // socket is left for the CLI to mistake for a live window.
            let _ = std::fs::remove_file(&h.socket_path);
        }
    }

    /// Stop every registered server and remove its socket file. Called on
    /// application exit so that quitting does not leave behind sockets that
    /// the `kiri` CLI would later report as ghost windows.
    pub fn stop_all(&self) {
        let mut map = self.handles.lock_recover();
        for (_label, h) in map.drain() {
            h.stop();
            let _ = std::fs::remove_file(&h.socket_path);
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
    let signals = Arc::new(signals::SignalRegistry::new());

    let ctx = dispatch::DispatchContext {
        label: label.clone(),
        app: Some(app),
        terminals,
        bus,
        pane_map: pane_map.clone(),
        pending: pending.clone(),
        buffers: buffers.clone(),
        signals: signals.clone(),
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
        buffers,
        signals,
    })
}

/// Probe whether a socket has a live listener by attempting to connect.
/// A leftover socket file from a crashed/force-quit session refuses the
/// connection; a socket served by a running kiri instance accepts it.
async fn socket_alive(path: &std::path::Path) -> bool {
    let Ok(name) = path.as_os_str().to_fs_name::<GenericFilePath>() else {
        return false;
    };
    interprocess::local_socket::tokio::Stream::connect(name)
        .await
        .is_ok()
}

/// On startup, delete socket files in `~/.kiri/instances` whose listener is
/// gone — leftovers from a previous session that crashed or was force-quit
/// before its own cleanup (the `RunEvent::Exit` handler) could run.
///
/// Only **dead** sockets are removed: a socket still served by another
/// running kiri instance answers the probe and is left untouched, so this
/// is safe to call unconditionally at launch. Best-effort — every error is
/// ignored so a sweep failure can never block startup.
pub async fn sweep_dead_sockets() {
    let Some(dir) = cli_install::socket_dir() else {
        return;
    };
    sweep_dead_sockets_in(&dir).await;
}

/// Directory-scoped core of [`sweep_dead_sockets`], split out so it can be
/// tested against a temp dir instead of the real `~/.kiri/instances`.
async fn sweep_dead_sockets_in(dir: &std::path::Path) {
    let entries = match std::fs::read_dir(dir) {
        Ok(it) => it,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("sock") {
            continue;
        }
        if !socket_alive(&path).await {
            if let Err(e) = std::fs::remove_file(&path) {
                log::warn!("failed to remove stale socket {}: {e}", path.display());
            } else {
                log::info!("removed stale socket {}", path.display());
            }
        }
    }
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
    // Snapshot the set of currently-live pane ids before replacing the
    // pane map, then have the signal registry prune anything no longer
    // present. This is the only path that learns about pane removals
    // (the cli_server's `close` handler routes through the frontend,
    // which then calls back into this command with the new layout).
    let known: std::collections::HashSet<String> =
        panes.iter().map(|p| p.pane_id.clone()).collect();
    let known_terminal_ids: std::collections::HashSet<u32> =
        panes.iter().map(|p| p.terminal_id).collect();
    handle.pane_map.replace(panes);
    handle.buffers.retain_terminal_ids(&known_terminal_ids);
    handle.signals.retain(&known);
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

    /// Build a handle backed by a placeholder file at `socket_path`. The
    /// listener task is a no-op that just waits for the stop signal, which is
    /// enough to exercise registry teardown without a Tauri runtime.
    fn fake_handle(label: &str, socket_path: PathBuf) -> CliServerHandle {
        let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
        let join = tokio::spawn(async move {
            let _ = (&mut stop_rx).await;
        });
        CliServerHandle {
            socket_path,
            label: label.to_string(),
            join,
            stop: Mutex::new(Some(stop_tx)),
            pending: Arc::new(frontend_bridge::PendingReplies::new()),
            pane_map: Arc::new(pane_map::PaneMap::new()),
            buffers: Arc::new(dispatch::TerminalBuffers::new()),
            signals: Arc::new(signals::SignalRegistry::new()),
        }
    }

    #[tokio::test]
    async fn stop_and_remove_deletes_socket_file() {
        let dir = std::env::temp_dir().join(format!(
            "kiri-cli-test-stop-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let socket_path = dir.join("window-test.sock");
        std::fs::File::create(&socket_path).unwrap();
        assert!(socket_path.exists());

        let registry = CliServerRegistry::new();
        registry.insert(
            "window-test".to_string(),
            Arc::new(fake_handle("window-test", socket_path.clone())),
        );

        registry.stop_and_remove("window-test");

        assert!(
            !socket_path.exists(),
            "stop_and_remove must delete the socket file"
        );
        assert!(registry.handles.lock().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn stop_all_clears_registry_and_removes_all_sockets() {
        let dir = std::env::temp_dir().join(format!(
            "kiri-cli-test-stopall-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let sock_a = dir.join("window-a.sock");
        let sock_b = dir.join("window-b.sock");
        std::fs::File::create(&sock_a).unwrap();
        std::fs::File::create(&sock_b).unwrap();

        let registry = CliServerRegistry::new();
        registry.insert(
            "window-a".to_string(),
            Arc::new(fake_handle("window-a", sock_a.clone())),
        );
        registry.insert(
            "window-b".to_string(),
            Arc::new(fake_handle("window-b", sock_b.clone())),
        );

        registry.stop_all();

        assert!(!sock_a.exists(), "stop_all must delete every socket file");
        assert!(!sock_b.exists(), "stop_all must delete every socket file");
        assert!(registry.handles.lock().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn sweep_removes_dead_but_keeps_live_sockets() {
        let dir = std::env::temp_dir().join(format!(
            "kiri-cli-test-sweep-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        // A leftover socket file with no listener — must be removed.
        let dead = dir.join("window-dead.sock");
        std::fs::File::create(&dead).unwrap();

        // A non-.sock file — must be ignored entirely.
        let other = dir.join("notes.txt");
        std::fs::File::create(&other).unwrap();

        // A socket with a real listener bound — must be preserved.
        let live = dir.join("window-live.sock");
        let name = live
            .as_os_str()
            .to_fs_name::<GenericFilePath>()
            .expect("fs name");
        let _listener = ListenerOptions::new()
            .name(name)
            .create_tokio()
            .expect("bind live listener");

        sweep_dead_sockets_in(&dir).await;

        assert!(!dead.exists(), "dead socket must be removed");
        assert!(live.exists(), "live socket must be preserved");
        assert!(other.exists(), "non-.sock files must be ignored");

        drop(_listener);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
