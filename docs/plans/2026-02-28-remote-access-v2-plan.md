# Remote Access v2 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Simplify remote access to URL-based auth, WebSocket-only communication, Cloudflare Quick/Named Tunnel modes, and a start screen shortcut.

**Architecture:** Replace Bearer-token + REST API with path-prefix token (`/{token}/`) and a single bidirectional WebSocket. Add Cloudflare Quick Tunnel mode alongside existing Named Tunnel. Add Remote Access toggle on StartScreen.

**Tech Stack:** Rust (axum), Svelte 5, TypeScript, vanilla JS (PWA), Cloudflare Tunnel (`cloudflared`)

**Design doc:** `docs/plans/2026-02-28-remote-access-v2-design.md`

---

## Task 1: Backend — Path-prefix token middleware + router refactor

Remove Bearer-token auth, REST API endpoints, and WebSocket query-param auth. Replace with path-prefix token validation (`/{token}/`).

**Files:**
- Modify: `src-tauri/src/commands/remote_access.rs`

**Step 1: Write tests for the new path-prefix token validation**

Add tests to the existing `#[cfg(test)] mod tests` block in `remote_access.rs`:

```rust
#[test]
fn test_strip_token_prefix_valid() {
    let token = "abc-123";
    let path = "/abc-123/ws";
    assert_eq!(strip_token_prefix(path, token), Some("/ws"));
}

#[test]
fn test_strip_token_prefix_root() {
    let token = "abc-123";
    let path = "/abc-123/";
    assert_eq!(strip_token_prefix(path, token), Some("/"));
}

#[test]
fn test_strip_token_prefix_without_trailing_slash() {
    let token = "abc-123";
    let path = "/abc-123";
    assert_eq!(strip_token_prefix(path, token), Some("/"));
}

#[test]
fn test_strip_token_prefix_invalid_token() {
    let token = "abc-123";
    let path = "/wrong-token/ws";
    assert_eq!(strip_token_prefix(path, token), None);
}

#[test]
fn test_strip_token_prefix_health_bypass() {
    let token = "abc-123";
    let path = "/api/health";
    // Health check should NOT go through token validation
    assert_eq!(strip_token_prefix(path, token), None);
}

#[test]
fn test_strip_token_prefix_static_file() {
    let token = "abc-123";
    let path = "/abc-123/app.js";
    assert_eq!(strip_token_prefix(path, token), Some("/app.js"));
}
```

**Step 2: Run tests to verify they fail**

Run: `cd src-tauri && cargo test --lib commands::remote_access::tests -- strip_token_prefix`
Expected: FAIL — `strip_token_prefix` function does not exist.

**Step 3: Implement `strip_token_prefix` and refactor the router**

Replace the entire `create_router` function and related code in `remote_access.rs`:

1. **Add `strip_token_prefix` function:**

```rust
/// Strip the `/{token}/` prefix from the request path.
/// Returns `Some(remaining_path)` if the token matches, `None` otherwise.
pub fn strip_token_prefix<'a>(path: &'a str, token: &str) -> Option<&'a str> {
    let prefix = format!("/{}", token);
    if let Some(rest) = path.strip_prefix(&prefix) {
        if rest.is_empty() {
            Some("/")
        } else if rest.starts_with('/') {
            Some(rest)
        } else {
            None
        }
    } else {
        None
    }
}
```

2. **Replace `auth_middleware` with `token_path_middleware`:**

```rust
/// Axum middleware that validates the `/{token}/` path prefix.
///
/// - `/api/health` bypasses token validation.
/// - All other paths must start with `/{token}/`.
/// - The token prefix is stripped before passing to downstream handlers.
pub async fn token_path_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string();

    // Health check bypasses token validation
    if path == "/api/health" {
        return Ok(next.run(request).await);
    }

    let token = state.auth_token.read().await;
    match strip_token_prefix(&path, &token) {
        Some(new_path) => {
            drop(token);
            // Rewrite the URI with the token prefix stripped
            let mut parts = request.uri().clone().into_parts();
            let query = parts
                .path_and_query
                .as_ref()
                .and_then(|pq| pq.query())
                .map(|q| format!("{}?{}", new_path, q))
                .unwrap_or_else(|| new_path.to_string());
            parts.path_and_query = Some(query.parse().unwrap());
            *request.uri_mut() = axum::http::Uri::from_parts(parts).unwrap();
            Ok(next.run(request).await)
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}
```

3. **Replace `create_router` — remove all REST API endpoints:**

```rust
pub fn create_router(
    auth_token: Arc<RwLock<String>>,
    app_handle: Option<tauri::AppHandle>,
) -> Router {
    let state = AppState {
        auth_token,
        app_handle: app_handle.clone(),
    };

    let router = Router::new()
        .route("/api/health", get(health_handler))
        .route("/ws", get(ws_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            token_path_middleware,
        ))
        .with_state(state);

    // Serve static PWA files as fallback
    let ui_path = resolve_remote_ui_path(app_handle.as_ref());
    if ui_path.exists() {
        router.fallback_service(tower_http::services::ServeDir::new(ui_path))
    } else {
        log::warn!("Remote UI directory not found: {:?}", ui_path);
        router
    }
}
```

