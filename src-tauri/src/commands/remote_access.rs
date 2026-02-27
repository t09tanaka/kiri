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
use std::sync::Arc;
use subtle::ConstantTimeEq;
use tokio::sync::{oneshot, RwLock};

/// Shared application state passed to handlers and middleware.
#[derive(Clone)]
pub struct AppState {
    /// The bearer token required to access protected endpoints.
    pub(crate) auth_token: Arc<RwLock<String>>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("auth_token", &"[REDACTED]")
            .finish()
    }
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

/// Build the axum router with all API routes and auth middleware.
///
/// The `auth_token` is used by the bearer-token middleware to gate
/// access to protected endpoints.
pub fn create_router(auth_token: Arc<RwLock<String>>) -> Router {
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
    auth_token: Arc<RwLock<String>>,
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
        let token = Arc::new(RwLock::new("test-token".to_string()));
        let _router = create_router(token);
    }

    #[test]
    fn test_app_state_clone() {
        let state = AppState {
            auth_token: Arc::new(RwLock::new("abc-123".to_string())),
        };
        let cloned = state.clone();
        // Both point to the same Arc
        assert!(Arc::ptr_eq(&state.auth_token, &cloned.auth_token));
    }

    #[test]
    fn test_app_state_debug_redacts_token() {
        let state = AppState {
            auth_token: Arc::new(RwLock::new("secret-token".to_string())),
        };
        let debug_output = format!("{:?}", state);
        assert!(debug_output.contains("[REDACTED]"));
        assert!(!debug_output.contains("secret-token"));
    }
}
