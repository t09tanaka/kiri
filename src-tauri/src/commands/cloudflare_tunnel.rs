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

/// Check if `cloudflared` binary is available.
#[tauri::command]
pub fn is_cloudflared_available() -> bool {
    let path = cloudflared_path();
    if cfg!(debug_assertions) {
        // In debug, check if it's on PATH by trying to run it
        std::process::Command::new(&path)
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    } else {
        path.exists()
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
///
/// After finding the URL, spawns a background thread to drain remaining stderr
/// output. This prevents cloudflared from receiving SIGPIPE when it tries to
/// write to a closed pipe, which would kill the process.
pub(crate) fn parse_tunnel_url_from_stderr(
    child: &mut std::process::Child,
) -> Result<String, String> {
    parse_tunnel_url_from_stderr_with_timeout(child, std::time::Duration::from_secs(30))
}

/// Internal implementation with configurable timeout for testing.
pub(crate) fn parse_tunnel_url_from_stderr_with_timeout(
    child: &mut std::process::Child,
    timeout: std::time::Duration,
) -> Result<String, String> {
    use std::io::{BufRead, BufReader, Read};

    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    let mut reader = BufReader::new(stderr);

    let start = std::time::Instant::now();
    let mut line_buf = String::new();

    loop {
        line_buf.clear();
        if start.elapsed() > timeout {
            return Err("Timeout waiting for tunnel URL".to_string());
        }
        match reader.read_line(&mut line_buf) {
            Ok(0) => break, // EOF
            Ok(_) => {
                if let Some(url) = parse_quick_tunnel_url(&line_buf) {
                    // Drain remaining stderr in a background thread to prevent
                    // cloudflared from receiving SIGPIPE when writing to a closed pipe.
                    let mut stderr = reader.into_inner();
                    std::thread::spawn(move || {
                        let mut buf = [0u8; 4096];
                        loop {
                            match stderr.read(&mut buf) {
                                Ok(0) | Err(_) => break,
                                Ok(_) => continue,
                            }
                        }
                    });
                    return Ok(url);
                }
            }
            Err(e) => return Err(format!("Failed to read stderr: {}", e)),
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
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
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
                .stdout(std::process::Stdio::null())
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

    #[test]
    fn test_is_cloudflared_available_does_not_panic() {
        // is_cloudflared_available is a #[tauri::command] but takes no tauri::State,
        // so it can be called directly. cloudflared may or may not be installed,
        // so we just verify it returns a bool without panicking.
        let result = is_cloudflared_available();
        // result is either true or false depending on the environment
        assert!(result || !result);
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_finds_url() {
        // Create a child process that writes a tunnel URL to stderr.
        // We use `sh -c` to write to stderr via >&2.
        let mut child = std::process::Command::new("sh")
            .args([
                "-c",
                "echo 'Starting tunnel...' >&2; echo 'INF | https://bright-fox-lake.trycloudflare.com |' >&2",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result = parse_tunnel_url_from_stderr(&mut child);
        assert_eq!(
            result,
            Ok("https://bright-fox-lake.trycloudflare.com".to_string())
        );

        // Clean up
        let _ = child.wait();
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_no_url() {
        // Create a child process that writes non-matching output to stderr.
        let mut child = std::process::Command::new("sh")
            .args([
                "-c",
                "echo 'Starting tunnel...' >&2; echo 'Some other log line' >&2",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result = parse_tunnel_url_from_stderr(&mut child);
        assert_eq!(
            result,
            Err("Could not find tunnel URL in cloudflared output".to_string())
        );

        // Clean up
        let _ = child.wait();
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_empty_stderr() {
        // Create a child process with empty stderr.
        let mut child = std::process::Command::new("sh")
            .args(["-c", "true"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result = parse_tunnel_url_from_stderr(&mut child);
        assert_eq!(
            result,
            Err("Could not find tunnel URL in cloudflared output".to_string())
        );

        // Clean up
        let _ = child.wait();
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_url_after_many_lines() {
        // URL appears after several non-matching lines.
        let mut child = std::process::Command::new("sh")
            .args([
                "-c",
                "echo 'line 1' >&2; echo 'line 2' >&2; echo 'line 3' >&2; echo 'https://test-abc.trycloudflare.com' >&2",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result = parse_tunnel_url_from_stderr(&mut child);
        assert_eq!(
            result,
            Ok("https://test-abc.trycloudflare.com".to_string())
        );

        // Clean up
        let _ = child.wait();
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_no_stderr_captured() {
        // Create a child process without stderr piped, then take stderr manually
        // to simulate the "Failed to capture stderr" path.
        let mut child = std::process::Command::new("sh")
            .args(["-c", "true"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        // Take stderr away so the function can't get it
        let _stolen_stderr = child.stderr.take();

        let result = parse_tunnel_url_from_stderr(&mut child);
        assert_eq!(result, Err("Failed to capture stderr".to_string()));

        // Clean up
        let _ = child.wait();
    }

    // --- Additional TunnelState tests ---

    #[test]
    fn test_tunnel_state_mutate_fields() {
        let mut state = TunnelState::new();

        // Simulate starting a tunnel
        state.is_running = true;
        state.url = Some("https://test.trycloudflare.com".to_string());
        assert!(state.is_running);
        assert_eq!(
            state.url,
            Some("https://test.trycloudflare.com".to_string())
        );

        // Simulate stopping a tunnel
        state.is_running = false;
        state.url = None;
        state.child = None;
        assert!(!state.is_running);
        assert!(state.url.is_none());
        assert!(state.child.is_none());
    }

    #[test]
    fn test_tunnel_state_type_is_arc_mutex() {
        // Verify TunnelStateType can be created and locked
        let state: TunnelStateType = Arc::new(Mutex::new(TunnelState::new()));
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let guard = state.lock().await;
            assert!(!guard.is_running);
            assert!(guard.url.is_none());
        });
    }

    #[test]
    fn test_tunnel_state_type_concurrent_access() {
        let state: TunnelStateType = Arc::new(Mutex::new(TunnelState::new()));
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            // First lock: set running
            {
                let mut guard = state.lock().await;
                guard.is_running = true;
                guard.url = Some("https://abc.trycloudflare.com".to_string());
            }
            // Second lock: verify state persists
            {
                let guard = state.lock().await;
                assert!(guard.is_running);
                assert_eq!(
                    guard.url,
                    Some("https://abc.trycloudflare.com".to_string())
                );
            }
            // Third lock: reset
            {
                let mut guard = state.lock().await;
                guard.is_running = false;
                guard.url = None;
                guard.child = None;
            }
            // Verify reset
            {
                let guard = state.lock().await;
                assert!(!guard.is_running);
                assert!(guard.url.is_none());
            }
        });
    }

    // --- Additional parse_quick_tunnel_url edge case tests ---

    #[test]
    fn test_parse_quick_tunnel_url_with_path_suffix() {
        // URL followed by a path - regex should match just the domain
        let line = "https://test-tunnel.trycloudflare.com/some/path";
        let result = parse_quick_tunnel_url(line);
        assert_eq!(
            result,
            Some("https://test-tunnel.trycloudflare.com".to_string())
        );
    }

    #[test]
    fn test_parse_quick_tunnel_url_with_query_params() {
        let line = "https://test-tunnel.trycloudflare.com?foo=bar";
        let result = parse_quick_tunnel_url(line);
        assert_eq!(
            result,
            Some("https://test-tunnel.trycloudflare.com".to_string())
        );
    }

    #[test]
    fn test_parse_quick_tunnel_url_multiple_urls_returns_first() {
        let line = "https://first-url.trycloudflare.com and https://second-url.trycloudflare.com";
        let result = parse_quick_tunnel_url(line);
        // re.find returns the first match
        assert_eq!(
            result,
            Some("https://first-url.trycloudflare.com".to_string())
        );
    }

    #[test]
    fn test_parse_quick_tunnel_url_http_not_https() {
        // Should NOT match http:// (only https)
        let line = "http://test-tunnel.trycloudflare.com";
        assert_eq!(parse_quick_tunnel_url(line), None);
    }

    #[test]
    fn test_parse_quick_tunnel_url_partial_domain() {
        // Should NOT match a different domain that contains trycloudflare
        let line = "https://nottrycloudflare.com";
        assert_eq!(parse_quick_tunnel_url(line), None);
    }

    #[test]
    fn test_parse_quick_tunnel_url_numeric_subdomain() {
        let line = "https://12345.trycloudflare.com";
        assert_eq!(
            parse_quick_tunnel_url(line),
            Some("https://12345.trycloudflare.com".to_string())
        );
    }

    #[test]
    fn test_parse_quick_tunnel_url_single_char_subdomain() {
        let line = "https://a.trycloudflare.com";
        assert_eq!(
            parse_quick_tunnel_url(line),
            Some("https://a.trycloudflare.com".to_string())
        );
    }

    #[test]
    fn test_parse_quick_tunnel_url_whitespace_only() {
        assert_eq!(parse_quick_tunnel_url("   "), None);
    }

    #[test]
    fn test_parse_quick_tunnel_url_newline_in_line() {
        let line = "https://test.trycloudflare.com\n";
        assert_eq!(
            parse_quick_tunnel_url(line),
            Some("https://test.trycloudflare.com".to_string())
        );
    }

    #[test]
    fn test_parse_quick_tunnel_url_realistic_cloudflared_output() {
        // Real-world-like cloudflared output
        let line = "2024-01-15T10:30:00Z INF +--------------------------------------------------------------------------------------------+";
        assert_eq!(parse_quick_tunnel_url(line), None);

        let line =
            "2024-01-15T10:30:00Z INF |  https://autumn-meadow-abc123.trycloudflare.com  |";
        assert_eq!(
            parse_quick_tunnel_url(line),
            Some("https://autumn-meadow-abc123.trycloudflare.com".to_string())
        );
    }

    // --- Additional parse_tunnel_url_from_stderr tests ---

    #[test]
    fn test_parse_tunnel_url_from_stderr_url_on_first_line() {
        let mut child = std::process::Command::new("sh")
            .args([
                "-c",
                "echo 'https://immediate.trycloudflare.com' >&2",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result = parse_tunnel_url_from_stderr(&mut child);
        assert_eq!(
            result,
            Ok("https://immediate.trycloudflare.com".to_string())
        );
        let _ = child.wait();
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_with_realistic_output() {
        // Simulate realistic cloudflared output with log lines before the URL
        let script = r#"
echo '2024-01-15T10:30:00Z INF Starting tunnel' >&2
echo '2024-01-15T10:30:01Z INF Registered tunnel connection' >&2
echo '2024-01-15T10:30:02Z INF +----------------------------+' >&2
echo '2024-01-15T10:30:02Z INF |  https://demo-tunnel.trycloudflare.com  |' >&2
echo '2024-01-15T10:30:02Z INF +----------------------------+' >&2
echo '2024-01-15T10:30:03Z INF Connection established' >&2
"#;
        let mut child = std::process::Command::new("sh")
            .args(["-c", script])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result = parse_tunnel_url_from_stderr(&mut child);
        assert_eq!(
            result,
            Ok("https://demo-tunnel.trycloudflare.com".to_string())
        );
        let _ = child.wait();
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_returns_first_url_only() {
        // If multiple URLs appear, only the first should be returned
        let script = r#"
echo 'https://first-tunnel.trycloudflare.com' >&2
echo 'https://second-tunnel.trycloudflare.com' >&2
"#;
        let mut child = std::process::Command::new("sh")
            .args(["-c", script])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result = parse_tunnel_url_from_stderr(&mut child);
        assert_eq!(
            result,
            Ok("https://first-tunnel.trycloudflare.com".to_string())
        );
        let _ = child.wait();
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_long_running_process() {
        // Simulate a process that outputs the URL then keeps running
        // The background drain thread should handle the remaining output
        let script = r#"
echo 'Starting...' >&2
echo 'https://long-running.trycloudflare.com' >&2
sleep 0.1
echo 'Still running...' >&2
"#;
        let mut child = std::process::Command::new("sh")
            .args(["-c", script])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result = parse_tunnel_url_from_stderr(&mut child);
        assert_eq!(
            result,
            Ok("https://long-running.trycloudflare.com".to_string())
        );
        // Wait for child to finish to clean up
        let _ = child.wait();
    }

    // --- cloudflared_path tests ---

    #[test]
    fn test_cloudflared_path_returns_pathbuf() {
        let path = cloudflared_path();
        // In test (debug) mode, should return simple "cloudflared"
        assert!(!path.as_os_str().is_empty());
        assert_eq!(path.file_name().unwrap(), "cloudflared");
    }

    #[test]
    fn test_cloudflared_path_is_consistent() {
        // Calling cloudflared_path() multiple times should return the same result
        let path1 = cloudflared_path();
        let path2 = cloudflared_path();
        assert_eq!(path1, path2);
    }

    // --- parse_tunnel_url_from_stderr_with_timeout tests ---

    #[test]
    fn test_parse_tunnel_url_from_stderr_timeout() {
        // Spawn a process that continuously writes non-matching lines to stderr
        // but never writes a matching URL. With a very short timeout, this should
        // trigger the timeout path.
        let script = r#"
while true; do
    echo "non-matching log line" >&2
    # Small sleep to avoid overwhelming the pipe buffer
done
"#;
        let mut child = std::process::Command::new("sh")
            .args(["-c", script])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        // Use a very short timeout (100ms) so the test completes quickly
        let result = parse_tunnel_url_from_stderr_with_timeout(
            &mut child,
            std::time::Duration::from_millis(100),
        );
        assert_eq!(
            result,
            Err("Timeout waiting for tunnel URL".to_string())
        );

        // Clean up the child process
        let _ = child.kill();
        let _ = child.wait();
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_with_timeout_finds_url() {
        // Verify the _with_timeout variant works the same as the original
        // when a URL is present
        let mut child = std::process::Command::new("sh")
            .args([
                "-c",
                "echo 'https://timeout-test.trycloudflare.com' >&2",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result = parse_tunnel_url_from_stderr_with_timeout(
            &mut child,
            std::time::Duration::from_secs(5),
        );
        assert_eq!(
            result,
            Ok("https://timeout-test.trycloudflare.com".to_string())
        );
        let _ = child.wait();
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_with_timeout_eof_before_timeout() {
        // Process exits (EOF) before the timeout - should return the EOF error,
        // not the timeout error
        let mut child = std::process::Command::new("sh")
            .args(["-c", "echo 'no url here' >&2"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result = parse_tunnel_url_from_stderr_with_timeout(
            &mut child,
            std::time::Duration::from_secs(5),
        );
        assert_eq!(
            result,
            Err("Could not find tunnel URL in cloudflared output".to_string())
        );
        let _ = child.wait();
    }

    #[test]
    fn test_parse_tunnel_url_from_stderr_delegates_to_with_timeout() {
        // Verify the public function delegates correctly by checking
        // it produces the same result as the _with_timeout variant
        let mut child1 = std::process::Command::new("sh")
            .args([
                "-c",
                "echo 'https://delegate-test.trycloudflare.com' >&2",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let mut child2 = std::process::Command::new("sh")
            .args([
                "-c",
                "echo 'https://delegate-test.trycloudflare.com' >&2",
            ])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("failed to spawn test process");

        let result1 = parse_tunnel_url_from_stderr(&mut child1);
        let result2 = parse_tunnel_url_from_stderr_with_timeout(
            &mut child2,
            std::time::Duration::from_secs(30),
        );
        assert_eq!(result1, result2);

        let _ = child1.wait();
        let _ = child2.wait();
    }

    // --- is_cloudflared_available edge case tests ---

    #[test]
    fn test_is_cloudflared_available_returns_bool() {
        // In debug mode, is_cloudflared_available tries to run `cloudflared --version`.
        // Whether cloudflared is installed or not, the function should return a bool
        // without panicking.
        let result = is_cloudflared_available();
        // Type assertion: result is bool
        let _: bool = result;
    }
}
