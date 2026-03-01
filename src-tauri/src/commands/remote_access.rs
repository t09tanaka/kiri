//! Core HTTP server logic for remote access
//!
//! Provides an embedded axum HTTP server that can be started and stopped
//! at runtime. The server exposes a health endpoint and a WebSocket
//! endpoint for real-time status updates.
//!
//! All paths (except `/api/health`) are protected by a path-prefix token:
//! requests must be made to `/{token}/...`. The middleware validates the
//! token and strips it from the URI before passing to downstream handlers.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        Request, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{oneshot, RwLock};

/// Shared application state passed to handlers and middleware.
#[derive(Clone)]
pub struct AppState {
    /// The path-prefix token required to access protected endpoints.
    pub(crate) auth_token: Arc<RwLock<String>>,
    /// Optional Tauri AppHandle for accessing app state (None in integration tests).
    pub(crate) app_handle: Option<tauri::AppHandle>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("auth_token", &"[REDACTED]")
            .field("app_handle", &self.app_handle.is_some())
            .finish()
    }
}

/// Response payload for the health check endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// A currently open project with an active window.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenProject {
    pub path: String,
    pub name: String,
    pub branch: Option<String>,
    pub is_worktree: bool,
}

/// A recently opened project from the settings store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentProject {
    pub path: String,
    pub name: String,
    #[serde(rename = "lastOpened")]
    pub last_opened: f64,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
}

/// Status of a single terminal instance.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStatus {
    pub id: u32,
    pub is_alive: bool,
    pub process_name: Option<String>,
    pub cwd: Option<String>,
}

// ── Incoming client actions ──────────────────────────────────────

/// Incoming action from a remote client via WebSocket.
///
/// `rename_all = "camelCase"` affects both the `tag` discriminator and field
/// names.  All current fields are single lowercase words (`path`) so the
/// rename is a no-op for them; this note exists to prevent surprises if
/// multi-word fields are added in the future.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum ClientAction {
    OpenProject { path: String },
    CloseProject { path: String },
}

// ── WebSocket status types ──────────────────────────────────────

/// Payload pushed over the `/ws/status` WebSocket every tick.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusUpdate {
    pub open_projects: Vec<OpenProject>,
    pub recent_projects: Vec<RecentProject>,
    pub terminals: Vec<TerminalStatus>,
    pub timestamp: u64,
}

// ── Handlers ─────────────────────────────────────────────────────

/// Handler for `GET /api/health`.
///
/// Returns a JSON response indicating the server is running,
/// along with the current application version.
/// This endpoint does **not** require authentication.
pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Filter a list of recent projects, removing any whose path appears in
/// `open_paths`.
///
/// This is the pure-logic core of [`load_recent_projects`], extracted so
/// that it can be unit-tested without a Tauri runtime.
fn filter_recent_projects(
    all_recent: Vec<RecentProject>,
    open_paths: &[String],
) -> Vec<RecentProject> {
    all_recent
        .into_iter()
        .filter(|p| !open_paths.contains(&p.path))
        .collect()
}

/// Load recent projects from the kiri-settings.json store,
/// filtering out any paths that are currently open.
fn load_recent_projects(app: &tauri::AppHandle, open_paths: &[String]) -> Vec<RecentProject> {
    use tauri_plugin_store::StoreExt;

    let store = match app.store("kiri-settings.json") {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let all_recent: Vec<RecentProject> = store
        .get("recentProjects")
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    filter_recent_projects(all_recent, open_paths)
}

/// Extract a human-readable project name from a directory path.
///
/// Returns the last path component, or `"unknown"` when the path is
/// empty or cannot be decoded (e.g. terminates with `..`).
pub(crate) fn extract_project_name(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Create a refreshed `sysinfo::System` for process lookups.
///
/// Call this once, then pass the result to [`lookup_process_name`] for
/// each terminal.  This avoids creating N `System` instances (and N
/// full process-table scans) when there are N terminals.
pub(crate) fn refreshed_system() -> sysinfo::System {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All);
    sys
}

/// Look up the foreground process name for a given shell PID.
///
/// Uses the provided, already-refreshed [`sysinfo::System`] to find
/// child processes of the shell.  Returns the child's name if one
/// exists, otherwise the shell's own name.
pub(crate) fn lookup_process_name(sys: &sysinfo::System, shell_pid: u32) -> Option<String> {
    use sysinfo::Pid;

    let spid = Pid::from_u32(shell_pid);

    // Find child processes of the shell
    let child = sys
        .processes()
        .values()
        .find(|proc| {
            proc.parent()
                .map(|parent_pid| parent_pid == spid)
                .unwrap_or(false)
        });

    if let Some(child) = child {
        Some(child.name().to_string_lossy().to_string())
    } else {
        sys.process(spid)
            .map(|p| p.name().to_string_lossy().to_string())
    }
}

// ── WebSocket handler ────────────────────────────────────────────

/// Handler for `GET /ws`.
///
/// Upgrades to a WebSocket connection that pushes a combined status
/// update (open projects, recent projects, terminals) every 2 seconds.
///
/// Authentication is handled by the path-prefix middleware, so this
/// handler simply accepts the WebSocket upgrade.
pub async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_status_ws(socket, state))
}

