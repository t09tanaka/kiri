//! Core HTTP server logic for remote access
//!
//! Provides an embedded axum HTTP server that can be started and stopped
//! at runtime. The server exposes REST API endpoints for remote control
//! of the kiri application, plus a WebSocket endpoint for real-time
//! status updates.
//!
//! All `/api/*` routes (except `/api/health`) are protected by bearer-token
//! authentication via the `Authorization` header.  WebSocket `/ws/*` routes
//! authenticate via a `token` query parameter instead, because the browser
//! `WebSocket` API does not support custom request headers.

use axum::{
    extract::{
        ws::{Message, WebSocket},
        FromRequest, Query, Request, State, WebSocketUpgrade,
    },
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{any, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::sync::{oneshot, RwLock};

/// Shared application state passed to handlers and middleware.
#[derive(Clone)]
pub struct AppState {
    /// The bearer token required to access protected endpoints.
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

// ── Project API types ────────────────────────────────────────────

/// Response payload for the project list endpoint.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectsResponse {
    pub open_projects: Vec<OpenProject>,
    pub recent_projects: Vec<RecentProject>,
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

/// Request payload for opening a project.
#[derive(Debug, Clone, Deserialize)]
pub struct OpenProjectRequest {
    pub path: String,
}

/// Request payload for closing a project.
#[derive(Debug, Clone, Deserialize)]
pub struct CloseProjectRequest {
    pub path: String,
}

// ── Terminal API types ──────────────────────────────────────────

/// Response payload for the terminal list endpoint.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalStatusResponse {
    pub terminals: Vec<TerminalStatus>,
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

/// Handler for `POST /api/auth/verify`.
///
/// If the request reaches this handler the auth middleware has already
/// validated the bearer token, so we simply return `{ "valid": true }`.
pub async fn verify_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "valid": true }))
}

