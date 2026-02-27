//! Core HTTP server logic for remote access
//!
//! Provides an embedded axum HTTP server that can be started and stopped
//! at runtime. The server exposes REST API endpoints for remote control
//! of the kiri application.
//!
//! All `/api/*` and `/ws/*` routes (except `/api/health`) are protected
//! by bearer-token authentication.

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
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
pub struct ProjectsResponse {
    pub open_projects: Vec<OpenProject>,
    pub recent_projects: Vec<RecentProject>,
}

/// A currently open project with an active window.
#[derive(Debug, Clone, Serialize)]
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

    Ok(Json(
        serde_json::json!({ "success": true, "path": req.path }),
    ))
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

// ── Middleware ────────────────────────────────────────────────────

/// Axum middleware that validates `Authorization: Bearer <token>`.
///
/// The following paths are exempt from authentication:
/// - `/api/health` -- always accessible for health checks
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

    // Skip auth for non-API / non-WS paths (static files)
    if !path.starts_with("/api/") && !path.starts_with("/ws/") {
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
        app_handle,
    };

    Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/auth/verify", post(verify_handler))
        .route("/api/projects", get(list_projects))
        .route("/api/projects/open", post(open_project))
        .route("/api/projects/close", post(close_project))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
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
        assert_eq!(json["open_projects"][0]["path"], "/Users/user/projects/kiri");
        assert_eq!(json["open_projects"][0]["name"], "kiri");
        assert_eq!(json["open_projects"][0]["branch"], "main");
        assert_eq!(
            json["recent_projects"][0]["path"],
            "/Users/user/projects/old-project"
        );
        assert_eq!(json["recent_projects"][0]["lastOpened"], 1700000000.0);
        assert_eq!(json["recent_projects"][0]["gitBranch"], "develop");
    }

    #[test]
    fn test_projects_response_serialization_empty() {
        let response = ProjectsResponse {
            open_projects: vec![],
            recent_projects: vec![],
        };
        let json = serde_json::to_value(&response).unwrap();
        assert!(json["open_projects"].as_array().unwrap().is_empty());
        assert!(json["recent_projects"].as_array().unwrap().is_empty());
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
}