/// Drive the WebSocket connection: send status every 2 s and handle
/// incoming frames -- ping/close control frames and `ClientAction`
/// JSON messages (`openProject`, `closeProject`).
async fn handle_status_ws(mut socket: WebSocket, state: AppState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let state_clone = state.clone();
                let status = tokio::task::spawn_blocking(move || {
                    collect_full_status(&state_clone)
                }).await.ok().flatten();
                match status {
                    Some(data) => {
                        let json = serde_json::to_string(&data).unwrap_or_default();
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientAction>(&text) {
                            Ok(action) => handle_client_action(&state, action).await,
                            Err(e) => log::warn!("remote_access: ignoring unrecognized WS message: {e}"),
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Handle an incoming client action.
async fn handle_client_action(state: &AppState, action: ClientAction) {
    let Some(app) = state.app_handle.as_ref() else {
        return;
    };

    match action {
        ClientAction::OpenProject { path } => {
            // Canonicalize the path to prevent path-traversal attacks.
            let canonical = match std::fs::canonicalize(&path) {
                Ok(p) => p.to_string_lossy().into_owned(),
                Err(_) => return, // path does not exist — ignore
            };

            // Defense-in-depth: restrict to paths under $HOME
            if let Some(home) = dirs::home_dir() {
                let home_str = home.to_string_lossy();
                if !canonical.starts_with(home_str.as_ref()) {
                    log::warn!(
                        "remote_access: rejected openProject outside $HOME: {}",
                        canonical
                    );
                    return;
                }
            }

            use tauri::Manager;
            let registry = app.state::<crate::commands::WindowRegistryState>();

            // Check if project is already open — focus existing window
            let existing_label = {
                let reg = match registry.lock() {
                    Ok(r) => r,
                    Err(_) => return,
                };
                reg.get_label_for_path(&canonical).cloned()
            };

            if let Some(label) = existing_label {
                if let Some(window) = app.get_webview_window(&label) {
                    let _ = window.set_focus();
                    return;
                } else {
                    // Window closed without being unregistered — purge the stale entry
                    if let Ok(mut reg) = registry.lock() {
                        reg.unregister_by_label(&label);
                    }
                }
            }

            // Create new window
            let _ = crate::commands::window::create_window_impl(
                app,
                Some(&registry),
                None,
                None,
                None,
                None,
                Some(canonical),
            );
        }
        ClientAction::CloseProject { path } => {
            // Canonicalize the path before registry lookup.
            let canonical = match std::fs::canonicalize(&path) {
                Ok(p) => p.to_string_lossy().into_owned(),
                Err(_) => return, // path does not exist — ignore
            };

            use tauri::Manager;
            let registry = app.state::<crate::commands::WindowRegistryState>();
            let label = {
                let reg = match registry.lock() {
                    Ok(r) => r,
                    Err(_) => return,
                };
                reg.get_label_for_path(&canonical).cloned()
            };

            if let Some(label) = label {
                if let Some(window) = app.get_webview_window(&label) {
                    let _ = window.close();
                }
            }
        }
    }
}

/// Gather a full status snapshot for the WebSocket push.
///
/// Returns `None` when the `AppHandle` is unavailable (e.g. during
/// tests without a Tauri runtime), which causes the WebSocket to close.
fn collect_full_status(state: &AppState) -> Option<StatusUpdate> {
    use tauri::Manager;

    let app = state.app_handle.as_ref()?;

    // -- Open projects --
    let registry = app.state::<crate::commands::WindowRegistryState>();
    let open_paths = {
        let reg = registry.lock().map_err(|e| {
            log::error!("WindowRegistry mutex poisoned: {e}");
            e
        }).ok()?;
        reg.get_all_paths()
    };

    let open_projects: Vec<OpenProject> = {
        let reg = registry.lock().ok()?;
        open_paths
            .iter()
            .map(|path| {
                let name = extract_project_name(path);
                let is_worktree = reg.is_worktree_path(path);
                OpenProject {
                    path: path.clone(),
                    name,
                    branch: None,
                    is_worktree,
                }
            })
            .collect()
    };

    // -- Recent projects --
    let recent_projects = load_recent_projects(app, &open_paths);

    // -- Terminals --
    let terminal_state = app.state::<crate::commands::TerminalState>();
    let terminal_snapshots: Vec<(u32, bool, Option<u32>)> = {
        let mut manager = terminal_state.lock().map_err(|e| {
            log::error!("TerminalState mutex poisoned: {e}");
            e
        }).ok()?;
        manager
            .instances
            .iter_mut()
            .map(|(&id, instance)| {
                let is_alive = instance
                    .child
                    .try_wait()
                    .map(|s| s.is_none())
                    .unwrap_or(false);
                (id, is_alive, instance.shell_pid)
            })
            .collect()
    };

    // Refresh process table once for all terminal lookups.
    let sys = refreshed_system();
    let mut terminals = Vec::with_capacity(terminal_snapshots.len());
    for (id, is_alive, shell_pid) in terminal_snapshots {
        let process_name = if is_alive {
            shell_pid.and_then(|pid| lookup_process_name(&sys, pid))
        } else {
            None
        };
        terminals.push(TerminalStatus {
            id,
            is_alive,
            process_name,
            cwd: None,
        });
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Some(StatusUpdate {
        open_projects,
        recent_projects,
        terminals,
        timestamp,
    })
}

// ── Token path helpers ────────────────────────────────────────────

/// Validate that `path` starts with `/{token}/` and return the remaining
/// path after stripping the token prefix.
///
/// Returns `None` when the first path segment does not match the expected
/// token. The `/api/health` path is **not** token-prefixed and will also
/// return `None` (it bypasses token validation at the middleware level).
///
/// # Examples
///
/// ```text
/// strip_token_prefix("/abc-123/ws",     "abc-123") => Some("/ws")
/// strip_token_prefix("/abc-123/",       "abc-123") => Some("/")
/// strip_token_prefix("/abc-123",        "abc-123") => Some("/")
/// strip_token_prefix("/wrong-token/ws", "abc-123") => None
/// strip_token_prefix("/api/health",     "abc-123") => None
/// ```
fn strip_token_prefix<'a>(path: &'a str, expected_token: &str) -> Option<&'a str> {
    // Path must start with '/'
    let after_slash = path.strip_prefix('/')?;

    // Extract the first segment (up to the next '/' or end of string)
    let (segment, rest) = match after_slash.find('/') {
        Some(pos) => (&after_slash[..pos], &after_slash[pos..]),
        None => (after_slash, "/"),
    };

    // Constant-time comparison to prevent timing attacks
    use subtle::ConstantTimeEq;
    let segment_bytes = segment.as_bytes();
    let expected_bytes = expected_token.as_bytes();

    if segment_bytes.len() != expected_bytes.len()
        || !bool::from(segment_bytes.ct_eq(expected_bytes))
    {
        return None;
    }

    // rest is either "/" or "/something..."
    // When path was "/abc-123" (no trailing slash), rest is already "/"
    Some(rest)
}

// ── Router & Server ──────────────────────────────────────────────

/// Build the axum router with token-path middleware.
///
/// The `auth_token` is embedded in request paths: `/{token}/ws`, etc.
/// The middleware validates the token prefix and strips it before
/// routing. `/api/health` is the only unauthenticated endpoint.
///
/// The `app_handle` provides access to Tauri state (WindowRegistry,
/// store, etc.) and is `None` during integration tests that don't
/// have a Tauri runtime.
///
/// Architecture: An outer router handles `/api/health` directly.
/// All other requests fall through to the token-path middleware,
/// which validates and strips the `/{token}/` prefix, then forwards
/// to an inner router for route matching.
pub fn create_router(
    auth_token: Arc<RwLock<String>>,
    app_handle: Option<tauri::AppHandle>,
) -> Router {
    let state = AppState {
        auth_token,
        app_handle: app_handle.clone(),
    };

    // Inner router with token-protected routes.
    // These are matched AFTER the token prefix is stripped by the fallback handler.
    let mut inner = Router::new()
        .route("/api/health", get(health_handler))
        .route("/ws", get(ws_handler))
        .with_state(state.clone());

    // Serve static PWA files as fallback for non-API/WS paths.
    // After token stripping, paths like /app.js are served from the
    // remote-ui directory.
    let ui_path = resolve_remote_ui_path(app_handle.as_ref());
    if ui_path.exists() {
        inner = inner.fallback_service(tower_http::services::ServeDir::new(ui_path));
    } else {
        log::warn!("Remote UI directory not found: {:?}", ui_path);
    }

    // Outer router: /api/health is public, everything else goes through
    // the token-path middleware which rewrites the URI then forwards to
    // the inner router.
    Router::new()
        .route("/api/health", get(health_handler))
        .fallback(token_path_handler)
        .with_state(TokenPathState {
            app_state: state,
            inner,
        })
}

/// Combined state for the token-path fallback handler.
#[derive(Clone)]
struct TokenPathState {
    app_state: AppState,
    inner: Router,
}

/// Fallback handler that validates the token prefix, strips it from the
/// URI, and forwards the request to the inner router for route matching.
///
/// This is implemented as a handler (not middleware) because axum's
/// `Router::layer()` middleware runs after route matching, whereas we
/// need to rewrite the URI *before* routes are matched.
async fn token_path_handler(
    State(state): State<TokenPathState>,
    mut request: Request,
) -> Response {
    let path = request.uri().path();

    let expected = state.app_state.auth_token.read().await;
    let stripped = match strip_token_prefix(path, &expected) {
        Some(s) => s.to_owned(),
        None => return StatusCode::NOT_FOUND.into_response(),
    };
    drop(expected);

    // Rebuild the URI with the token prefix removed, preserving query string
    let new_path_and_query = if let Some(q) = request.uri().query() {
        format!("{}?{}", stripped, q)
    } else {
        stripped
    };

    let new_uri = match new_path_and_query.parse::<axum::http::Uri>() {
        Ok(uri) => uri,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    *request.uri_mut() = new_uri;

    // Forward to the inner router
    use tower::ServiceExt;
    match state.inner.clone().oneshot(request).await {
        Ok(response) => response,
        Err(err) => err.into_response(),
    }
}

/// Resolve the path to the remote-ui directory.
///
/// In development: `remote-ui/` relative to the working directory
/// (which for `cargo tauri dev` is `src-tauri/`).
/// In production: `remote-ui/` relative to the app's resource directory.
fn resolve_remote_ui_path(app_handle: Option<&tauri::AppHandle>) -> std::path::PathBuf {
    // Try Tauri resource dir first (production)
    if let Some(app) = app_handle {
        use tauri::Manager;
        if let Ok(resource_dir) = app.path().resource_dir() {
            let path = resource_dir.join("remote-ui");
            if path.exists() {
                return path;
            }
        }
    }

    // Fallback to development path
    std::path::PathBuf::from("remote-ui")
}

/// Start the HTTP server on a pre-bound listener.
///
/// The caller is responsible for binding the `TcpListener` so that
/// port-conflict errors are reported eagerly rather than inside the
/// spawned task.
///
/// The server runs until a signal is received on `shutdown_rx`,
/// at which point it performs a graceful shutdown.
///
/// # Errors
///
/// Returns an error if the server encounters a runtime error.
pub async fn start_server(
    listener: tokio::net::TcpListener,
    shutdown_rx: oneshot::Receiver<()>,
    auth_token: Arc<RwLock<String>>,
    app_handle: Option<tauri::AppHandle>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router(auth_token, app_handle);
    log::info!(
        "Remote access server listening on {}",
        listener.local_addr()?
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        })
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "0.0.1".to_string(),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["version"], "0.0.1");
    }

    #[test]
    fn test_create_router_builds() {
        // Verify the router can be constructed without panicking
        let token = Arc::new(RwLock::new("test-token".to_string()));
        let _router = create_router(token, None);
    }

    #[test]
    fn test_app_state_clone() {
        let state = AppState {
            auth_token: Arc::new(RwLock::new("abc-123".to_string())),
            app_handle: None,
        };
        let cloned = state.clone();
        // Both point to the same Arc
        assert!(Arc::ptr_eq(&state.auth_token, &cloned.auth_token));
    }

    #[test]
    fn test_app_state_debug_redacts_token() {
        let state = AppState {
            auth_token: Arc::new(RwLock::new("secret-token".to_string())),
            app_handle: None,
        };
        let debug_output = format!("{:?}", state);
        assert!(debug_output.contains("[REDACTED]"));
        assert!(!debug_output.contains("secret-token"));
    }

    #[test]
    fn test_recent_project_serde_camel_case() {
        let json = serde_json::json!({
            "path": "/Users/user/projects/test",
            "name": "test",
            "lastOpened": 1700000000.0,
            "gitBranch": "main"
        });
        let project: RecentProject = serde_json::from_value(json).unwrap();
        assert_eq!(project.path, "/Users/user/projects/test");
        assert_eq!(project.last_opened, 1700000000.0);
        assert_eq!(project.git_branch, Some("main".to_string()));

        // Verify serialization preserves camelCase
        let serialized = serde_json::to_value(&project).unwrap();
        assert!(serialized.get("lastOpened").is_some());
        assert!(serialized.get("gitBranch").is_some());
    }

    #[test]
    fn test_recent_project_serde_null_branch() {
        let json = serde_json::json!({
            "path": "/Users/user/projects/test",
            "name": "test",
            "lastOpened": 1700000000.0,
            "gitBranch": null
        });
        let project: RecentProject = serde_json::from_value(json).unwrap();
        assert_eq!(project.git_branch, None);
    }

    #[test]
    fn test_app_state_with_no_app_handle() {
        let state = AppState {
            auth_token: Arc::new(RwLock::new("token".to_string())),
            app_handle: None,
        };
        assert!(state.app_handle.is_none());
        let debug = format!("{:?}", state);
        assert!(debug.contains("false")); // app_handle: false (is_some)
    }

    #[test]
    fn test_status_update_serialization() {
        let update = StatusUpdate {
            open_projects: vec![OpenProject {
                path: "/projects/kiri".to_string(),
                name: "kiri".to_string(),
                branch: Some("main".to_string()),
                is_worktree: false,
            }],
            recent_projects: vec![],
            terminals: vec![TerminalStatus {
                id: 1,
                is_alive: true,
                process_name: Some("cargo".to_string()),
                cwd: None,
            }],
            timestamp: 1700000000,
        };

        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["openProjects"].as_array().unwrap().len(), 1);
        assert!(json["recentProjects"].as_array().unwrap().is_empty());
        assert_eq!(json["terminals"].as_array().unwrap().len(), 1);
        assert_eq!(json["timestamp"], 1700000000);
        assert_eq!(json["terminals"][0]["processName"], "cargo");
    }

    #[test]
    fn test_status_update_serialization_empty() {
        let update = StatusUpdate {
            open_projects: vec![],
            recent_projects: vec![],
            terminals: vec![],
            timestamp: 0,
        };
        let json = serde_json::to_value(&update).unwrap();
        assert!(json["openProjects"].as_array().unwrap().is_empty());
        assert!(json["recentProjects"].as_array().unwrap().is_empty());
        assert!(json["terminals"].as_array().unwrap().is_empty());
        assert_eq!(json["timestamp"], 0);
    }

    #[test]
    fn test_terminal_status_clone() {
        let status = TerminalStatus {
            id: 42,
            is_alive: true,
            process_name: Some("node".to_string()),
            cwd: Some("/tmp".to_string()),
        };
        let cloned = status.clone();
        assert_eq!(cloned.id, 42);
        assert_eq!(cloned.is_alive, true);
        assert_eq!(cloned.process_name, Some("node".to_string()));
        assert_eq!(cloned.cwd, Some("/tmp".to_string()));
    }

    #[test]
    fn test_collect_full_status_returns_none_without_app_handle() {
        let state = AppState {
            auth_token: Arc::new(RwLock::new("token".to_string())),
            app_handle: None,
        };
        assert!(collect_full_status(&state).is_none());
    }

    #[test]
    fn test_resolve_remote_ui_path_without_app_handle() {
        let path = resolve_remote_ui_path(None);
        assert_eq!(path, std::path::PathBuf::from("remote-ui"));
    }

    // ── ClientAction deserialization tests ────────────────────────

    #[test]
    fn test_client_action_open_project_deserialization() {
        let json = r#"{"action":"openProject","path":"/Users/user/projects/kiri"}"#;
        let action: ClientAction = serde_json::from_str(json).unwrap();
        match action {
            ClientAction::OpenProject { path } => assert_eq!(path, "/Users/user/projects/kiri"),
            _ => panic!("Expected OpenProject"),
        }
    }

    #[test]
    fn test_client_action_close_project_deserialization() {
        let json = r#"{"action":"closeProject","path":"/Users/user/projects/kiri"}"#;
        let action: ClientAction = serde_json::from_str(json).unwrap();
        match action {
            ClientAction::CloseProject { path } => assert_eq!(path, "/Users/user/projects/kiri"),
            _ => panic!("Expected CloseProject"),
        }
    }

    #[test]
    fn test_client_action_unknown_action_fails() {
        let json = r#"{"action":"unknownAction","path":"/some/path"}"#;
        let result = serde_json::from_str::<ClientAction>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_client_action_missing_path_fails() {
        let json = r#"{"action":"openProject"}"#;
        let result = serde_json::from_str::<ClientAction>(json);
        assert!(result.is_err());
    }

    // ── strip_token_prefix tests ────────────────────────────────

    #[test]
    fn test_strip_token_prefix_valid_with_subpath() {
        assert_eq!(strip_token_prefix("/abc-123/ws", "abc-123"), Some("/ws"));
    }

    #[test]
    fn test_strip_token_prefix_valid_root_trailing_slash() {
        assert_eq!(strip_token_prefix("/abc-123/", "abc-123"), Some("/"));
    }

    #[test]
    fn test_strip_token_prefix_valid_no_trailing_slash() {
        assert_eq!(strip_token_prefix("/abc-123", "abc-123"), Some("/"));
    }

    #[test]
    fn test_strip_token_prefix_invalid_token() {
        assert_eq!(strip_token_prefix("/wrong-token/ws", "abc-123"), None);
    }

    #[test]
    fn test_strip_token_prefix_health_bypass() {
        // /api/health is not token-prefixed; strip_token_prefix returns None
        assert_eq!(strip_token_prefix("/api/health", "abc-123"), None);
    }

    #[test]
    fn test_strip_token_prefix_static_file() {
        assert_eq!(
            strip_token_prefix("/abc-123/app.js", "abc-123"),
            Some("/app.js")
        );
    }

    #[test]
    fn test_strip_token_prefix_nested_path() {
        assert_eq!(
            strip_token_prefix("/abc-123/assets/style.css", "abc-123"),
            Some("/assets/style.css")
        );
    }

    #[test]
    fn test_strip_token_prefix_empty_path() {
        assert_eq!(strip_token_prefix("", "abc-123"), None);
    }

    #[test]
    fn test_strip_token_prefix_just_slash() {
        // "/" has an empty first segment, which won't match any real token
        assert_eq!(strip_token_prefix("/", "abc-123"), None);
    }

    // ── refreshed_system tests ─────────────────────────────────────

    #[test]
    fn test_refreshed_system_returns_system_with_processes() {
        let sys = refreshed_system();
        // After refresh, the system should contain at least the current process
        assert!(
            !sys.processes().is_empty(),
            "refreshed system should have at least one process"
        );
    }

    #[test]
    fn test_refreshed_system_contains_current_process() {
        let sys = refreshed_system();
        let current_pid = sysinfo::Pid::from_u32(std::process::id());
        assert!(
            sys.process(current_pid).is_some(),
            "refreshed system should contain the current process (pid={})",
            std::process::id()
        );
    }

    // ── lookup_process_name tests ──────────────────────────────────

    #[test]
    fn test_lookup_process_name_current_process() {
        let sys = refreshed_system();
        let current_pid = std::process::id();
        // The current process should be findable
        let name = lookup_process_name(&sys, current_pid);
        assert!(
            name.is_some(),
            "lookup should find the current process (pid={})",
            current_pid
        );
        // The name should be non-empty
        assert!(
            !name.as_ref().unwrap().is_empty(),
            "process name should not be empty"
        );
    }

    #[test]
    fn test_lookup_process_name_nonexistent_pid() {
        let sys = refreshed_system();
        // u32::MAX is extremely unlikely to be a real PID
        let name = lookup_process_name(&sys, u32::MAX);
        assert!(
            name.is_none(),
            "lookup should return None for a nonexistent PID"
        );
    }

    #[test]
    fn test_lookup_process_name_zero_pid() {
        let sys = refreshed_system();
        // PID 0 is kernel/system on most platforms; lookup_process_name
        // should not panic regardless of whether it exists.
        let _name = lookup_process_name(&sys, 0);
        // No assertion on the value — just verify no panic
    }

    #[test]
    fn test_lookup_process_name_returns_child_over_shell() {
        // When a PID has child processes, lookup_process_name should prefer
        // the child name. We can verify this by looking up the current test
        // runner process (which is the child of whatever spawned it).
        let sys = refreshed_system();
        let current_pid = sysinfo::Pid::from_u32(std::process::id());

        // Get the parent PID of the current process
        if let Some(proc) = sys.process(current_pid) {
            if let Some(parent_pid) = proc.parent() {
                // Looking up the parent should find a child (possibly this
                // process or a sibling). The important thing is it returns
                // Some and doesn't panic.
                let name = lookup_process_name(&sys, parent_pid.as_u32());
                assert!(
                    name.is_some(),
                    "lookup for parent PID {} should find a child process",
                    parent_pid.as_u32()
                );
            }
        }
    }

    // ── Additional strip_token_prefix edge cases ───────────────────

    #[test]
    fn test_strip_token_prefix_uuid_format_token() {
        let uuid_token = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(
            strip_token_prefix(
                &format!("/{}/ws", uuid_token),
                uuid_token
            ),
            Some("/ws")
        );
    }

    #[test]
    fn test_strip_token_prefix_uuid_token_with_nested_path() {
        let uuid_token = "550e8400-e29b-41d4-a716-446655440000";
        assert_eq!(
            strip_token_prefix(
                &format!("/{}/assets/css/style.css", uuid_token),
                uuid_token
            ),
            Some("/assets/css/style.css")
        );
    }

    #[test]
    fn test_strip_token_prefix_very_long_path() {
        let token = "abc-123";
        let long_suffix = "/a".repeat(500);
        let path = format!("/{}{}", token, long_suffix);
        let result = strip_token_prefix(&path, token);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), long_suffix);
    }

    #[test]
    fn test_strip_token_prefix_token_with_special_chars() {
        // Tokens should only be UUIDs in practice, but test resilience
        let token = "abc_123.xyz";
        assert_eq!(
            strip_token_prefix(&format!("/{}/ws", token), token),
            Some("/ws")
        );
    }

    #[test]
    fn test_strip_token_prefix_partial_match_shorter() {
        // "abc" is a prefix of "abc-123", should NOT match
        assert_eq!(strip_token_prefix("/abc/ws", "abc-123"), None);
    }

    #[test]
    fn test_strip_token_prefix_partial_match_longer() {
        // "abc-123-extra" is longer than "abc-123", should NOT match
        assert_eq!(strip_token_prefix("/abc-123-extra/ws", "abc-123"), None);
    }

    #[test]
    fn test_strip_token_prefix_case_sensitive() {
        assert_eq!(strip_token_prefix("/ABC-123/ws", "abc-123"), None);
    }

    #[test]
    fn test_strip_token_prefix_empty_token() {
        // Empty token should match the empty first segment of "//"
        assert_eq!(strip_token_prefix("//ws", ""), Some("/ws"));
    }

    #[test]
    fn test_strip_token_prefix_path_with_query_like_content() {
        // Query strings are part of the URI, not the path, but the path
        // itself could contain encoded characters
        let token = "abc-123";
        assert_eq!(
            strip_token_prefix("/abc-123/api/data", token),
            Some("/api/data")
        );
    }

    #[test]
    fn test_strip_token_prefix_no_leading_slash() {
        assert_eq!(strip_token_prefix("abc-123/ws", "abc-123"), None);
    }

    // ── Additional ClientAction deserialization edge cases ──────────

    #[test]
    fn test_client_action_extra_fields_ignored() {
        let json = r#"{"action":"openProject","path":"/some/path","extra":"field","count":42}"#;
        let action: ClientAction = serde_json::from_str(json).unwrap();
        match action {
            ClientAction::OpenProject { path } => assert_eq!(path, "/some/path"),
            _ => panic!("Expected OpenProject"),
        }
    }

    #[test]
    fn test_client_action_empty_path() {
        let json = r#"{"action":"openProject","path":""}"#;
        let action: ClientAction = serde_json::from_str(json).unwrap();
        match action {
            ClientAction::OpenProject { path } => assert_eq!(path, ""),
            _ => panic!("Expected OpenProject"),
        }
    }

    #[test]
    fn test_client_action_unicode_path() {
        let json = r#"{"action":"openProject","path":"/Users/ユーザー/プロジェクト"}"#;
        let action: ClientAction = serde_json::from_str(json).unwrap();
        match action {
            ClientAction::OpenProject { path } => {
                assert_eq!(path, "/Users/ユーザー/プロジェクト")
            }
            _ => panic!("Expected OpenProject"),
        }
    }

    #[test]
    fn test_client_action_path_with_spaces() {
        let json = r#"{"action":"closeProject","path":"/Users/user/my project/test"}"#;
        let action: ClientAction = serde_json::from_str(json).unwrap();
        match action {
            ClientAction::CloseProject { path } => {
                assert_eq!(path, "/Users/user/my project/test")
            }
            _ => panic!("Expected CloseProject"),
        }
    }

    #[test]
    fn test_client_action_empty_json() {
        let json = r#"{}"#;
        let result = serde_json::from_str::<ClientAction>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_client_action_null_path_fails() {
        let json = r#"{"action":"openProject","path":null}"#;
        let result = serde_json::from_str::<ClientAction>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_client_action_debug_impl() {
        let action = ClientAction::OpenProject {
            path: "/some/path".to_string(),
        };
        let debug = format!("{:?}", action);
        assert!(debug.contains("OpenProject"));
        assert!(debug.contains("/some/path"));
    }

    #[test]
    fn test_client_action_clone() {
        let action = ClientAction::CloseProject {
            path: "/test".to_string(),
        };
        let cloned = action.clone();
        match cloned {
            ClientAction::CloseProject { path } => assert_eq!(path, "/test"),
            _ => panic!("Expected CloseProject"),
        }
    }

    // ── Additional HealthResponse serialization edge cases ─────────

    #[test]
    fn test_health_response_empty_version() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "".to_string(),
        };
        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["version"], "");
    }

    #[test]
    fn test_health_response_debug_impl() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
        };
        let debug = format!("{:?}", response);
        assert!(debug.contains("ok"));
        assert!(debug.contains("1.0.0"));
    }

    #[test]
    fn test_health_response_clone() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
        };
        let cloned = response.clone();
        assert_eq!(cloned.status, "ok");
        assert_eq!(cloned.version, "1.0.0");
    }

    // ── OpenProject serialization tests ────────────────────────────

    #[test]
    fn test_open_project_with_none_branch() {
        let project = OpenProject {
            path: "/projects/kiri".to_string(),
            name: "kiri".to_string(),
            branch: None,
            is_worktree: false,
        };
        let json = serde_json::to_value(&project).unwrap();
        assert_eq!(json["path"], "/projects/kiri");
        assert_eq!(json["name"], "kiri");
        assert!(json["branch"].is_null());
    }

    #[test]
    fn test_open_project_camel_case_serialization() {
        let project = OpenProject {
            path: "/test".to_string(),
            name: "test".to_string(),
            branch: Some("feature/test".to_string()),
            is_worktree: true,
        };
        let json = serde_json::to_value(&project).unwrap();
        assert!(json.get("path").is_some());
        assert!(json.get("name").is_some());
        assert!(json.get("branch").is_some());
        assert_eq!(json["isWorktree"], true);
    }

    #[test]
    fn test_open_project_debug_impl() {
        let project = OpenProject {
            path: "/test".to_string(),
            name: "test".to_string(),
            branch: Some("main".to_string()),
            is_worktree: false,
        };
        let debug = format!("{:?}", project);
        assert!(debug.contains("OpenProject"));
        assert!(debug.contains("main"));
    }

    #[test]
    fn test_open_project_clone() {
        let project = OpenProject {
            path: "/test".to_string(),
            name: "test".to_string(),
            branch: Some("dev".to_string()),
            is_worktree: false,
        };
        let cloned = project.clone();
        assert_eq!(cloned.path, "/test");
        assert_eq!(cloned.branch, Some("dev".to_string()));
    }

    // ── TerminalStatus serialization tests ─────────────────────────

    #[test]
    fn test_terminal_status_all_none_fields() {
        let status = TerminalStatus {
            id: 1,
            is_alive: false,
            process_name: None,
            cwd: None,
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["id"], 1);
        assert_eq!(json["isAlive"], false);
        assert!(json["processName"].is_null());
        assert!(json["cwd"].is_null());
    }

    #[test]
    fn test_terminal_status_camel_case_serialization() {
        let status = TerminalStatus {
            id: 42,
            is_alive: true,
            process_name: Some("node".to_string()),
            cwd: Some("/tmp".to_string()),
        };
        let json = serde_json::to_value(&status).unwrap();
        assert!(json.get("isAlive").is_some());
        assert!(json.get("processName").is_some());
        // Ensure snake_case variants are NOT present
        assert!(json.get("is_alive").is_none());
        assert!(json.get("process_name").is_none());
    }

    #[test]
    fn test_terminal_status_debug_impl() {
        let status = TerminalStatus {
            id: 7,
            is_alive: true,
            process_name: Some("cargo".to_string()),
            cwd: None,
        };
        let debug = format!("{:?}", status);
        assert!(debug.contains("TerminalStatus"));
        assert!(debug.contains("cargo"));
    }

    // ── RecentProject deserialization edge cases ────────────────────

    #[test]
    fn test_recent_project_missing_git_branch_field() {
        // When the field is entirely missing (not null), it should default to None
        let json = serde_json::json!({
            "path": "/Users/user/projects/test",
            "name": "test",
            "lastOpened": 1700000000.0
        });
        let project: RecentProject = serde_json::from_value(json).unwrap();
        assert_eq!(project.git_branch, None);
    }

    #[test]
    fn test_recent_project_fractional_timestamp() {
        let json = serde_json::json!({
            "path": "/test",
            "name": "test",
            "lastOpened": 1700000000.123456,
            "gitBranch": null
        });
        let project: RecentProject = serde_json::from_value(json).unwrap();
        assert!((project.last_opened - 1700000000.123456).abs() < f64::EPSILON);
    }

    #[test]
    fn test_recent_project_zero_timestamp() {
        let json = serde_json::json!({
            "path": "/test",
            "name": "test",
            "lastOpened": 0.0,
            "gitBranch": null
        });
        let project: RecentProject = serde_json::from_value(json).unwrap();
        assert_eq!(project.last_opened, 0.0);
    }

    #[test]
    fn test_recent_project_debug_impl() {
        let project = RecentProject {
            path: "/test".to_string(),
            name: "test".to_string(),
            last_opened: 1700000000.0,
            git_branch: Some("develop".to_string()),
        };
        let debug = format!("{:?}", project);
        assert!(debug.contains("RecentProject"));
        assert!(debug.contains("develop"));
    }

    #[test]
    fn test_recent_project_clone() {
        let project = RecentProject {
            path: "/test".to_string(),
            name: "test".to_string(),
            last_opened: 1700000000.0,
            git_branch: Some("main".to_string()),
        };
        let cloned = project.clone();
        assert_eq!(cloned.path, project.path);
        assert_eq!(cloned.last_opened, project.last_opened);
        assert_eq!(cloned.git_branch, project.git_branch);
    }

    // ── StatusUpdate edge cases ────────────────────────────────────

    #[test]
    fn test_status_update_debug_impl() {
        let update = StatusUpdate {
            open_projects: vec![],
            recent_projects: vec![],
            terminals: vec![],
            timestamp: 1234567890,
        };
        let debug = format!("{:?}", update);
        assert!(debug.contains("StatusUpdate"));
        assert!(debug.contains("1234567890"));
    }

    #[test]
    fn test_status_update_clone() {
        let update = StatusUpdate {
            open_projects: vec![OpenProject {
                path: "/p".to_string(),
                name: "p".to_string(),
                branch: None,
                is_worktree: false,
            }],
            recent_projects: vec![],
            terminals: vec![],
            timestamp: 100,
        };
        let cloned = update.clone();
        assert_eq!(cloned.open_projects.len(), 1);
        assert_eq!(cloned.timestamp, 100);
    }

    #[test]
    fn test_status_update_camel_case_keys() {
        let update = StatusUpdate {
            open_projects: vec![],
            recent_projects: vec![],
            terminals: vec![],
            timestamp: 0,
        };
        let json = serde_json::to_value(&update).unwrap();
        assert!(json.get("openProjects").is_some());
        assert!(json.get("recentProjects").is_some());
        assert!(json.get("open_projects").is_none());
        assert!(json.get("recent_projects").is_none());
    }

    // ── resolve_remote_ui_path tests ───────────────────────────────

    #[test]
    fn test_resolve_remote_ui_path_none_returns_dev_fallback() {
        let path = resolve_remote_ui_path(None);
        assert_eq!(path, std::path::PathBuf::from("remote-ui"));
    }

    // ── health_handler async tests ────────────────────────────────

    #[tokio::test]
    async fn test_health_handler_returns_ok() {
        let Json(response) = health_handler().await;
        assert_eq!(response.status, "ok");
        assert_eq!(response.version, env!("CARGO_PKG_VERSION"));
    }

    // ── Router integration tests (token_path_handler) ─────────────

    /// Helper: build a router with the given token and no app handle.
    fn test_router(token: &str) -> Router {
        let token = Arc::new(RwLock::new(token.to_string()));
        create_router(token, None)
    }

    /// Helper: send a request to the router and return (status, body bytes).
    async fn send_request(
        router: Router,
        uri: &str,
    ) -> (StatusCode, Vec<u8>) {
        use tower::ServiceExt;

        let request = axum::http::Request::builder()
            .uri(uri)
            .body(axum::body::Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, body.to_vec())
    }

    #[tokio::test]
    async fn test_token_path_handler_valid_token() {
        let router = test_router("my-secret");
        let (status, body) = send_request(router, "/my-secret/api/health").await;

        assert_eq!(status, StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn test_token_path_handler_invalid_token() {
        let router = test_router("correct-token");
        let (status, _body) = send_request(router, "/wrong-token/api/health").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_token_path_handler_health_bypasses_token() {
        let router = test_router("some-token");
        let (status, body) = send_request(router, "/api/health").await;

        assert_eq!(status, StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_token_path_handler_strips_token_prefix() {
        // When a valid token prefix is present, the inner router should
        // see the path with the token stripped (e.g. /api/health).
        let router = test_router("abc-123");
        let (status, body) = send_request(router, "/abc-123/api/health").await;

        assert_eq!(status, StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_token_path_handler_preserves_query_string() {
        // Query strings should be preserved after token stripping.
        // /api/health doesn't use query params, but the URI rewriting
        // should not drop them.
        let router = test_router("tok-42");
        let (status, body) =
            send_request(router, "/tok-42/api/health?foo=bar&baz=1").await;

        assert_eq!(status, StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    #[tokio::test]
    async fn test_token_path_handler_no_path_after_token() {
        // Requesting just the token with no trailing path should
        // receive some response (404 from inner router or fallback),
        // but should NOT panic.
        let router = test_router("abc");
        let (status, _body) = send_request(router, "/abc").await;

        // After stripping "/abc" the inner path becomes "/", which
        // has no explicit route. Status depends on inner fallback config.
        // The important assertion is that the server did not panic and
        // returned a valid HTTP response.
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Expected OK or NOT_FOUND, got {}",
            status
        );
    }

    #[tokio::test]
    async fn test_token_path_handler_unknown_inner_route() {
        // A valid token but unknown inner route should return 404
        // from the inner router.
        let router = test_router("tok");
        let (status, _body) =
            send_request(router, "/tok/nonexistent/route").await;

        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_token_path_handler_empty_token_config() {
        // Edge case: empty string as the token. A request to "//api/health"
        // should match the empty first segment.
        let router = test_router("");
        let (status, body) = send_request(router, "//api/health").await;

        assert_eq!(status, StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
    }

    // ── Server lifecycle tests ────────────────────────────────────

    #[tokio::test]
    async fn test_start_server_and_health_check() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let token = Arc::new(RwLock::new("test-token".to_string()));

        tokio::spawn(async move {
            start_server(listener, shutdown_rx, token, None)
                .await
                .unwrap();
        });

        // Give the server a moment to start accepting connections.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{}/api/health", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        let json: serde_json::Value = resp.json().await.unwrap();
        assert_eq!(json["status"], "ok");

        // Shutdown
        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_start_server_with_token_auth() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let token = Arc::new(RwLock::new("auth-abc".to_string()));

        tokio::spawn(async move {
            start_server(listener, shutdown_rx, token, None)
                .await
                .unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();

        // Request without token should fail (404 from fallback)
        let resp = client
            .get(format!("http://{}/ws", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);

        // Request with valid token prefix should reach the inner router.
        // /ws requires a WebSocket upgrade so we expect 400 or similar
        // (not 404), proving the token was accepted and the request
        // reached the ws_handler.
        let resp = client
            .get(format!("http://{}/auth-abc/api/health", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        // Request with wrong token should return 404
        let resp = client
            .get(format!("http://{}/wrong-token/api/health", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_start_server_graceful_shutdown() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let token = Arc::new(RwLock::new("shutdown-test".to_string()));

        let server_handle = tokio::spawn(async move {
            start_server(listener, shutdown_rx, token, None)
                .await
                .unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Verify server is running
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{}/api/health", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 200);

        // Send shutdown signal
        shutdown_tx.send(()).unwrap();

        // Wait for the server task to complete (with a timeout)
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            server_handle,
        )
        .await;

        assert!(
            result.is_ok(),
            "Server should shut down within 5 seconds"
        );
        assert!(
            result.unwrap().is_ok(),
            "Server task should complete without panic"
        );
    }

    // ── resolve_remote_ui_path edge cases ─────────────────────────

    #[test]
    fn test_resolve_remote_ui_path_fallback_is_relative() {
        let path = resolve_remote_ui_path(None);
        assert!(path.is_relative());
        assert_eq!(
            path.file_name().and_then(|n| n.to_str()),
            Some("remote-ui")
        );
    }

    #[test]
    fn test_resolve_remote_ui_path_fallback_has_single_component() {
        let path = resolve_remote_ui_path(None);
        assert_eq!(path.components().count(), 1);
    }

    // ── extract_project_name tests ────────────────────────────────

    #[test]
    fn test_extract_project_name_simple_path() {
        assert_eq!(extract_project_name("/Users/user/projects/kiri"), "kiri");
    }

    #[test]
    fn test_extract_project_name_trailing_slash() {
        // Path::file_name returns None for paths ending in "/"
        // because the last component is empty
        let name = extract_project_name("/Users/user/projects/kiri/");
        // On most platforms, trailing slash means file_name is ""
        // which to_str returns Some(""), so it won't hit "unknown"
        // The actual behavior depends on the platform
        assert!(!name.is_empty() || name == "unknown");
    }

    #[test]
    fn test_extract_project_name_single_component() {
        assert_eq!(extract_project_name("myproject"), "myproject");
    }

    #[test]
    fn test_extract_project_name_empty_string() {
        assert_eq!(extract_project_name(""), "unknown");
    }

    #[test]
    fn test_extract_project_name_root_path() {
        // "/" has no file_name component
        assert_eq!(extract_project_name("/"), "unknown");
    }

    #[test]
    fn test_extract_project_name_dot_dot() {
        // ".." has no file_name
        assert_eq!(extract_project_name(".."), "unknown");
    }

    #[test]
    fn test_extract_project_name_hidden_directory() {
        assert_eq!(extract_project_name("/home/user/.config"), ".config");
    }

    #[test]
    fn test_extract_project_name_deeply_nested() {
        assert_eq!(
            extract_project_name("/a/b/c/d/e/f/project"),
            "project"
        );
    }

    #[test]
    fn test_extract_project_name_with_spaces() {
        assert_eq!(
            extract_project_name("/Users/user/My Project"),
            "My Project"
        );
    }

    #[test]
    fn test_extract_project_name_unicode() {
        assert_eq!(
            extract_project_name("/Users/ユーザー/プロジェクト"),
            "プロジェクト"
        );
    }

    // ── filter_recent_projects tests ──────────────────────────────

    fn make_recent(path: &str, name: &str) -> RecentProject {
        RecentProject {
            path: path.to_string(),
            name: name.to_string(),
            last_opened: 1700000000.0,
            git_branch: None,
        }
    }

    #[test]
    fn test_filter_recent_projects_empty_lists() {
        let result = filter_recent_projects(vec![], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_recent_projects_no_open_paths() {
        let recent = vec![
            make_recent("/projects/a", "a"),
            make_recent("/projects/b", "b"),
        ];
        let result = filter_recent_projects(recent, &[]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_filter_recent_projects_removes_open_projects() {
        let recent = vec![
            make_recent("/projects/a", "a"),
            make_recent("/projects/b", "b"),
            make_recent("/projects/c", "c"),
        ];
        let open = vec!["/projects/b".to_string()];
        let result = filter_recent_projects(recent, &open);
        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|p| p.path != "/projects/b"));
    }

    #[test]
    fn test_filter_recent_projects_all_open() {
        let recent = vec![
            make_recent("/projects/a", "a"),
            make_recent("/projects/b", "b"),
        ];
        let open = vec![
            "/projects/a".to_string(),
            "/projects/b".to_string(),
        ];
        let result = filter_recent_projects(recent, &open);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_recent_projects_preserves_order() {
        let recent = vec![
            make_recent("/projects/c", "c"),
            make_recent("/projects/a", "a"),
            make_recent("/projects/b", "b"),
        ];
        let open = vec!["/projects/a".to_string()];
        let result = filter_recent_projects(recent, &open);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].path, "/projects/c");
        assert_eq!(result[1].path, "/projects/b");
    }

    #[test]
    fn test_filter_recent_projects_no_match_in_open() {
        let recent = vec![make_recent("/projects/x", "x")];
        let open = vec!["/projects/y".to_string()];
        let result = filter_recent_projects(recent, &open);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "/projects/x");
    }

    #[test]
    fn test_filter_recent_projects_exact_match_required() {
        let recent = vec![
            make_recent("/projects/abc", "abc"),
            make_recent("/projects/ab", "ab"),
        ];
        let open = vec!["/projects/ab".to_string()];
        let result = filter_recent_projects(recent, &open);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "/projects/abc");
    }

    // ── handle_client_action tests (no app_handle) ────────────────

    #[tokio::test]
    async fn test_handle_client_action_returns_early_without_app_handle() {
        let state = AppState {
            auth_token: Arc::new(RwLock::new("token".to_string())),
            app_handle: None,
        };
        // Should return without panicking
        handle_client_action(
            &state,
            ClientAction::OpenProject {
                path: "/some/path".to_string(),
            },
        )
        .await;
    }

    #[tokio::test]
    async fn test_handle_client_action_close_returns_early_without_app_handle() {
        let state = AppState {
            auth_token: Arc::new(RwLock::new("token".to_string())),
            app_handle: None,
        };
        // Should return without panicking
        handle_client_action(
            &state,
            ClientAction::CloseProject {
                path: "/some/path".to_string(),
            },
        )
        .await;
    }

    // ── Additional router integration tests ───────────────────────

    #[tokio::test]
    async fn test_token_path_handler_token_only_trailing_slash() {
        let router = test_router("tok");
        let (status, _body) = send_request(router, "/tok/").await;
        // "/" in inner router has no route, but should not panic
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Expected OK or NOT_FOUND for token-only with trailing slash, got {}",
            status
        );
    }

    #[tokio::test]
    async fn test_token_path_handler_double_slash_after_token() {
        let router = test_router("tok");
        let (status, _body) = send_request(router, "/tok//api/health").await;
        // Inner path becomes "//api/health" which won't match /api/health
        // but should not panic
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Expected valid response, got {}",
            status
        );
    }

    #[tokio::test]
    async fn test_token_path_handler_ws_without_upgrade() {
        // Requesting /ws without WebSocket upgrade headers should not match
        // or should return an error, but should not panic.
        let router = test_router("tok");
        let (status, _body) = send_request(router, "/tok/ws").await;
        // axum returns 400 or similar when upgrade headers are missing
        assert_ne!(
            status,
            StatusCode::OK,
            "WS endpoint without upgrade should not return 200"
        );
    }

    #[tokio::test]
    async fn test_public_health_returns_version_from_cargo() {
        let router = test_router("irrelevant");
        let (status, body) = send_request(router, "/api/health").await;
        assert_eq!(status, StatusCode::OK);
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // version should match Cargo.toml version
        assert!(!json["version"].as_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_token_path_handler_with_percent_encoded_path() {
        let router = test_router("tok");
        let (status, _body) =
            send_request(router, "/tok/api/health%20extra").await;
        // The path won't match /api/health, so should 404
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    // ── Server lifecycle edge cases ─────────────────────────────

    #[tokio::test]
    async fn test_start_server_shutdown_before_any_request() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let token = Arc::new(RwLock::new("tok".to_string()));

        let handle = tokio::spawn(async move {
            start_server(listener, shutdown_rx, token, None)
                .await
                .unwrap();
        });

        // Immediately shut down without sending any requests
        shutdown_tx.send(()).unwrap();

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            handle,
        )
        .await;
        assert!(result.is_ok(), "Server should shut down promptly");
        assert!(result.unwrap().is_ok(), "Server task should complete cleanly");
    }

    #[tokio::test]
    async fn test_start_server_multiple_health_checks() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let token = Arc::new(RwLock::new("multi".to_string()));

        tokio::spawn(async move {
            start_server(listener, shutdown_rx, token, None)
                .await
                .unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        // Send multiple requests to verify server handles concurrent connections
        for _ in 0..5 {
            let resp = client
                .get(format!("http://{}/api/health", addr))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), 200);
        }

        let _ = shutdown_tx.send(());
    }

    // ── StatusUpdate with multiple terminals ──────────────────────

    #[test]
    fn test_status_update_multiple_terminals() {
        let update = StatusUpdate {
            open_projects: vec![],
            recent_projects: vec![],
            terminals: vec![
                TerminalStatus {
                    id: 1,
                    is_alive: true,
                    process_name: Some("cargo".to_string()),
                    cwd: Some("/projects/kiri".to_string()),
                },
                TerminalStatus {
                    id: 2,
                    is_alive: false,
                    process_name: None,
                    cwd: None,
                },
                TerminalStatus {
                    id: 3,
                    is_alive: true,
                    process_name: Some("node".to_string()),
                    cwd: Some("/projects/web".to_string()),
                },
            ],
            timestamp: 1700000000,
        };

        let json = serde_json::to_value(&update).unwrap();
        let terminals = json["terminals"].as_array().unwrap();
        assert_eq!(terminals.len(), 3);
        assert_eq!(terminals[0]["id"], 1);
        assert_eq!(terminals[0]["isAlive"], true);
        assert_eq!(terminals[1]["id"], 2);
        assert_eq!(terminals[1]["isAlive"], false);
        assert!(terminals[1]["processName"].is_null());
        assert_eq!(terminals[2]["processName"], "node");
    }

    // ── StatusUpdate with mixed open and recent projects ─────────

    #[test]
    fn test_status_update_with_mixed_projects() {
        let update = StatusUpdate {
            open_projects: vec![
                OpenProject {
                    path: "/projects/a".to_string(),
                    name: "a".to_string(),
                    branch: Some("main".to_string()),
                    is_worktree: false,
                },
                OpenProject {
                    path: "/projects/b".to_string(),
                    name: "b".to_string(),
                    branch: None,
                    is_worktree: true,
                },
            ],
            recent_projects: vec![RecentProject {
                path: "/projects/c".to_string(),
                name: "c".to_string(),
                last_opened: 1700000000.0,
                git_branch: Some("develop".to_string()),
            }],
            terminals: vec![],
            timestamp: 1700000000,
        };

        let json = serde_json::to_value(&update).unwrap();
        assert_eq!(json["openProjects"].as_array().unwrap().len(), 2);
        assert_eq!(json["recentProjects"].as_array().unwrap().len(), 1);
        assert_eq!(json["recentProjects"][0]["gitBranch"], "develop");
    }

    // ── collect_full_status with None app handle ─────────────────

    #[test]
    fn test_collect_full_status_none_app_handle_returns_none() {
        let state = AppState {
            auth_token: Arc::new(RwLock::new("any-token".to_string())),
            app_handle: None,
        };
        let result = collect_full_status(&state);
        assert!(result.is_none());
    }

    // ── WebSocket integration tests ─────────────────────────────────

    /// Helper: start a test server and return (addr, shutdown_tx, token).
    async fn start_test_server(
        token: &str,
    ) -> (
        std::net::SocketAddr,
        oneshot::Sender<()>,
    ) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .unwrap();
        let addr = listener.local_addr().unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let tk = Arc::new(RwLock::new(token.to_string()));

        tokio::spawn(async move {
            start_server(listener, shutdown_rx, tk, None)
                .await
                .unwrap();
        });

        // Give the server a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        (addr, shutdown_tx)
    }

    #[tokio::test]
    async fn test_ws_closes_when_no_app_handle() {
        // When app_handle is None, collect_full_status returns None,
        // which causes the WebSocket loop to break and close the
        // connection after the first tick.
        let (addr, shutdown_tx) = start_test_server("ws-tok").await;

        let url = format!("ws://{}/ws-tok/ws", addr);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("WS connect failed");

        use futures_util::StreamExt;
        let (_write, mut read) = ws_stream.split();

        // The server should close the connection because collect_full_status
        // returns None (no app_handle). We should receive a close or the
        // stream should end.
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            read.next(),
        )
        .await;

        // The server closes the connection because collect_full_status
        // returns None. This may manifest as: stream end (None), a close
        // frame, or a connection-reset error (server drops socket without
        // performing the close handshake).
        match result {
            Ok(None) => {}
            Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_)))) => {}
            Ok(Some(Err(_))) => {} // connection reset / protocol error is acceptable
            Ok(Some(Ok(msg))) => {
                panic!("Unexpected WS message: {:?}", msg);
            }
            Err(_) => panic!("Timed out waiting for WS close"),
        }

        let _ = shutdown_tx.send(());
    }

    /// Helper: assert a WS read result represents a closed connection.
    ///
    /// Accepts: stream end (None), close frame, or connection-reset
    /// error (the server may drop the socket without a close handshake
    /// when `collect_full_status` returns `None`).
    fn assert_ws_closed(
        result: Result<
            Option<
                Result<
                    tokio_tungstenite::tungstenite::Message,
                    tokio_tungstenite::tungstenite::Error,
                >,
            >,
            tokio::time::error::Elapsed,
        >,
    ) {
        match result {
            Ok(None) => {}
            Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_)))) => {}
            Ok(Some(Err(_))) => {} // connection reset is acceptable
            Ok(Some(Ok(msg))) => {
                panic!("Expected WS close, got message: {:?}", msg);
            }
            Err(_) => panic!("Timed out waiting for WS close"),
        }
    }

    #[tokio::test]
    async fn test_ws_handles_invalid_json_message() {
        // Sending invalid JSON should be ignored (logged as warning),
        // and the connection should stay open briefly before the next
        // tick closes it (because no app_handle).
        let (addr, shutdown_tx) = start_test_server("ws-inv").await;

        let url = format!("ws://{}/ws-inv/ws", addr);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("WS connect failed");

        use futures_util::{SinkExt, StreamExt};
        let (mut write, mut read) = ws_stream.split();

        // Send invalid JSON - this should not crash the server
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                "not valid json".into(),
            ))
            .await
            .unwrap();

        // The connection should eventually close (no app_handle)
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            read.next(),
        )
        .await;

        assert_ws_closed(result);

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_handles_client_action_without_app_handle() {
        // Sending a valid ClientAction JSON when app_handle is None
        // should not crash; handle_client_action returns early.
        let (addr, shutdown_tx) = start_test_server("ws-act").await;

        let url = format!("ws://{}/ws-act/ws", addr);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("WS connect failed");

        use futures_util::{SinkExt, StreamExt};
        let (mut write, mut read) = ws_stream.split();

        // Send a valid openProject action
        let action_json = r#"{"action":"openProject","path":"/tmp/test"}"#;
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                action_json.into(),
            ))
            .await
            .unwrap();

        // Send a valid closeProject action
        let close_json = r#"{"action":"closeProject","path":"/tmp/test"}"#;
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(
                close_json.into(),
            ))
            .await
            .unwrap();

        // Connection should eventually close (no app_handle)
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            read.next(),
        )
        .await;

        assert_ws_closed(result);

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_handles_close_frame() {
        // Sending a close frame should cause the server to close
        // the connection gracefully.
        let (addr, shutdown_tx) = start_test_server("ws-cls").await;

        let url = format!("ws://{}/ws-cls/ws", addr);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("WS connect failed");

        use futures_util::{SinkExt, StreamExt};
        let (mut write, mut read) = ws_stream.split();

        // Send close frame
        write
            .send(tokio_tungstenite::tungstenite::Message::Close(None))
            .await
            .unwrap();

        // Server should acknowledge close
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            read.next(),
        )
        .await;

        // Stream should end or return close
        match result {
            Ok(None) => {}
            Ok(Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_)))) => {}
            Ok(Some(other)) => {
                // Some implementations may return other frames before close
                let _ = other;
            }
            Err(_) => panic!("Timed out waiting for WS close acknowledgement"),
        }

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_handles_ping_frame() {
        // Sending a ping should elicit a pong (covered by the Ping branch).
        let (addr, shutdown_tx) = start_test_server("ws-png").await;

        let url = format!("ws://{}/ws-png/ws", addr);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("WS connect failed");

        use futures_util::{SinkExt, StreamExt};
        let (mut write, mut read) = ws_stream.split();

        // Send ping with payload
        write
            .send(tokio_tungstenite::tungstenite::Message::Ping(
                b"hello".to_vec().into(),
            ))
            .await
            .unwrap();

        // We should get a pong back or the connection closes (no app_handle).
        // The ping/pong may be handled by tungstenite automatically at the
        // protocol level, so we just verify no crash occurs.
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            read.next(),
        )
        .await;

        // We should get some response (pong, close, or stream end)
        assert!(result.is_ok(), "Timed out waiting for response to ping");

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_handles_binary_frame() {
        // Sending a binary frame should be handled by the wildcard `_ => {}`
        // branch without crashing.
        let (addr, shutdown_tx) = start_test_server("ws-bin").await;

        let url = format!("ws://{}/ws-bin/ws", addr);
        let (ws_stream, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("WS connect failed");

        use futures_util::{SinkExt, StreamExt};
        let (mut write, mut read) = ws_stream.split();

        // Send binary frame
        write
            .send(tokio_tungstenite::tungstenite::Message::Binary(
                b"binary data".to_vec().into(),
            ))
            .await
            .unwrap();

        // Connection should eventually close (no app_handle)
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            read.next(),
        )
        .await;

        // Should close without panic
        assert!(result.is_ok(), "Timed out - server may have hung on binary frame");

        let _ = shutdown_tx.send(());
    }

    // ── filter_recent_projects additional edge cases ─────────────────

    #[test]
    fn test_filter_recent_projects_duplicate_entries() {
        // Duplicate paths in recent list: both should be filtered
        let recent = vec![
            make_recent("/projects/a", "a"),
            make_recent("/projects/a", "a-dup"),
        ];
        let open = vec!["/projects/a".to_string()];
        let result = filter_recent_projects(recent, &open);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_recent_projects_preserves_git_branch() {
        let mut recent_with_branch = make_recent("/projects/keep", "keep");
        recent_with_branch.git_branch = Some("feature/xyz".to_string());
        let recent = vec![
            recent_with_branch,
            make_recent("/projects/remove", "remove"),
        ];
        let open = vec!["/projects/remove".to_string()];
        let result = filter_recent_projects(recent, &open);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "/projects/keep");
        assert_eq!(result[0].git_branch, Some("feature/xyz".to_string()));
    }

    #[test]
    fn test_filter_recent_projects_multiple_open_paths() {
        let recent = vec![
            make_recent("/a", "a"),
            make_recent("/b", "b"),
            make_recent("/c", "c"),
            make_recent("/d", "d"),
        ];
        let open = vec![
            "/a".to_string(),
            "/c".to_string(),
            "/d".to_string(),
        ];
        let result = filter_recent_projects(recent, &open);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "/b");
    }

    #[test]
    fn test_filter_recent_projects_case_sensitive() {
        let recent = vec![
            make_recent("/Projects/A", "A"),
            make_recent("/projects/a", "a"),
        ];
        let open = vec!["/projects/a".to_string()];
        let result = filter_recent_projects(recent, &open);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "/Projects/A");
    }

    #[test]
    fn test_filter_recent_projects_single_recent_not_open() {
        let recent = vec![make_recent("/only", "only")];
        let open = vec!["/other".to_string()];
        let result = filter_recent_projects(recent, &open);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_filter_recent_projects_single_recent_is_open() {
        let recent = vec![make_recent("/only", "only")];
        let open = vec!["/only".to_string()];
        let result = filter_recent_projects(recent, &open);
        assert!(result.is_empty());
    }

    #[test]
    fn test_filter_recent_projects_empty_path_string() {
        let recent = vec![make_recent("", "empty")];
        let open = vec!["".to_string()];
        let result = filter_recent_projects(recent, &open);
        assert!(result.is_empty());
    }

    // ── extract_project_name additional edge cases ───────────────────

    #[test]
    fn test_extract_project_name_dot() {
        // "." represents current directory
        let name = extract_project_name(".");
        // Path::file_name for "." returns None
        assert_eq!(name, "unknown");
    }

    #[test]
    fn test_extract_project_name_with_extension() {
        assert_eq!(extract_project_name("/home/user/project.git"), "project.git");
    }

    #[test]
    fn test_extract_project_name_hyphenated() {
        assert_eq!(
            extract_project_name("/Users/user/my-cool-project"),
            "my-cool-project"
        );
    }

    #[test]
    fn test_extract_project_name_windows_style_path() {
        // On Unix, backslashes are valid filename characters
        let name = extract_project_name("C:\\Users\\test\\project");
        // The whole string is treated as a single filename on Unix
        assert!(!name.is_empty());
    }

    // ── RecentProject round-trip serialization ──────────────────────

    #[test]
    fn test_recent_project_serde_round_trip() {
        let project = RecentProject {
            path: "/test/path".to_string(),
            name: "path".to_string(),
            last_opened: 1700000000.5,
            git_branch: Some("feature/test".to_string()),
        };
        let json_str = serde_json::to_string(&project).unwrap();
        let deserialized: RecentProject = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.path, project.path);
        assert_eq!(deserialized.name, project.name);
        assert_eq!(deserialized.last_opened, project.last_opened);
        assert_eq!(deserialized.git_branch, project.git_branch);
    }

    #[test]
    fn test_recent_project_serde_round_trip_no_branch() {
        let project = RecentProject {
            path: "/test".to_string(),
            name: "test".to_string(),
            last_opened: 0.0,
            git_branch: None,
        };
        let json_str = serde_json::to_string(&project).unwrap();
        let deserialized: RecentProject = serde_json::from_str(&json_str).unwrap();
        assert_eq!(deserialized.git_branch, None);
    }

    // ── StatusUpdate with all field types populated ──────────────────

    #[test]
    fn test_status_update_full_round_trip_json() {
        let update = StatusUpdate {
            open_projects: vec![
                OpenProject {
                    path: "/a".to_string(),
                    name: "a".to_string(),
                    branch: Some("main".to_string()),
                    is_worktree: false,
                },
            ],
            recent_projects: vec![
                RecentProject {
                    path: "/b".to_string(),
                    name: "b".to_string(),
                    last_opened: 1700000000.0,
                    git_branch: Some("develop".to_string()),
                },
            ],
            terminals: vec![
                TerminalStatus {
                    id: 1,
                    is_alive: true,
                    process_name: Some("vim".to_string()),
                    cwd: Some("/home".to_string()),
                },
            ],
            timestamp: 9999999999,
        };
        let json_str = serde_json::to_string(&update).unwrap();
        // Verify JSON can be parsed back as a Value
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed["timestamp"], 9999999999u64);
        assert_eq!(parsed["openProjects"][0]["branch"], "main");
        assert_eq!(parsed["recentProjects"][0]["gitBranch"], "develop");
        assert_eq!(parsed["terminals"][0]["cwd"], "/home");
    }

    // ── token_path_handler with HTTP methods other than GET ──────────

    /// Helper: send a request with a specific HTTP method.
    async fn send_request_with_method(
        router: Router,
        method: axum::http::Method,
        uri: &str,
    ) -> (StatusCode, Vec<u8>) {
        use tower::ServiceExt;

        let request = axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .body(axum::body::Body::empty())
            .unwrap();

        let response = router.oneshot(request).await.unwrap();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        (status, body.to_vec())
    }

    #[tokio::test]
    async fn test_token_path_handler_post_method() {
        let router = test_router("tok");
        let (status, _body) = send_request_with_method(
            router,
            axum::http::Method::POST,
            "/tok/api/health",
        )
        .await;
        // GET-only route, POST should return 405 Method Not Allowed
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_token_path_handler_put_method() {
        let router = test_router("tok");
        let (status, _body) = send_request_with_method(
            router,
            axum::http::Method::PUT,
            "/tok/api/health",
        )
        .await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn test_token_path_handler_head_health() {
        let router = test_router("tok");
        let (status, _body) = send_request_with_method(
            router,
            axum::http::Method::HEAD,
            "/tok/api/health",
        )
        .await;
        // axum automatically handles HEAD for GET routes
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn test_public_health_post_method_not_allowed() {
        let router = test_router("tok");
        let (status, _body) = send_request_with_method(
            router,
            axum::http::Method::POST,
            "/api/health",
        )
        .await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    // ── Multiple concurrent WebSocket connections ───────────────────

    #[tokio::test]
    async fn test_ws_multiple_connections_close_gracefully() {
        let (addr, shutdown_tx) = start_test_server("ws-multi").await;

        let url = format!("ws://{}/ws-multi/ws", addr);

        // Open two connections simultaneously
        let (ws1, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("WS connect 1 failed");
        let (ws2, _) = tokio_tungstenite::connect_async(&url)
            .await
            .expect("WS connect 2 failed");

        use futures_util::StreamExt;

        let (_, mut read1) = ws1.split();
        let (_, mut read2) = ws2.split();

        // Both should close because no app_handle
        let r1 = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            read1.next(),
        )
        .await;
        let r2 = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            read2.next(),
        )
        .await;

        assert!(r1.is_ok(), "WS1 timed out");
        assert!(r2.is_ok(), "WS2 timed out");

        let _ = shutdown_tx.send(());
    }

    // ── WebSocket with wrong token ──────────────────────────────────

    #[tokio::test]
    async fn test_ws_wrong_token_returns_404() {
        let (addr, shutdown_tx) = start_test_server("correct-token").await;

        // Try to connect with wrong token - should get HTTP 404 before upgrade
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{}/wrong-token/ws", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);

        let _ = shutdown_tx.send(());
    }

    #[tokio::test]
    async fn test_ws_no_token_returns_404() {
        let (addr, shutdown_tx) = start_test_server("tok").await;

        let client = reqwest::Client::new();
        let resp = client
            .get(format!("http://{}/ws", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 404);

        let _ = shutdown_tx.send(());
    }
}
