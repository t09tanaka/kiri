//! Integration tests for the remote access HTTP server
//!
//! These tests verify that the embedded axum server starts correctly,
//! responds to health checks, and shuts down gracefully.

#[tokio::test]
async fn test_remote_server_starts_and_responds() {
    // Find an available port by binding to port 0
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        app_lib::commands::remote_access::start_server(port, shutdown_rx)
            .await
            .unwrap();
    });

    // Wait for the server to be ready
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

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

    // Shut down the server gracefully
    shutdown_tx.send(()).unwrap();
    let _ = server_handle.await;
}

#[tokio::test]
async fn test_remote_server_graceful_shutdown() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        app_lib::commands::remote_access::start_server(port, shutdown_rx)
            .await
            .unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify server is running
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
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        let _ = app_lib::commands::remote_access::start_server(port, shutdown_rx).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{}/api/health", port))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();

    // Verify all expected fields exist
    assert!(body.get("status").is_some(), "Missing 'status' field");
    assert!(body.get("version").is_some(), "Missing 'version' field");

    // Verify field types
    assert!(body["status"].is_string());
    assert!(body["version"].is_string());

    shutdown_tx.send(()).unwrap();
}

#[tokio::test]
async fn test_unknown_route_returns_404() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tokio::spawn(async move {
        let _ = app_lib::commands::remote_access::start_server(port, shutdown_rx).await;
    });

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{}/api/nonexistent", port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    shutdown_tx.send(()).unwrap();
}
