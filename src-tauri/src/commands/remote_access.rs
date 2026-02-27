//! Core HTTP server logic for remote access
//!
//! Provides an embedded axum HTTP server that can be started and stopped
//! at runtime. The server exposes REST API endpoints for remote control
//! of the kiri application.

use axum::{routing::get, Json, Router};
use serde::Serialize;
use tokio::sync::oneshot;

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
pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Build the axum router with all API routes.
pub fn create_router() -> Router {
    Router::new().route("/api/health", get(health_handler))
}

/// Start the HTTP server on the given port.
///
/// The server runs until a signal is received on `shutdown_rx`,
/// at which point it performs a graceful shutdown.
///
/// # Errors
///
/// Returns an error if the server fails to bind to the address
/// or encounters a runtime error.
pub async fn start_server(
    port: u16,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router();
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    log::info!("Remote access server listening on {}", addr);

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
        let _router = create_router();
    }
}