4. **Remove the following functions and types** (no longer needed):
   - `verify_handler`
   - `list_projects`
   - `load_recent_projects` (will be kept — used by WebSocket handler)
   - `open_project` handler
   - `close_project` handler
   - `get_terminals` handler
   - `auth_middleware`
   - `OpenProjectRequest`
   - `CloseProjectRequest`
   - `ProjectsResponse`
   - `TerminalStatusResponse`

5. **Keep these types** (used by WebSocket status push):
   - `HealthResponse`
   - `OpenProject`
   - `RecentProject`
   - `TerminalStatus`
   - `StatusUpdate`
   - `AppState`

6. **Rename `ws_status` to `ws_handler` and remove query-param auth** (auth is now handled by path middleware):

```rust
/// Handler for `GET /{token}/ws`.
///
/// Upgrades to a bidirectional WebSocket connection.
/// Authentication is handled by the path-prefix middleware.
pub async fn ws_handler(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_status_ws(socket, state))
}
```

7. **Remove unused imports:**
   - `Query`, `header` from axum
   - `subtle::ConstantTimeEq`
   - `HashMap`

**Step 4: Run all tests to verify they pass**

Run: `cd src-tauri && cargo test --lib commands::remote_access`
Expected: All tests pass. Some old tests for removed types will need to be deleted too.

**Step 5: Clean up removed-code tests**

Remove tests that reference deleted functions/types:
- `test_projects_response_serialization`
- `test_projects_response_serialization_empty`
- `test_open_project_request_deserialization`
- `test_close_project_request_deserialization`
- `test_terminal_status_response_serialization`
- `test_terminal_status_response_serialization_empty`

Keep:
- `test_health_response_serialization`
- `test_create_router_builds`
- `test_app_state_clone`
- `test_app_state_debug_redacts_token`
- `test_status_update_serialization`
- `test_status_update_serialization_empty`
- `test_terminal_status_clone`
- `test_collect_full_status_returns_none_without_app_handle`
- `test_resolve_remote_ui_path_without_app_handle`
- All new `strip_token_prefix` tests
- `test_recent_project_serde_*` (RecentProject is still used)

**Step 6: Run tests**

Run: `cd src-tauri && cargo test --lib commands::remote_access`
Expected: PASS

**Step 7: Commit**

```bash
git add src-tauri/src/commands/remote_access.rs
git commit -m "refactor(remote-access): replace Bearer auth and REST API with path-prefix token middleware"
```

---

## Task 2: Backend — WebSocket bidirectional protocol

Add incoming message handling to the WebSocket for `openProject` and `closeProject` actions.

**Files:**
- Modify: `src-tauri/src/commands/remote_access.rs`

**Step 1: Add WebSocket action types**

```rust
/// Incoming action from a remote client via WebSocket.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum ClientAction {
    OpenProject { path: String },
    CloseProject { path: String },
}
```

**Step 2: Add test for ClientAction deserialization**

```rust
#[test]
fn test_client_action_open_project_deserialization() {
    let json = r#"{"action":"openProject","path":"/Users/user/projects/kiri"}"#;
    let action: ClientAction = serde_json::from_str(json).unwrap();
    match action {
        ClientAction::OpenProject { path } => assert_eq!(path, "/Users/user/projects/kiri"),
        _ => panic!("Expected OpenProject"),
    }
}

#[test]
fn test_client_action_close_project_deserialization() {
    let json = r#"{"action":"closeProject","path":"/Users/user/projects/kiri"}"#;
    let action: ClientAction = serde_json::from_str(json).unwrap();
    match action {
        ClientAction::CloseProject { path } => assert_eq!(path, "/Users/user/projects/kiri"),
        _ => panic!("Expected CloseProject"),
    }
}
```

**Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib commands::remote_access::tests -- client_action`
Expected: PASS

**Step 4: Modify `handle_status_ws` to process incoming actions**

```rust
async fn handle_status_ws(mut socket: WebSocket, state: AppState) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let status = collect_full_status(&state);
                match status {
                    Some(data) => {
                        let json = serde_json::to_string(&data).unwrap_or_default();
                        if socket.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(action) = serde_json::from_str::<ClientAction>(&text) {
                            handle_client_action(&state, action).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        if socket.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Handle an incoming client action.
async fn handle_client_action(state: &AppState, action: ClientAction) {
    let Some(app) = state.app_handle.as_ref() else {
        return;
    };

    match action {
        ClientAction::OpenProject { path } => {
            use tauri::Manager;
            let registry = app.state::<crate::commands::WindowRegistryState>();

            // Check if project is already open — focus existing window
            let existing_label = {
                let reg = match registry.lock() {
                    Ok(r) => r,
                    Err(_) => return,
                };
                reg.get_label_for_path(&path).cloned()
            };

            if let Some(label) = existing_label {
                if let Some(window) = app.get_webview_window(&label) {
                    let _ = window.set_focus();
                    return;
                }
            }

            // Create new window
            let _ = crate::commands::window::create_window_impl(
                app,
                Some(&registry),
                None,
                None,
                None,
                None,
                Some(path),
            );
        }
        ClientAction::CloseProject { path } => {
            use tauri::Manager;
            let registry = app.state::<crate::commands::WindowRegistryState>();
            let label = {
                let reg = match registry.lock() {
                    Ok(r) => r,
                    Err(_) => return,
                };
                reg.get_label_for_path(&path).cloned()
            };

            if let Some(label) = label {
                if let Some(window) = app.get_webview_window(&label) {
                    let _ = window.close();
                }
            }
        }
    }
}
```

**Step 5: Run all remote_access tests**

Run: `cd src-tauri && cargo test --lib commands::remote_access`
Expected: PASS

**Step 6: Commit**

```bash
git add src-tauri/src/commands/remote_access.rs
git commit -m "feat(remote-access): add bidirectional WebSocket protocol for client actions"
```

---

## Task 3: Backend — Cloudflare Quick Tunnel mode

Add Quick Tunnel support (no token, random URL parsed from stderr) alongside existing Named Tunnel.

**Files:**
- Modify: `src-tauri/src/commands/cloudflare_tunnel.rs`

**Step 1: Add URL parsing function and tests**

```rust
/// Parse the Quick Tunnel URL from cloudflared's stderr output.
///
/// cloudflared prints lines like:
/// `... | https://random-words.trycloudflare.com |`
pub fn parse_quick_tunnel_url(line: &str) -> Option<String> {
    // Look for https://*.trycloudflare.com pattern
    let re_pattern = "https://[a-zA-Z0-9-]+\\.trycloudflare\\.com";
    let re = regex::Regex::new(re_pattern).ok()?;
    re.find(line).map(|m| m.as_str().to_string())
}
```

Tests:

```rust
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
    let line = "INF +-------------------------------------------+\nINF |  https://bright-fox-lake.trycloudflare.com  |\nINF +-------------------------------------------+";
    assert_eq!(
        parse_quick_tunnel_url(line),
        Some("https://bright-fox-lake.trycloudflare.com".to_string())
    );
}
```

**Step 2: Run tests**

Run: `cd src-tauri && cargo test --lib commands::cloudflare_tunnel::tests`
Expected: PASS (after adding `regex` dependency if needed — check Cargo.toml)

**Step 3: Add `regex` to Cargo.toml if not present**

Run: `grep -q '^regex' src-tauri/Cargo.toml || echo "Need to add regex"`

If needed, add to `[dependencies]` in `src-tauri/Cargo.toml`:
```toml
regex = "1"
```

**Step 4: Refactor TunnelState to store URL**

```rust
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
```

**Step 5: Add `start_cloudflare_tunnel` for both modes**

Modify the `start_cloudflare_tunnel` command to accept `Option<String>` token (None = Quick Tunnel):

```rust
/// Start a Cloudflare Tunnel.
///
/// - `token: Some("...")` → Named Tunnel: `cloudflared tunnel run --token <token>`
/// - `token: None` → Quick Tunnel: `cloudflared tunnel --url http://localhost:<port>`
///
/// For Quick Tunnel, the URL is parsed from cloudflared's stderr output.
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

    let (child, url) = match token {
        Some(t) => {
            // Named Tunnel
            let child = std::process::Command::new(cloudflared_path())
                .args(["tunnel", "run", "--token", &t])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start cloudflared: {}", e))?;
            (child, None)
        }
        None => {
            // Quick Tunnel
            let local_url = format!("http://localhost:{}", port);
            let mut child = std::process::Command::new(cloudflared_path())
                .args(["tunnel", "--url", &local_url])
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to start cloudflared: {}", e))?;

            // Read stderr to find the tunnel URL (with timeout)
            let url = parse_tunnel_url_from_stderr(&mut child)?;
            (child, Some(url))
        }
    };

    tunnel.child = Some(child);
    tunnel.is_running = true;
    tunnel.url = url.clone();
    log::info!("Cloudflare Tunnel started (url: {:?})", url);
    Ok(url)
}

/// Read cloudflared stderr looking for the Quick Tunnel URL.
fn parse_tunnel_url_from_stderr(child: &mut std::process::Child) -> Result<String, String> {
    use std::io::{BufRead, BufReader};

    let stderr = child
        .stderr
        .take()
        .ok_or("Failed to capture stderr")?;
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
```

**Step 6: Update `stop_cloudflare_tunnel` to clear URL**

```rust
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
```

**Step 7: Run tests**

Run: `cd src-tauri && cargo test --lib commands::cloudflare_tunnel`
Expected: PASS

**Step 8: Commit**

```bash
git add src-tauri/src/commands/cloudflare_tunnel.rs src-tauri/Cargo.toml
git commit -m "feat(remote-access): add Cloudflare Quick Tunnel mode with URL parsing"
```

---

## Task 4: Backend — QR code generation update

Change QR code to encode full URL with token in path instead of JSON payload.

**Files:**
- Modify: `src-tauri/src/commands/remote_access_commands.rs`

**Step 1: Update `generate_remote_qr_code` to encode full URL**

```rust
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
        Some(url) => url,
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
```

**Step 2: Update tests**

```rust
#[test]
fn test_generate_qr_base64_encodes_url() {
    let url = "http://192.168.1.5:9876/abc-123-token/";
    let result = generate_qr_base64(url);
    assert!(result.is_ok());
}
```

**Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib commands::remote_access_commands`
Expected: PASS

**Step 4: Commit**

```bash
git add src-tauri/src/commands/remote_access_commands.rs
git commit -m "feat(remote-access): encode full URL with token path in QR code"
```

---

## Task 5: Backend — Update Tauri command registration

Update `lib.rs` to match new command signatures.

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Step 1: Update command registration**

The `start_cloudflare_tunnel` signature changed (now takes `Option<String>` token + `u16` port). The command names haven't changed, so `lib.rs` shouldn't need code changes for registration. But verify it compiles.

**Step 2: Run full backend build**

Run: `cd src-tauri && cargo build`
Expected: Compiles without errors.

**Step 3: Run all Rust tests**

Run: `cd src-tauri && cargo test`
Expected: PASS

**Step 4: Commit (if any changes needed)**

```bash
git add src-tauri/src/lib.rs
git commit -m "chore(remote-access): update lib.rs for new command signatures"
```

---

## Task 6: Frontend — persistenceService changes

Flatten `CloudflareConfig` into `RemoteAccessSettings`, add `tunnelUrl` field.

**Files:**
- Modify: `src/lib/services/persistenceService.ts`
- Modify: `src/lib/services/persistenceService.remote.test.ts`

**Step 1: Update the interface and defaults**

Replace the Remote Access Settings section in `persistenceService.ts`:

```typescript
/**
 * Remote access settings for the built-in server
 */
export interface RemoteAccessSettings {
  enabled: boolean;
  port: number;
  authToken: string | null;
  tunnelToken: string | null;  // Cloudflare tunnel token (null = Quick Tunnel)
  tunnelUrl: string | null;    // Named Tunnel URL (null when using Quick Tunnel)
}

export const DEFAULT_REMOTE_ACCESS_SETTINGS: RemoteAccessSettings = {
  enabled: false,
  port: 9876,
  authToken: null,
  tunnelToken: null,
  tunnelUrl: null,
};
```

**Step 2: Update `loadRemoteAccessSettings` for migration**

```typescript
export async function loadRemoteAccessSettings(): Promise<RemoteAccessSettings> {
  try {
    const s = await getStore();
    await s.reload();

    const settings = await s.get<RemoteAccessSettings & { cloudflare?: { enabled?: boolean; tunnelToken?: string | null } }>('remoteAccess');
    if (!settings) {
      return { ...DEFAULT_REMOTE_ACCESS_SETTINGS };
    }

    // Migrate from old CloudflareConfig format
    const tunnelToken = settings.tunnelToken
      ?? settings.cloudflare?.tunnelToken
      ?? DEFAULT_REMOTE_ACCESS_SETTINGS.tunnelToken;

    return {
      enabled: settings.enabled ?? DEFAULT_REMOTE_ACCESS_SETTINGS.enabled,
      port: settings.port ?? DEFAULT_REMOTE_ACCESS_SETTINGS.port,
      authToken: settings.authToken ?? DEFAULT_REMOTE_ACCESS_SETTINGS.authToken,
      tunnelToken,
      tunnelUrl: settings.tunnelUrl ?? DEFAULT_REMOTE_ACCESS_SETTINGS.tunnelUrl,
    };
  } catch (error) {
    console.error('Failed to load remote access settings:', error);
    return { ...DEFAULT_REMOTE_ACCESS_SETTINGS };
  }
}
```

**Step 3: Remove `CloudflareConfig` interface**

Delete `export interface CloudflareConfig { ... }`.

**Step 4: Update tests in `persistenceService.remote.test.ts`**

Update all test expectations to use the flattened format (no `.cloudflare`).

**Step 5: Run tests**

Run: `npm run test -- persistenceService.remote`
Expected: PASS

**Step 6: Commit**

```bash
git add src/lib/services/persistenceService.ts src/lib/services/persistenceService.remote.test.ts
git commit -m "refactor(remote-access): flatten CloudflareConfig into RemoteAccessSettings"
```

---

## Task 7: Frontend — remoteAccessService and store updates

Update service to match new backend signatures. Update store state.

**Files:**
- Modify: `src/lib/services/remoteAccessService.ts`
- Modify: `src/lib/services/remoteAccessService.test.ts`
- Modify: `src/lib/stores/remoteAccessStore.ts`
- Modify: `src/lib/stores/remoteAccessStore.test.ts`

**Step 1: Update `remoteAccessService.ts`**

```typescript
import { invoke } from '@tauri-apps/api/core';

export const remoteAccessService = {
  startServer: (port: number): Promise<void> => invoke('start_remote_server', { port }),

  stopServer: (): Promise<void> => invoke('stop_remote_server'),

  isRunning: (): Promise<boolean> => invoke('is_remote_server_running'),

  /**
   * Generate a QR code encoding the full access URL with token in path.
   * If tunnelUrl is provided, it is used as the base URL.
   */
  generateQrCode: (port: number, tunnelUrl?: string): Promise<string> =>
    invoke('generate_remote_qr_code', { port, tunnelUrl: tunnelUrl ?? null }),

  regenerateToken: (): Promise<string> => invoke('regenerate_remote_token'),

  /**
   * Start Cloudflare Tunnel.
   * token: null = Quick Tunnel, string = Named Tunnel
   * Returns the tunnel URL for Quick Tunnel mode.
   */
  startTunnel: (token: string | null, port: number): Promise<string | null> =>
    invoke('start_cloudflare_tunnel', { token, port }),

  stopTunnel: (): Promise<void> => invoke('stop_cloudflare_tunnel'),
};
```

**Step 2: Update `remoteAccessService.test.ts`**

Update tests to match new signatures:
- `generateQrCode` now takes `(port, tunnelUrl?)` — update test to call `generateQrCode(9876)`
- `startTunnel` now takes `(token | null, port)` — update test to call `startTunnel('my-token', 9876)` and `startTunnel(null, 9876)`

**Step 3: Update `remoteAccessStore.ts`**

No changes needed — the store already has the correct shape per the design doc:
```typescript
interface RemoteAccessState {
  serverRunning: boolean;
  tunnelRunning: boolean;
  tunnelUrl: string | null;
  port: number;
  hasToken: boolean;
}
```

**Step 4: Run tests**

Run: `npm run test -- remoteAccessService`
Run: `npm run test -- remoteAccessStore`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/services/remoteAccessService.ts src/lib/services/remoteAccessService.test.ts src/lib/stores/remoteAccessStore.ts src/lib/stores/remoteAccessStore.test.ts
git commit -m "feat(remote-access): update service and store for v2 API signatures"
```

---

## Task 8: Frontend — RemoteAccessSettings.svelte redesign

Remove Authentication section. Move QR code to Server Control. Redesign Cloudflare Tunnel section with Quick/Named modes.

**Files:**
- Modify: `src/lib/components/settings/RemoteAccessSettings.svelte`

**Step 1: Update the script section**

Key changes:
- Remove `isRevokingToken` state
- Remove `handleRevokeToken` function
- Update `toggleTunnel` to support Quick Tunnel (no token required)
- Update `handleGenerateQr` to pass port and tunnel URL
- Replace `settings.cloudflare.enabled` / `settings.cloudflare.tunnelToken` with `settings.tunnelToken`
- Add `tunnelUrlInput` for Named Tunnel URL

**Step 2: Update the template**

1. **Server Control section:** Add QR code display (moved from Authentication section). Show QR code when server is running.

2. **Remove entire Authentication section** (the `<div class="section">` with "Authentication" header, `handleGenerateQr`, `handleRevokeToken` buttons).

3. **Cloudflare Tunnel section:** Redesign:
   - ON/OFF toggle (no token required for Quick Tunnel)
   - Tunnel Token input (optional — empty = Quick Tunnel)
   - Tunnel URL input (shown only when token is set, for Named Tunnel)
   - Current tunnel URL display (for both modes)

**Step 3: Update `onMount` settings loading**

Replace:
```typescript
tunnelTokenInput = loaded.cloudflare.tunnelToken ?? '';
```
With:
```typescript
tunnelTokenInput = loaded.tunnelToken ?? '';
tunnelUrlInput = loaded.tunnelUrl ?? '';
```

**Step 4: Update `toggleTunnel` function**

```typescript
async function toggleTunnel() {
  if (!settings || isTogglingTunnel) return;
  isTogglingTunnel = true;

  try {
    if (store.tunnelRunning) {
      await remoteAccessService.stopTunnel();
      remoteAccessStore.setTunnelRunning(false);
      toastStore.info('Cloudflare tunnel stopped');
    } else {
      const token = tunnelTokenInput.trim() || null;
      const port = parseInt(portInput, 10);
      const tunnelUrl = await remoteAccessService.startTunnel(token, port);
      remoteAccessStore.setTunnelRunning(true, tunnelUrl ?? undefined);

      if (token) {
        settings.tunnelToken = token;
        settings.tunnelUrl = tunnelUrlInput.trim() || null;
      }
      toastStore.success('Cloudflare tunnel started');
    }
    await saveRemoteAccessSettings(settings);
  } catch (error) {
    toastStore.error('Failed to toggle tunnel: ' + String(error));
  } finally {
    isTogglingTunnel = false;
  }
}
```

**Step 5: Run frontend-design skill review**

Per project rules, review design changes with `frontend-design` skill.

**Step 6: Commit**

```bash
git add src/lib/components/settings/RemoteAccessSettings.svelte
git commit -m "refactor(remote-access): redesign settings panel with Quick/Named tunnel modes"
```

---

## Task 9: Frontend — StartScreen Remote Access shortcut

Add Remote Access row below Startup Command with ON/OFF toggle + settings gear icon.

**Files:**
- Modify: `src/lib/components/start/StartScreen.svelte`

**Step 1: Add imports and state**

Add to the `<script>` section:

```typescript
import { remoteAccessService } from '@/lib/services/remoteAccessService';
import { remoteAccessStore, isRemoteActive } from '@/lib/stores/remoteAccessStore';
import {
  loadRemoteAccessSettings,
  saveRemoteAccessSettings,
} from '@/lib/services/persistenceService';

let isTogglingRemote = $state(false);
let showRemoteSettings = $state(false);
let remotePort = $state(9876);
```

**Step 2: Add toggle handler**

```typescript
async function handleRemoteToggle() {
  if (isTogglingRemote) return;
  isTogglingRemote = true;

  try {
    const settings = await loadRemoteAccessSettings();
    if ($isRemoteActive) {
      await remoteAccessService.stopServer();
      remoteAccessStore.setServerRunning(false);
      settings.enabled = false;
    } else {
      await remoteAccessService.startServer(settings.port);
      remoteAccessStore.setServerRunning(true);
      remoteAccessStore.setPort(settings.port);
      remoteAccessStore.setHasToken(true);
      settings.enabled = true;
      remotePort = settings.port;
    }
    await saveRemoteAccessSettings(settings);
  } catch (error) {
    console.error('Failed to toggle remote access:', error);
  } finally {
    isTogglingRemote = false;
  }
}
```

**Step 3: Add Remote Access row after Startup Command row**

Add this markup right after the closing `</div>` of `.startup-command-row`:

```svelte
<div class="startup-command-row">
  <span class="startup-label">
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
      stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
      <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
      <line x1="8" y1="21" x2="16" y2="21"></line>
      <line x1="12" y1="17" x2="12" y2="21"></line>
    </svg>
    Remote Access
  </span>
  <div class="remote-controls">
    <button
      class="remote-toggle"
      class:active={$isRemoteActive}
      onclick={handleRemoteToggle}
      disabled={isTogglingRemote}
    >
      <span class="remote-status-dot" class:active={$isRemoteActive}></span>
      {$isRemoteActive ? 'ON' : 'OFF'}
    </button>
    <button class="remote-settings-btn" onclick={() => (showRemoteSettings = true)} aria-label="Remote access settings">
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor"
        stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <circle cx="12" cy="12" r="3"></circle>
        <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path>
      </svg>
    </button>
  </div>
</div>

{#if showRemoteSettings}
  <RemoteAccessSettings onClose={() => (showRemoteSettings = false)} />
{/if}
```

Import `RemoteAccessSettings` component:
```typescript
import RemoteAccessSettings from '@/lib/components/settings/RemoteAccessSettings.svelte';
```

**Step 4: Add styles for the Remote Access row**

```css
.remote-controls {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.remote-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 5px 14px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--border-color);
  border-radius: calc(var(--radius-md) - 3px);
  font-size: 11px;
  font-weight: 500;
  color: var(--text-muted);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.remote-toggle:hover:not(:disabled) {
  color: var(--text-secondary);
  background: rgba(125, 211, 252, 0.04);
}

.remote-toggle.active {
  background: var(--accent-subtle);
  color: var(--accent-color);
  border-color: rgba(125, 211, 252, 0.3);
}

.remote-toggle:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.remote-status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--git-deleted);
  transition: background var(--transition-fast);
}

.remote-status-dot.active {
  background: var(--git-added);
  box-shadow: 0 0 4px rgba(74, 222, 128, 0.4);
}

.remote-settings-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  padding: 0;
  background: transparent;
  border: 1px solid transparent;
  border-radius: calc(var(--radius-md) - 3px);
  color: var(--text-muted);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.remote-settings-btn:hover {
  color: var(--accent-color);
  background: rgba(125, 211, 252, 0.04);
  border-color: var(--border-color);
}
```

**Step 5: Load initial remote server status on mount**

Add to `onMount`:

```typescript
try {
  const running = await remoteAccessService.isRunning();
  remoteAccessStore.setServerRunning(running);
} catch {
  // Backend not ready yet
}
```

**Step 6: Run frontend-design skill review**

Per project rules, review design changes with `frontend-design` skill.

**Step 7: Commit**

```bash
git add src/lib/components/start/StartScreen.svelte
git commit -m "feat(remote-access): add Remote Access shortcut to start screen"
```

---

## Task 10: PWA — Remove auth screen, WebSocket-only, optimistic UI

Rewrite the PWA to connect directly via WebSocket (no auth screen, no REST API calls), with optimistic UI.

**Files:**
- Modify: `src-tauri/remote-ui/index.html`
- Modify: `src-tauri/remote-ui/app.js`
- Modify: `src-tauri/remote-ui/style.css`
- Modify: `src-tauri/remote-ui/sw.js`
- Modify: `src-tauri/remote-ui/manifest.json`

**Step 1: Update `index.html` — remove auth screen**

Replace the entire `<main>` content, removing the auth screen div:

```html
<main id="main-content">
  <div id="dashboard-screen" class="screen">
    <section class="section">
      <h2 class="section-title">Open Projects</h2>
      <div id="open-projects" class="project-list">
        <p class="empty-state">No open projects</p>
      </div>
    </section>
    <section class="section">
      <h2 class="section-title">Terminals</h2>
      <div id="terminals" class="terminal-list">
        <p class="empty-state">No terminals</p>
      </div>
    </section>
    <section class="section">
      <h2 class="section-title">Recent Projects</h2>
      <div id="recent-projects" class="project-list">
        <p class="empty-state">No recent projects</p>
      </div>
    </section>
  </div>
</main>
```

Update script and CSS references to use relative paths:
```html
<link rel="stylesheet" href="style.css" />
<!-- ... -->
<script src="app.js"></script>
```

Update manifest link to relative:
```html
<link rel="manifest" href="manifest.json" />
```

**Step 2: Rewrite `app.js` — WebSocket-only with optimistic UI**

```javascript
// kiri remote - PWA application logic (v2)

// ── State ─────────────────────────────────────────
var ws = null;
var reconnectTimer = null;
var lastStatus = null;

// ── DOM elements ──────────────────────────────────
var statusDot = document.getElementById('status-dot');
var statusText = document.getElementById('status-text');
var openProjectsEl = document.getElementById('open-projects');
var recentProjectsEl = document.getElementById('recent-projects');
var terminalsEl = document.getElementById('terminals');

// ── Initialize ────────────────────────────────────
function init() {
  // Register service worker with relative scope
  if ('serviceWorker' in navigator) {
    var basePath = getBasePath();
    navigator.serviceWorker.register(basePath + 'sw.js', { scope: basePath }).catch(function () {
      // Service worker registration failed -- not critical
    });
  }

  connectWebSocket();
}

// ── Path helpers ──────────────────────────────────
function getBasePath() {
  // Extract /{token}/ from the current URL path
  var path = location.pathname;
  // path is /{token}/ or /{token}/index.html
  var parts = path.split('/').filter(Boolean);
  if (parts.length > 0) {
    return '/' + parts[0] + '/';
  }
  return '/';
}

// ── WebSocket ─────────────────────────────────────
function connectWebSocket() {
  if (ws) ws.close();

  var protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
  var basePath = getBasePath();
  ws = new WebSocket(protocol + '//' + location.host + basePath + 'ws');

  ws.onopen = function () {
    setStatus('connected', 'Connected');
    clearTimeout(reconnectTimer);
  };

  ws.onmessage = function (e) {
    try {
      var data = JSON.parse(e.data);
      lastStatus = data;
      renderDashboard(data);
    } catch (err) {
      console.error('Failed to parse WS message:', err);
    }
  };

  ws.onclose = function () {
    setStatus('disconnected', 'Disconnected');
    reconnectTimer = setTimeout(connectWebSocket, 3000);
  };

  ws.onerror = function () {
    setStatus('disconnected', 'Error');
  };
}

function sendAction(action) {
  if (ws && ws.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(action));
  }
}

