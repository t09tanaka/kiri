//! Cloudflare Tunnel management for remote access.
//!
//! Manages a `cloudflared` child process that creates a tunnel
//! to expose the embedded HTTP server to the internet.

use std::sync::Arc;
use tokio::sync::Mutex;

/// State for the Cloudflare Tunnel child process.
pub struct TunnelState {
    child: Option<std::process::Child>,
    pub is_running: bool,
}

impl TunnelState {
    pub fn new() -> Self {
        Self {
            child: None,
            is_running: false,
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

/// Start the Cloudflare Tunnel with the given token.
#[tauri::command]
pub async fn start_cloudflare_tunnel(
    state: tauri::State<'_, TunnelStateType>,
    token: String,
) -> Result<(), String> {
    let mut tunnel = state.lock().await;
    if tunnel.is_running {
        return Err("Tunnel is already running".to_string());
    }

    let child = std::process::Command::new(cloudflared_path())
        .args(["tunnel", "run", "--token", &token])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start cloudflared: {}", e))?;

    tunnel.child = Some(child);
    tunnel.is_running = true;
    log::info!("Cloudflare Tunnel started");
    Ok(())
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
    }

    #[test]
    fn test_tunnel_state_default() {
        let state = TunnelState::default();
        assert!(!state.is_running);
        assert!(state.child.is_none());
    }
}
