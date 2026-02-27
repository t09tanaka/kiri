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

    all_recent
        .into_iter()
        .filter(|p| !open_paths.contains(&p.path))
        .collect()
}

/// Create a refreshed `sysinfo::System` for process lookups.
///
/// Call this once, then pass the result to [`lookup_process_name`] for
/// each terminal.  This avoids creating N `System` instances (and N
/// full process-table scans) when there are N terminals.
fn refreshed_system() -> sysinfo::System {
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::All);
    sys
}

/// Look up the foreground process name for a given shell PID.
///
/// Uses the provided, already-refreshed [`sysinfo::System`] to find
/// child processes of the shell.  Returns the child's name if one
/// exists, otherwise the shell's own name.
fn lookup_process_name(sys: &sysinfo::System, shell_pid: u32) -> Option<String> {
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
                let status = collect_full_status(&state);
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

    let open_projects: Vec<OpenProject> = open_paths
        .iter()
        .map(|path| {
            let name = std::path::Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            OpenProject {
                path: path.clone(),
                name,
                branch: None,
            }
        })
        .collect();

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
}