function setStatus(state, text) {
  statusDot.className = 'status-dot ' + state;
  statusText.textContent = text;
}

// ── Render ────────────────────────────────────────
function renderDashboard(data) {
  renderOpenProjects(data.openProjects || []);
  renderTerminals(data.terminals || []);
  renderRecentProjects(data.recentProjects || []);
}

function renderOpenProjects(projects) {
  if (projects.length === 0) {
    openProjectsEl.innerHTML = '<p class="empty-state">No open projects</p>';
    return;
  }

  openProjectsEl.innerHTML = projects
    .map(function (p) {
      return (
        '<div class="project-card open">' +
        '<div class="card-header">' +
        '<span class="project-name">' + escapeHtml(p.name) + '</span>' +
        (p.branch ? '<span class="branch-badge">' + escapeHtml(p.branch) + '</span>' : '') +
        '</div>' +
        '<div class="card-path">' + escapeHtml(p.path) + '</div>' +
        '<div class="card-actions">' +
        '<button class="btn btn-danger btn-sm" onclick="closeProject(\'' + escapeAttr(p.path) + '\')">Close</button>' +
        '</div>' +
        '</div>'
      );
    })
    .join('');
}

function renderTerminals(terminals) {
  if (terminals.length === 0) {
    terminalsEl.innerHTML = '<p class="empty-state">No terminals</p>';
    return;
  }

  terminalsEl.innerHTML = terminals
    .map(function (t) {
      return (
        '<div class="terminal-item">' +
        '<span class="terminal-dot ' + (t.isAlive ? 'active' : 'idle') + '"></span>' +
        '<span class="terminal-process">' + (t.processName ? escapeHtml(t.processName) : 'idle') + '</span>' +
        '<span class="terminal-id">#' + t.id + '</span>' +
        '</div>'
      );
    })
    .join('');
}

