//! Tauri command wrappers for remote access server
//!
//! These commands allow starting, stopping, and querying the state
//! of the embedded HTTP server from the frontend. Includes token
//! management and QR code generation for mobile pairing.

use base64::Engine;
use image::ImageEncoder;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex, RwLock};

/// Holds the runtime state of the remote access server.
pub struct RemoteServerState {
    /// Sender half of the shutdown channel. Sending a value triggers graceful shutdown.
    shutdown_tx: Option<oneshot::Sender<()>>,
    /// Handle to the spawned server task, used for health checking.
    server_handle: Option<tokio::task::JoinHandle<()>>,
    /// The port the server is listening on.
    port: u16,
    /// Whether the server is currently running.
    is_running: bool,
    /// Bearer token used to authenticate remote API requests.
    auth_token: Option<String>,
    /// Shared reference to the live token used by the running server.
    /// Updating this value immediately affects the running server's auth.
    live_token: Option<Arc<RwLock<String>>>,
}

impl RemoteServerState {
    pub fn new() -> Self {
        Self {
            shutdown_tx: None,
            server_handle: None,
            port: 9876,
            is_running: false,
            auth_token: None,
            live_token: None,
        }
    }
}

impl Default for RemoteServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for the shared remote server state, managed by Tauri.
pub type RemoteServerStateType = Arc<Mutex<RemoteServerState>>;

/// Start the remote access HTTP server on the specified port.
///
/// A bearer token is generated automatically if one does not already
/// exist.  The token is passed to the axum router so that protected
/// endpoints can validate incoming requests.
///
/// The listener is bound eagerly so that port-conflict errors are
/// reported to the caller instead of being silently swallowed inside
/// the spawned task.
///
/// Returns an error if the server is already running or if the port
/// cannot be bound.
#[tauri::command]
pub async fn start_remote_server(
    app: tauri::AppHandle,
    state: tauri::State<'_, RemoteServerStateType>,
    port: u16,
) -> Result<String, String> {
    let mut server = state.lock().await;
    if server.is_running {
        return Err("Server is already running".to_string());
    }

    // Ensure we have a token before starting
    if server.auth_token.is_none() {
        server.auth_token = Some(uuid::Uuid::new_v4().to_string());
    }
    let token = server.auth_token.clone().unwrap();
    let live_token = Arc::new(RwLock::new(token));
    server.live_token = Some(live_token.clone());

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| format!("Failed to bind to port {}: {}", port, e))?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let app_clone = app.clone();
    let handle = tokio::spawn(async move {
        if let Err(e) = super::remote_access::start_server(
            listener,
            shutdown_rx,
            live_token,
            Some(app_clone),
        )
        .await
        {
            log::error!("Remote server error: {}", e);
        }
    });

    let token = server.auth_token.clone().unwrap();
    server.shutdown_tx = Some(shutdown_tx);
    server.server_handle = Some(handle);
    server.port = port;
    server.is_running = true;
    Ok(token)
}

/// Stop the remote access HTTP server gracefully.
///
/// If the server is not running, this is a no-op.
#[tauri::command]
pub async fn stop_remote_server(
    state: tauri::State<'_, RemoteServerStateType>,
) -> Result<(), String> {
    let mut server = state.lock().await;
    if !server.is_running {
        return Ok(());
    }

    if let Some(tx) = server.shutdown_tx.take() {
        let _ = tx.send(());
    }
    server.server_handle = None;
    server.is_running = false;
    server.live_token = None;
    Ok(())
}

/// Check whether the remote access server is currently running.
///
/// If the server task has finished unexpectedly (e.g. due to a panic
/// or runtime error), the state is cleaned up and `false` is returned.
#[tauri::command]
pub async fn is_remote_server_running(
    state: tauri::State<'_, RemoteServerStateType>,
) -> Result<bool, String> {
    let mut server = state.lock().await;
    if server.is_running {
        if let Some(ref handle) = server.server_handle {
            if handle.is_finished() {
                server.is_running = false;
                server.shutdown_tx = None;
                server.server_handle = None;
                server.live_token = None;
                return Ok(false);
            }
        }
    }
    Ok(server.is_running)
}

/// Generate a QR code PNG (as a base64 data-URI) that encodes the
/// full URL with the authentication token embedded in the path.
///
/// The encoded URL has the form `{base_url}/{token}/` where:
/// - If `tunnel_url` is provided, it is used as the base URL.
/// - Otherwise, the LAN IP is auto-detected: `http://{local_ip}:{port}`.
///
/// A new token is generated if none exists yet.
///
/// The returned string is a complete `data:image/png;base64,...` URI
/// suitable for display in an `<img>` element.
#[tauri::command]
pub async fn generate_remote_qr_code(
    state: tauri::State<'_, RemoteServerStateType>,
    port: u16,
    tunnel_url: Option<String>,
) -> Result<String, String> {
    let mut server = state.lock().await;

    // Ensure we have a token
    if server.auth_token.is_none() {
        server.auth_token = Some(uuid::Uuid::new_v4().to_string());
    }

    let token = server.auth_token.as_ref().unwrap();

    let base_url = match tunnel_url {
        Some(url) => url.trim_end_matches('/').to_string(),
        None => {
            let host = local_ip_address::local_ip()
                .map(|ip| ip.to_string())
                .unwrap_or_else(|_| "localhost".to_string());
            format!("http://{}:{}", host, port)
        }
    };

    let full_url = format!("{}/{}/", base_url, token);
    generate_qr_base64(&full_url)
}

/// Replace the current auth token with a freshly generated UUID v4.
///
/// Returns the new token string.
///
/// If the server is currently running, the live token is updated
/// immediately so the new token takes effect without restart.
#[tauri::command]
pub async fn regenerate_remote_token(
    state: tauri::State<'_, RemoteServerStateType>,
) -> Result<String, String> {
    let mut server = state.lock().await;
    let new_token = uuid::Uuid::new_v4().to_string();
    server.auth_token = Some(new_token.clone());

    // Update live server token immediately
    if let Some(ref live_token) = server.live_token {
        let mut t = live_token.write().await;
        *t = new_token.clone();
    }

    Ok(new_token)
}

/// Encode `data` into a QR code and return it as a base64 data-URI PNG.
fn generate_qr_base64(data: &str) -> Result<String, String> {
    let code = qrcode::QrCode::new(data).map_err(|e| e.to_string())?;
    let image = code.render::<image::Luma<u8>>().build();

    let mut png_bytes = Vec::new();
    image::codecs::png::PngEncoder::new(&mut png_bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::L8,
        )
        .map_err(|e| e.to_string())?;

    let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
    Ok(format!("data:image/png;base64,{}", b64))
}

// ── Unit tests ───────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remote_server_state_default() {
        let state = RemoteServerState::new();
        assert!(!state.is_running);
        assert!(state.auth_token.is_none());
        assert!(state.live_token.is_none());
        assert_eq!(state.port, 9876);
    }

    #[test]
    fn test_generate_qr_base64_produces_data_uri() {
        let result = generate_qr_base64("hello").unwrap();
        assert!(result.starts_with("data:image/png;base64,"));
        // Verify the base64 part is valid
        let b64_part = result.strip_prefix("data:image/png;base64,").unwrap();
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64_part)
            .unwrap();
        // PNG magic bytes
        assert_eq!(&decoded[..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_generate_qr_base64_encodes_url() {
        let url = "http://192.168.1.5:9876/abc-123-token/";
        let result = generate_qr_base64(url);
        assert!(result.is_ok());
    }
}
