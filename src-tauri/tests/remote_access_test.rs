//! Integration tests for the remote access HTTP server
//!
//! These tests verify that the embedded axum server starts correctly,
//! responds to health checks, enforces path-prefix token authentication,
//! and shuts down gracefully.

use std::sync::Arc;
use tokio::sync::RwLock;

const TEST_TOKEN: &str = "test-token-abc-123";

/// Helper: start a server on an ephemeral port and return (port, shutdown_tx, handle).
async fn start_test_server() -> (
    u16,
    tokio::sync::oneshot::Sender<()>,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let token = Arc::new(RwLock::new(TEST_TOKEN.to_string()));
    let handle = tokio::spawn(async move {
        app_lib::commands::remote_access::start_server(listener, shutdown_rx, token, None)
            .await
            .unwrap();
    });

    // Wait for the server to be ready
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    (port, shutdown_tx, handle)
}

// ── Health endpoint (no auth required) ───────────────────────────

#[tokio::test]
async fn test_remote_server_starts_and_responds() {
    let (port, shutdown_tx, _handle) = start_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{}/api/health", port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));

    shutdown_tx.send(()).unwrap();
}

#[tokio::test]
async fn test_remote_server_graceful_shutdown() {
    let (port, shutdown_tx, server_handle) = start_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{}/api/health", port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Send shutdown signal
    shutdown_tx.send(()).unwrap();

    // Server task should complete without error
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), server_handle).await;
    assert!(result.is_ok(), "Server did not shut down within timeout");
    assert!(
        result.unwrap().is_ok(),
        "Server task panicked during shutdown"
    );
}

#[tokio::test]
async fn test_health_endpoint_returns_correct_fields() {
    let (port, shutdown_tx, _handle) = start_test_server().await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{}/api/health", port))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();

    assert!(body.get("status").is_some(), "Missing 'status' field");
    assert!(body.get("version").is_some(), "Missing 'version' field");
    assert!(body["status"].is_string());
    assert!(body["version"].is_string());

    shutdown_tx.send(()).unwrap();
}

#[tokio::test]
async fn test_health_endpoint_requires_no_auth() {
    let (port, shutdown_tx, _handle) = start_test_server().await;

    // Request WITHOUT token prefix should succeed for /api/health
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{}/api/health", port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    shutdown_tx.send(()).unwrap();
}

// ── Path-prefix token auth ──────────────────────────────────────

#[tokio::test]
async fn test_invalid_token_returns_404() {
    let (port, shutdown_tx, _handle) = start_test_server().await;

    let client = reqwest::Client::new();

    // Request with invalid token prefix returns 404
    let resp = client
        .get(format!("http://127.0.0.1:{}/wrong-token/ws", port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    shutdown_tx.send(()).unwrap();
}

#[tokio::test]
async fn test_missing_token_returns_404() {
    let (port, shutdown_tx, _handle) = start_test_server().await;

    let client = reqwest::Client::new();

    // Request without any token prefix returns 404
    let resp = client
        .get(format!("http://127.0.0.1:{}/ws", port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    shutdown_tx.send(()).unwrap();
}

#[tokio::test]
async fn test_valid_token_prefix_accepted() {
    let (port, shutdown_tx, _handle) = start_test_server().await;

    let client = reqwest::Client::new();

    // With valid token prefix, health endpoint is also accessible via token path
    // (the middleware strips the token and the request reaches /api/health)
    let resp = client
        .get(format!(
            "http://127.0.0.1:{}/{}/api/health",
            port, TEST_TOKEN
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    shutdown_tx.send(()).unwrap();
}

// ── Live token update ────────────────────────────────────────────

#[tokio::test]
async fn test_live_token_update_takes_effect_immediately() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let live_token = Arc::new(RwLock::new(TEST_TOKEN.to_string()));
    let live_token_clone = live_token.clone();

    let _handle = tokio::spawn(async move {
        app_lib::commands::remote_access::start_server(listener, shutdown_rx, live_token_clone, None)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let client = reqwest::Client::new();

    // Old token works (200 from /api/health means token was accepted)
    let resp = client
        .get(format!(
            "http://127.0.0.1:{}/{}/api/health",
            port, TEST_TOKEN
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    // Update the live token
    let new_token = "new-token-xyz-789";
    {
        let mut t = live_token.write().await;
        *t = new_token.to_string();
    }

    // Old token no longer works
    let resp = client
        .get(format!(
            "http://127.0.0.1:{}/{}/api/health",
            port, TEST_TOKEN
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // New token works
    let resp = client
        .get(format!(
            "http://127.0.0.1:{}/{}/api/health",
            port, new_token
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);

    shutdown_tx.send(()).unwrap();
}