function renderRecentProjects(projects) {
  if (projects.length === 0) {
    recentProjectsEl.innerHTML = '<p class="empty-state">No recent projects</p>';
    return;
  }

  recentProjectsEl.innerHTML = projects
    .map(function (p) {
      return (
        '<div class="project-card recent">' +
        '<div class="card-header">' +
        '<span class="project-name">' + escapeHtml(p.name) + '</span>' +
        (p.gitBranch ? '<span class="branch-badge">' + escapeHtml(p.gitBranch) + '</span>' : '') +
        '</div>' +
        '<div class="card-meta">' + timeAgo(p.lastOpened) + '</div>' +
        '<div class="card-actions">' +
        '<button class="btn btn-primary btn-sm" onclick="openProject(\'' + escapeAttr(p.path) + '\')">Open</button>' +
        '</div>' +
        '</div>'
      );
    })
    .join('');
}

// ── Actions (Optimistic UI) ───────────────────────
function openProject(path) {
  // Optimistic: move from recent to open immediately
  if (lastStatus) {
    var project = null;
    var newRecent = [];
    (lastStatus.recentProjects || []).forEach(function (p) {
      if (p.path === path) {
        project = p;
      } else {
        newRecent.push(p);
      }
    });

    if (project) {
      var openList = lastStatus.openProjects || [];
      openList.push({ path: project.path, name: project.name, branch: project.gitBranch || null });
      lastStatus.openProjects = openList;
      lastStatus.recentProjects = newRecent;
      renderDashboard(lastStatus);
    }
  }

  sendAction({ action: 'openProject', path: path });
}