/// Handler for `GET /api/projects`.
///
/// Returns both currently open projects (from WindowRegistry) and
/// recently opened projects (from the settings store). Recent projects
/// that are currently open are excluded from the recent list.
pub async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<ProjectsResponse>, StatusCode> {
    use tauri::Manager;

    let app = state
        .app_handle
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // Get open projects from WindowRegistry
    // NOTE: std::sync::Mutex -- keep lock scope minimal, never hold across await points
    let registry = app.state::<crate::commands::WindowRegistryState>();
    let open_paths = {
        let reg = registry.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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

    // Get recent projects from settings store
    let recent_projects = load_recent_projects(app, &open_paths);

    Ok(Json(ProjectsResponse {
        open_projects,
        recent_projects,
    }))
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

/// Handler for `POST /api/projects/open`.
///
/// Opens a project in a new window. If a window for the given path
/// already exists, it is focused instead.
pub async fn open_project(
    State(state): State<AppState>,
    Json(req): Json<OpenProjectRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use tauri::Manager;

    let app = state
        .app_handle
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let registry = app.state::<crate::commands::WindowRegistryState>();

    // Check if project is already open
    // NOTE: std::sync::Mutex -- keep lock scope minimal, never hold across await points
    let existing_label = {
        let reg = registry.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        reg.get_label_for_path(&req.path).cloned()
    };

    if let Some(label) = existing_label {
        // Focus existing window
        if let Some(window) = app.get_webview_window(&label) {
            window
                .set_focus()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            return Ok(Json(serde_json::json!({
                "success": true, "path": req.path, "action": "focused"
            })));
        }
        // Window gone but registry stale -- fall through to create
    }

    // Create new window
    crate::commands::window::create_window_impl(
        app,
        Some(&registry),
        None,
        None,
        None,
        None,
        Some(req.path.clone()),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({
        "success": true, "path": req.path, "action": "opened"
    })))
}

/// Handler for `POST /api/projects/close`.
///
/// Closes the window associated with the given project path.
/// Returns 404 if no window is found for the path.
pub async fn close_project(
    State(state): State<AppState>,
    Json(req): Json<CloseProjectRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    use tauri::Manager;

    let app = state
        .app_handle
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    // NOTE: std::sync::Mutex -- keep lock scope minimal, never hold across await points
    let registry = app.state::<crate::commands::WindowRegistryState>();
    let label = {
        let reg = registry.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        reg.get_label_for_path(&req.path).cloned()
    };

    match label {
        Some(label) => {
            if let Some(window) = app.get_webview_window(&label) {
                window
                    .close()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
            Ok(Json(serde_json::json!({ "success": true })))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

// ── Terminal handler ─────────────────────────────────────────────

/// Handler for `GET /api/terminals`.
///
/// Returns all terminal instances with their current process info.
/// The endpoint requires authentication (bearer token).
/// Returns 503 if no Tauri `AppHandle` is available.
pub async fn get_terminals(
    State(state): State<AppState>,
) -> Result<Json<TerminalStatusResponse>, StatusCode> {
    use tauri::Manager;

    let app = state
        .app_handle
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let terminal_state = app.state::<crate::commands::TerminalState>();

    // Collect IDs and shell PIDs while holding the lock briefly.
    // We need mutable access because try_wait takes &mut self.
    let terminal_snapshots: Vec<(u32, bool, Option<u32>)> = {
        let mut manager = terminal_state
            .lock()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        manager
            .instances
            .iter_mut()
            .map(|(&id, instance)| {
                let is_alive = instance
                    .child
                    .try_wait()
                    .map(|status| status.is_none())
                    .unwrap_or(false);
                (id, is_alive, instance.shell_pid)
            })
            .collect()
    };

    // Refresh process table once, then look up each terminal.
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
            cwd: None, // CWD lookup is expensive; skip for list
        });
    }

    Ok(Json(TerminalStatusResponse { terminals }))
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

/// Handler for `GET /ws/status`.
///
/// Upgrades to a WebSocket connection that pushes a combined status
/// update (open projects, recent projects, terminals) every 2 seconds.
///
/// Authentication is checked **before** the WebSocket upgrade so that
/// unauthenticated requests always receive `401 Unauthorized`, even if
/// the request lacks the `Upgrade: websocket` header.
///
/// The browser WebSocket API does not support custom headers, so the
/// token is passed via a `token` query parameter.  The auth middleware
/// is skipped for `/ws/` paths -- this handler does its own auth.
pub async fn ws_status(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
    request: Request,
) -> Result<Response, StatusCode> {
    // Validate token from query parameter -- checked BEFORE the upgrade
    let token = params.get("token").ok_or(StatusCode::UNAUTHORIZED)?;
    let expected = state.auth_token.read().await;

    let token_bytes = token.as_bytes();
    let expected_bytes = expected.as_bytes();
    if token_bytes.len() != expected_bytes.len()
        || !bool::from(token_bytes.ct_eq(expected_bytes))
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    drop(expected);

    // Attempt the WebSocket upgrade from the raw request
    let ws = WebSocketUpgrade::from_request(request, &())
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(ws
        .on_upgrade(|socket| handle_status_ws(socket, state))
        .into_response())
}

/// Drive the WebSocket connection: send status every 2 s, handle
/// incoming ping/close frames.
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
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {} // Ignore other messages
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
        let reg = registry.lock().ok()?;
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
        let mut manager = terminal_state.lock().ok()?;
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

// ── Middleware ────────────────────────────────────────────────────

/// Axum middleware that validates `Authorization: Bearer <token>`.
///
/// The following paths are exempt from authentication:
/// - `/api/health` -- always accessible for health checks
/// - `/ws/*` -- WebSocket endpoints handle their own auth via query params
/// - Any path that does **not** start with `/api/` or `/ws/` (static PWA files)
///
/// Token comparison uses constant-time equality to prevent timing attacks.
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path();

    // Skip auth for health endpoint
    if path == "/api/health" {
        return Ok(next.run(request).await);
    }

    // Skip auth for WebSocket paths (auth handled in handler via query param)
    if path.starts_with("/ws/") {
        return Ok(next.run(request).await);
    }

    // Skip auth for non-API paths (static files)
    if !path.starts_with("/api/") {
        return Ok(next.run(request).await);
    }

    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match auth_header {
        Some(auth) if auth.starts_with("Bearer ") => {
            let token = &auth[7..];
            let expected = state.auth_token.read().await;
            let token_bytes = token.as_bytes();
            let expected_bytes = expected.as_bytes();
            if token_bytes.len() == expected_bytes.len()
                && bool::from(token_bytes.ct_eq(expected_bytes))
            {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

// ── Router & Server ──────────────────────────────────────────────

/// Build the axum router with all API routes and auth middleware.
///
/// The `auth_token` is used by the bearer-token middleware to gate
/// access to protected endpoints. The `app_handle` provides access
/// to Tauri state (WindowRegistry, store, etc.) and is `None` during
/// integration tests that don't have a Tauri runtime.
pub fn create_router(
    auth_token: Arc<RwLock<String>>,
    app_handle: Option<tauri::AppHandle>,
) -> Router {
    let state = AppState {
        auth_token,
        app_handle: app_handle.clone(),
    };

    let router = Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/auth/verify", post(verify_handler))
        .route("/api/projects", get(list_projects))
        .route("/api/projects/open", post(open_project))
        .route("/api/projects/close", post(close_project))
        .route("/api/terminals", get(get_terminals))
        .route("/ws/status", get(ws_status))
        // Catch-all for unknown /api/ routes so they go through auth middleware
        .route("/api/{*rest}", any(|| async { StatusCode::NOT_FOUND }))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state);

    // Serve static PWA files as fallback for non-API/WS paths.
    // Unknown /api/* requests are caught by the catch-all route above
    // (and thus pass through auth middleware). The fallback only serves
    // static files for paths like /, /style.css, /app.js, etc.
    let ui_path = resolve_remote_ui_path(app_handle.as_ref());
    if ui_path.exists() {
        router.fallback_service(tower_http::services::ServeDir::new(ui_path))
    } else {
        log::warn!("Remote UI directory not found: {:?}", ui_path);
        router
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
    fn test_projects_response_serialization() {
        let response = ProjectsResponse {
            open_projects: vec![OpenProject {
                path: "/Users/user/projects/kiri".to_string(),
                name: "kiri".to_string(),
                branch: Some("main".to_string()),
            }],
            recent_projects: vec![RecentProject {
                path: "/Users/user/projects/old-project".to_string(),
                name: "old-project".to_string(),
                last_opened: 1700000000.0,
                git_branch: Some("develop".to_string()),
            }],
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["openProjects"][0]["path"], "/Users/user/projects/kiri");
        assert_eq!(json["openProjects"][0]["name"], "kiri");
        assert_eq!(json["openProjects"][0]["branch"], "main");
        assert_eq!(
            json["recentProjects"][0]["path"],
            "/Users/user/projects/old-project"
        );
        assert_eq!(json["recentProjects"][0]["lastOpened"], 1700000000.0);
        assert_eq!(json["recentProjects"][0]["gitBranch"], "develop");
    }

    #[test]
    fn test_projects_response_serialization_empty() {
        let response = ProjectsResponse {
            open_projects: vec![],
            recent_projects: vec![],
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json["openProjects"].as_array().unwrap().is_empty());
        assert!(json["recentProjects"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_open_project_request_deserialization() {
        let json = serde_json::json!({ "path": "/Users/user/projects/kiri" });
        let req: OpenProjectRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.path, "/Users/user/projects/kiri");
    }

    #[test]
    fn test_close_project_request_deserialization() {
        let json = serde_json::json!({ "path": "/Users/user/projects/kiri" });
        let req: CloseProjectRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.path, "/Users/user/projects/kiri");
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
    fn test_terminal_status_response_serialization() {
        let response = TerminalStatusResponse {
            terminals: vec![
                TerminalStatus {
                    id: 1,
                    is_alive: true,
                    process_name: Some("vim".to_string()),
                    cwd: Some("/home/user".to_string()),
                },
                TerminalStatus {
                    id: 2,
                    is_alive: false,
                    process_name: None,
                    cwd: None,
                },
            ],
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["terminals"].as_array().unwrap().len(), 2);
        assert_eq!(json["terminals"][0]["id"], 1);
        assert_eq!(json["terminals"][0]["isAlive"], true);
        assert_eq!(json["terminals"][0]["processName"], "vim");
        assert_eq!(json["terminals"][0]["cwd"], "/home/user");
        assert_eq!(json["terminals"][1]["id"], 2);
        assert_eq!(json["terminals"][1]["isAlive"], false);
        assert!(json["terminals"][1]["processName"].is_null());
        assert!(json["terminals"][1]["cwd"].is_null());
    }

    #[test]
    fn test_terminal_status_response_serialization_empty() {
        let response = TerminalStatusResponse {
            terminals: vec![],
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json["terminals"].as_array().unwrap().is_empty());
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
}
