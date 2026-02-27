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
use serde::Serialize;
use tokio::sync::oneshot;

/// Shared application state passed to handlers and middleware.
#[derive(Clone, Debug)]
pub struct AppState {
    /// The bearer token required to access protected endpoints.
    pub auth_token: String,
}

/// Response payload for the health check endpoint.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

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

/// Axum middleware that validates `Authorization: Bearer <token>`.
///
/// The following paths are exempt from authentication:
/// - `/api/health` — always accessible for health checks
/// - Any path that does **not** start with `/api/` or `/ws/` (static PWA files)
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
            if token == state.auth_token {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Build the axum router with all API routes and auth middleware.
///
/// The `auth_token` is used by the bearer-token middleware to gate
/// access to protected endpoints.
pub fn create_router(auth_token: String) -> Router {
    let state = AppState { auth_token };

    Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/auth/verify", post(verify_handler))
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
    auth_token: String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router(auth_token);
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
        let _router = create_router("test-token".to_string());
    }

    #[test]
    fn test_app_state_clone() {
        let state = AppState {
            auth_token: "abc-123".to_string(),
        };
        let cloned = state.clone();
        assert_eq!(cloned.auth_token, "abc-123");
    }
}
