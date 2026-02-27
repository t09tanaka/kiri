//! Cloudflare Tunnel management for remote access.
//!
//! Manages a `cloudflared` child process that creates a tunnel
//! to expose the embedded HTTP server to the internet.
//!
//! Supports two modes:
//! - **Named Tunnel**: Uses a pre-configured tunnel token (`cloudflared tunnel run --token <token>`)
//! - **Quick Tunnel**: Creates a temporary tunnel with a random URL (`cloudflared tunnel --url http://localhost:<port>`)

use std::sync::Arc;
use tokio::sync::Mutex;

/// State for the Cloudflare Tunnel child process.
pub struct TunnelState {
    child: Option<std::process::Child>,
    pub is_running: bool,
    pub url: Option<String>,
}

impl TunnelState {
    pub fn new() -> Self {
        Self {
            child: None,
            is_running: false,
            url: None,
        }
    }
}

impl Default for TunnelState {
    fn default() -> Self {
        Self::new()
    }
}

/// Type alias for managed tunnel state.
pub type TunnelStateType = Arc<Mutex<TunnelState>>;

/// Get the path to the cloudflared binary.
///
/// In debug builds, assumes `cloudflared` is on the system PATH.
/// In release builds, looks for the binary bundled alongside the app executable.
pub fn cloudflared_path() -> std::path::PathBuf {
    if cfg!(debug_assertions) {
        std::path::PathBuf::from("cloudflared")
    } else {
        std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .map(|p| p.join("cloudflared"))
            .unwrap_or_else(|| std::path::PathBuf::from("cloudflared"))
    }
}

/// Parse the Quick Tunnel URL from cloudflared's stderr output.
///
/// cloudflared prints lines like:
/// `... | https://random-words.trycloudflare.com |`
pub fn parse_quick_tunnel_url(line: &str) -> Option<String> {
    use std::sync::OnceLock;
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"https://[a-zA-Z0-9-]+\.trycloudflare\.com\b")
            .expect("hardcoded regex must be valid")
    });
    re.find(line).map(|m| m.as_str().to_string())
}

/// Read stderr from a child process line by line and extract the Quick Tunnel URL.
///
/// Waits up to 30 seconds for the URL to appear. Returns an error if the URL
/// is not found within the timeout or if stderr cannot be read.
fn parse_tunnel_url_from_stderr(child: &mut std::process::Child) -> Result<String, String> {
    use std::io::{BufRead, BufReader};

    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let reader = BufReader::new(stderr);

    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(30);

    for line in reader.lines() {
        if start.elapsed() > timeout {
            return Err("Timeout waiting for tunnel URL".to_string());
        }
        let line = line.map_err(|e| format!("Failed to read stderr: {}", e))?;
        if let Some(url) = parse_quick_tunnel_url(&line) {
            return Ok(url);
        }
    }

    Err("Could not find tunnel URL in cloudflared output".to_string())
}

/// Start the Cloudflare Tunnel.
///
/// - If `token` is `Some`, starts a Named Tunnel: `cloudflared tunnel run --token <token>`
/// - If `token` is `None`, starts a Quick Tunnel: `cloudflared tunnel --url http://localhost:<port>`
///
/// Returns the Quick Tunnel URL when in Quick Tunnel mode, or `None` for Named Tunnel mode.
#[tauri::command]
pub async fn start_cloudflare_tunnel(
    state: tauri::State<'_, TunnelStateType>,
    token: Option<String>,
    port: u16,
) -> Result<Option<String>, String> {
    let mut tunnel = state.lock().await;
    if tunnel.is_running {
        return Err("Tunnel is already running".to_string());
    }

    match token {
        Some(ref t) => {
            // Named Tunnel mode
            let child = std::process::Command::new(cloudflared_path())
                .args(["tunnel", "run", "--token", t])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start cloudflared: {}", e))?;

            tunnel.child = Some(child);
            tunnel.is_running = true;
            tunnel.url = None;
            log::info!("Cloudflare Named Tunnel started");
            Ok(None)
        }
        None => {
            // Quick Tunnel mode
            let local_url = format!("http://localhost:{}", port);
            let mut child = std::process::Command::new(cloudflared_path())
                .args(["tunnel", "--url", &local_url])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start cloudflared: {}", e))?;

            // Parse the tunnel URL from stderr in a blocking task
            // to avoid blocking the Tokio async runtime.
            let (child, url) = tokio::task::spawn_blocking(move || {
                match parse_tunnel_url_from_stderr(&mut child) {
                    Ok(url) => Ok((child, url)),
                    Err(e) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        Err(e)
                    }
                }
            })
            .await
            .map_err(|e| format!("Task join error: {}", e))??;

            tunnel.child = Some(child);
            tunnel.is_running = true;
            tunnel.url = Some(url.clone());
            log::info!("Cloudflare Quick Tunnel started: {}", url);
            Ok(Some(url))
        }
    }
}

/// Stop the running Cloudflare Tunnel.
#[tauri::command]
pub async fn stop_cloudflare_tunnel(
    state: tauri::State<'_, TunnelStateType>,
) -> Result<(), String> {
    let mut tunnel = state.lock().await;
    if let Some(ref mut child) = tunnel.child {
        child
            .kill()
            .map_err(|e| format!("Failed to stop cloudflared: {}", e))?;
        child.wait().ok();
    }
    tunnel.child = None;
    tunnel.is_running = false;
    tunnel.url = None;
    log::info!("Cloudflare Tunnel stopped");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cloudflared_path_in_debug() {
        let path = cloudflared_path();
        // In debug mode, should just be "cloudflared" (on PATH)
        assert_eq!(path, std::path::PathBuf::from("cloudflared"));
    }

    #[test]
    fn test_tunnel_state_new() {
        let state = TunnelState::new();
        assert!(!state.is_running);
        assert!(state.child.is_none());
        assert!(state.url.is_none());
    }

    #[test]
    fn test_tunnel_state_default() {
        let state = TunnelState::default();
        assert!(!state.is_running);
        assert!(state.child.is_none());
        assert!(state.url.is_none());
    }

    #[test]
    fn test_parse_quick_tunnel_url_valid() {
        let line = "2024-01-15 | https://random-words-here.trycloudflare.com |";
        assert_eq!(
            parse_quick_tunnel_url(line),
            Some("https://random-words-here.trycloudflare.com".to_string())
        );
    }

    #[test]
    fn test_parse_quick_tunnel_url_no_match() {
        let line = "Starting tunnel...";
        assert_eq!(parse_quick_tunnel_url(line), None);
    }

    #[test]
    fn test_parse_quick_tunnel_url_multipart() {
        let line = "INF |  https://bright-fox-lake.trycloudflare.com  |";
        assert_eq!(
            parse_quick_tunnel_url(line),
            Some("https://bright-fox-lake.trycloudflare.com".to_string())
        );
    }

    #[test]
    fn test_parse_quick_tunnel_url_with_long_subdomain() {
        let line = "https://my-super-long-random-subdomain-123.trycloudflare.com";
        assert_eq!(
            parse_quick_tunnel_url(line),
            Some("https://my-super-long-random-subdomain-123.trycloudflare.com".to_string())
        );
    }

    #[test]
    fn test_parse_quick_tunnel_url_ignores_non_trycloudflare() {
        let line = "https://example.com";
        assert_eq!(parse_quick_tunnel_url(line), None);
    }

    #[test]
    fn test_parse_quick_tunnel_url_empty_string() {
        assert_eq!(parse_quick_tunnel_url(""), None);
    }
}