function closeProject(path) {
  // Optimistic: remove from open immediately
  if (lastStatus) {
    var closedProject = null;
    var newOpen = [];
    (lastStatus.openProjects || []).forEach(function (p) {
      if (p.path === path) {
        closedProject = p;
      } else {
        newOpen.push(p);
      }
    });

    lastStatus.openProjects = newOpen;

    if (closedProject) {
      var recentList = lastStatus.recentProjects || [];
      recentList.unshift({
        path: closedProject.path,
        name: closedProject.name,
        lastOpened: Math.floor(Date.now() / 1000),
        gitBranch: closedProject.branch,
      });
      lastStatus.recentProjects = recentList;
    }

    renderDashboard(lastStatus);
  }

  sendAction({ action: 'closeProject', path: path });
}

// ── Utilities ─────────────────────────────────────
function escapeHtml(str) {
  var div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function escapeAttr(str) {
  return str.replace(/\\/g, '\\\\').replace(/'/g, "\\'").replace(/"/g, '\\"');
}

function timeAgo(timestamp) {
  var seconds = Math.floor(Date.now() / 1000 - timestamp);
  if (seconds < 60) return 'Just now';
  if (seconds < 3600) return Math.floor(seconds / 60) + 'm ago';
  if (seconds < 86400) return Math.floor(seconds / 3600) + 'h ago';
  return Math.floor(seconds / 86400) + 'd ago';
}

// ── Start ─────────────────────────────────────────
init();
```

**Step 3: Update `sw.js` — handle token-prefixed paths**

```javascript
var CACHE_NAME = 'kiri-remote-v2';

self.addEventListener('install', function (e) {
  // Don't pre-cache — paths are dynamic (/{token}/)
  self.skipWaiting();
});

self.addEventListener('activate', function (e) {
  e.waitUntil(
    caches.keys().then(function (keys) {
      return Promise.all(
        keys
          .filter(function (k) { return k !== CACHE_NAME; })
          .map(function (k) { return caches.delete(k); })
      );
    })
  );
  self.clients.claim();
});

self.addEventListener('fetch', function (e) {
  if (e.request.method !== 'GET') return;

  var url = new URL(e.request.url);
  // Don't cache WebSocket upgrade or health endpoint
  if (url.pathname.endsWith('/ws') || url.pathname === '/api/health') return;

  e.respondWith(
    fetch(e.request)
      .then(function (response) {
        var clone = response.clone();
        caches.open(CACHE_NAME).then(function (cache) {
          cache.put(e.request, clone);
        });
        return response;
      })
      .catch(function () {
        return caches.match(e.request);
      })
  );
});
```

**Step 4: Update `manifest.json` — use relative start_url**

```json
{
  "name": "kiri remote",
  "short_name": "kiri",
  "start_url": ".",
  "display": "standalone",
  "background_color": "#0d1117",
  "theme_color": "#0d1117",
  "description": "Remote control for kiri terminal manager",
  "icons": []
}
```

**Step 5: Update `style.css` — remove auth screen styles**

Remove the entire `/* ── Auth screen */` section (lines 164-218: `#auth-screen`, `.auth-card`, `.auth-card::before`, `.auth-card h2`, `.auth-card p`, `.error-text`). Also remove `.auth-card .btn` rule.

Keep all other styles (header, dashboard, project cards, terminal list, buttons, inputs, toast, scrollbar).

**Step 6: Commit**

```bash
git add src-tauri/remote-ui/
git commit -m "feat(remote-access): rewrite PWA for URL-based auth, WebSocket-only, optimistic UI"
```

---

## Task 11: Integration — Update Rust integration test

Update the integration test to work with the new path-prefix token approach.

**Files:**
- Modify: `src-tauri/tests/remote_access_test.rs`

**Step 1: Read current integration test**

Read: `src-tauri/tests/remote_access_test.rs`

**Step 2: Update tests**

- All API requests must include `/{token}/` prefix in the path
- Remove tests for `/api/auth/verify`, `/api/projects`, `/api/projects/open`, `/api/projects/close`, `/api/terminals`
- Update WebSocket tests to connect to `/{token}/ws` instead of `/ws/status?token=...`
- Add test for invalid token prefix returning 404
- Keep health check test at `/api/health` (no token prefix)

**Step 3: Run integration tests**

Run: `cd src-tauri && cargo test --test remote_access_test`
Expected: PASS

**Step 4: Commit**

```bash
git add src-tauri/tests/remote_access_test.rs
git commit -m "test(remote-access): update integration tests for v2 path-prefix auth"
```

---

## Task 12: Verification — Full build and test

**Step 1: Run all frontend tests**

Run: `npm run test`
Expected: PASS

**Step 2: Run all backend tests**

Run: `npm run test:rust`
Expected: PASS

**Step 3: Run lint and type check**

Run: `npm run lint && npm run check`
Expected: PASS

**Step 4: Run `npm run tauri dev` and verify**

Per verification rules:
1. Confirm branch: `git branch --show-current`
2. Start app: `npm run tauri dev`
3. Use Tauri MCP server to verify:
   - StartScreen shows Remote Access row
   - Toggle ON/OFF works
   - Settings panel opens via gear icon
   - QR code generates
   - PWA loads at `http://localhost:{port}/{token}/`

**Step 5: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix(remote-access): address verification issues"
```
