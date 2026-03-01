# Remote Access Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable controlling kiri from a smartphone browser — open projects, run startup commands, and monitor terminal process status remotely.

**Architecture:** Embed an axum HTTP server in kiri's Rust backend, sharing existing state (TerminalManager, WindowRegistry). Serve a PWA mobile UI as static files. Integrate Cloudflare Tunnel via bundled cloudflared binary for internet access. All features are OFF by default with per-setting ON/OFF toggles.

**Tech Stack:** axum (HTTP/WS), tokio (async), cloudflared (tunnel), vanilla HTML/CSS/JS (PWA)

---

### Task 1: Add Rust dependencies

**Files:**
- Modify: `src-tauri/Cargo.toml`

**Step 1: Add axum and related dependencies**

Add to `[dependencies]` section:

```toml
axum = { version = "0.8", features = ["ws"] }
axum-extra = { version = "0.10", features = ["typed-header"] }
tower-http = { version = "0.6", features = ["cors", "fs"] }
uuid = { version = "1", features = ["v4"] }
qrcode = "0.14"
image = { version = "0.25", default-features = false, features = ["png"] }
```

Note: `tokio` is already present with `rt-multi-thread`. `serde`/`serde_json` already present.

**Step 2: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles without errors

**Step 3: Commit**

```bash
git add src-tauri/Cargo.toml
git commit -m "chore: add axum and remote access dependencies"
```

---

### Task 2: Remote Access settings persistence

**Files:**
- Modify: `src/lib/services/persistenceService.ts`
- Create: `src/lib/services/persistenceService.remote.test.ts`

**Step 1: Write the failing test**

```typescript
// src/lib/services/persistenceService.remote.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';

// Mock tauri-plugin-store
const mockStore = {
  get: vi.fn(),
  set: vi.fn(),
  save: vi.fn(),
  reload: vi.fn(),
};
vi.mock('@tauri-apps/plugin-store', () => ({
  Store: { load: vi.fn().mockResolvedValue(mockStore) },
}));

import {
  loadRemoteAccessSettings,
  saveRemoteAccessSettings,
  DEFAULT_REMOTE_ACCESS_SETTINGS,
  type RemoteAccessSettings,
} from './persistenceService';

describe('Remote Access Settings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should return default settings when none are stored', async () => {
    mockStore.get.mockResolvedValue(null);
    const settings = await loadRemoteAccessSettings();
    expect(settings).toEqual(DEFAULT_REMOTE_ACCESS_SETTINGS);
    expect(settings.enabled).toBe(false);
    expect(settings.port).toBe(9876);
    expect(settings.authToken).toBeNull();
    expect(settings.cloudflare.enabled).toBe(false);
  });

  it('should save and load remote access settings', async () => {
    const settings: RemoteAccessSettings = {
      enabled: true,
      port: 9876,
      authToken: 'test-token',
      cloudflare: { enabled: false, tunnelToken: null },
    };
    await saveRemoteAccessSettings(settings);
    expect(mockStore.set).toHaveBeenCalledWith('remoteAccess', settings);
    expect(mockStore.save).toHaveBeenCalled();
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npm run test -- --run src/lib/services/persistenceService.remote.test.ts`
Expected: FAIL — `loadRemoteAccessSettings` not exported

**Step 3: Add types and functions to persistenceService.ts**

Add to `src/lib/services/persistenceService.ts`:

```typescript
// After existing interfaces
export interface CloudflareConfig {
  enabled: boolean;
  tunnelToken: string | null;
}

export interface RemoteAccessSettings {
  enabled: boolean;
  port: number;
  authToken: string | null;
  cloudflare: CloudflareConfig;
}

export const DEFAULT_REMOTE_ACCESS_SETTINGS: RemoteAccessSettings = {
  enabled: false,
  port: 9876,
  authToken: null,
  cloudflare: {
    enabled: false,
    tunnelToken: null,
  },
};

export async function loadRemoteAccessSettings(): Promise<RemoteAccessSettings> {
  const s = await getStore();
  await s.reload();
  const settings = await s.get<RemoteAccessSettings>('remoteAccess');
  return settings ?? { ...DEFAULT_REMOTE_ACCESS_SETTINGS };
}

export async function saveRemoteAccessSettings(
  settings: RemoteAccessSettings,
): Promise<void> {
  const s = await getStore();
  await s.set('remoteAccess', settings);
  await s.save();
}
```

**Step 4: Run test to verify it passes**

Run: `npm run test -- --run src/lib/services/persistenceService.remote.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/services/persistenceService.ts src/lib/services/persistenceService.remote.test.ts
git commit -m "feat(remote): add remote access settings persistence"
```

---

### Task 3: Remote Access Rust module — HTTP server core

**Files:**
- Create: `src-tauri/src/commands/remote_access.rs`
- Create: `src-tauri/src/commands/remote_access_commands.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/tests/remote_access_test.rs`

**Step 1: Write the integration test**

```rust
// src-tauri/tests/remote_access_test.rs
use std::net::TcpListener;

// Test that the server can bind to a port and respond to health check
#[tokio::test]
async fn test_remote_server_starts_and_responds() {
    // Find available port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);

    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    let server_handle = tokio::spawn(async move {
        kiri_lib::commands::remote_access::start_server(port, shutdown_rx).await.unwrap();
    });

    // Wait for server to start
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let client = reqwest::Client::new();
    let resp = client.get(format!("http://127.0.0.1:{}/api/health", port))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["status"], "ok");

    shutdown_tx.send(()).unwrap();
    let _ = server_handle.await;
}
```

Note: Add `reqwest` as dev dependency in Cargo.toml: `reqwest = { version = "0.12", features = ["json"] }`

**Step 2: Run test to verify it fails**

Run: `cd src-tauri && cargo test --test remote_access_test`
Expected: FAIL — module not found

**Step 3: Create remote_access.rs with server core**

```rust
// src-tauri/src/commands/remote_access.rs
use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub fn create_router() -> Router {
    Router::new()
        .route("/api/health", get(health_handler))
}

pub async fn start_server(
    port: u16,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router();
    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = shutdown_rx.await;
        })
        .await?;

    Ok(())
}
```

**Step 4: Create remote_access_commands.rs with Tauri command wrappers**

```rust
// src-tauri/src/commands/remote_access_commands.rs
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

pub struct RemoteServerState {
    shutdown_tx: Option<oneshot::Sender<()>>,
    port: u16,
    is_running: bool,
}

impl RemoteServerState {
    pub fn new() -> Self {
        Self {
            shutdown_tx: None,
            port: 9876,
            is_running: false,
        }
    }
}

pub type RemoteServerStateType = Arc<Mutex<RemoteServerState>>;

#[tauri::command]
pub async fn start_remote_server(
    state: tauri::State<'_, RemoteServerStateType>,
    port: u16,
) -> Result<(), String> {
    let mut server = state.lock().await;
    if server.is_running {
        return Err("Server is already running".to_string());
    }

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        if let Err(e) = super::remote_access::start_server(port, shutdown_rx).await {
            log::error!("Remote server error: {}", e);
        }
    });

    server.shutdown_tx = Some(shutdown_tx);
    server.port = port;
    server.is_running = true;
    Ok(())
}

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
    server.is_running = false;
    Ok(())
}

#[tauri::command]
pub async fn is_remote_server_running(
    state: tauri::State<'_, RemoteServerStateType>,
) -> Result<bool, String> {
    let server = state.lock().await;
    Ok(server.is_running)
}
```

**Step 5: Register in mod.rs and lib.rs**

In `src-tauri/src/commands/mod.rs`, add:
```rust
pub mod remote_access;
pub mod remote_access_commands;
pub use remote_access_commands::{
    start_remote_server, stop_remote_server, is_remote_server_running,
    RemoteServerState, RemoteServerStateType,
};
```

In `src-tauri/src/lib.rs`, add:
- State: `.manage(Arc::new(tokio::sync::Mutex::new(commands::RemoteServerState::new())) as commands::RemoteServerStateType)`
- Commands: `start_remote_server, stop_remote_server, is_remote_server_running` to `generate_handler![]`

**Step 6: Run integration test**

Run: `cd src-tauri && cargo test --test remote_access_test`
Expected: PASS

**Step 7: Commit**

```bash
git add src-tauri/
git commit -m "feat(remote): add embedded axum HTTP server with health endpoint"
```

---

### Task 4: Authentication — QR code and token

**Files:**
- Modify: `src-tauri/src/commands/remote_access.rs`
- Create: `src-tauri/tests/remote_auth_test.rs`

**Step 1: Write the test**

```rust
// src-tauri/tests/remote_auth_test.rs
#[tokio::test]
async fn test_auth_rejects_without_token() {
    // Start server, make request without Authorization header
    // Expected: 401 Unauthorized
}

#[tokio::test]
async fn test_auth_accepts_valid_token() {
    // Start server with known token, make request with Bearer token
    // Expected: 200 OK
}

#[tokio::test]
async fn test_qr_code_generation() {
    // Start server, GET /api/auth/qr with valid token
    // Expected: 200 with PNG image data
}
```

**Step 2: Run to verify fail**

Run: `cd src-tauri && cargo test --test remote_auth_test`

**Step 3: Implement auth middleware and QR endpoint**

Add to `remote_access.rs`:
- Bearer token extraction middleware using axum's `middleware::from_fn`
- Token validation against stored auth token
- `/api/auth/qr` endpoint using `qrcode` and `image` crates to generate QR PNG
- `/api/auth/verify` POST endpoint

Auth middleware skips `/api/auth/qr` (needs to be accessed from kiri desktop UI).

QR code encodes JSON: `{"url": "http://host:port", "token": "bearer-token"}`

**Step 4: Run tests**

Run: `cd src-tauri && cargo test --test remote_auth_test`
Expected: PASS

**Step 5: Commit**

```bash
git add src-tauri/
git commit -m "feat(remote): add bearer token auth middleware and QR code generation"
```

---

### Task 5: Project API endpoints

**Files:**
- Modify: `src-tauri/src/commands/remote_access.rs`
- Create: `src-tauri/tests/remote_projects_test.rs`

**Step 1: Write tests**

Test `GET /api/projects` returns open + recent projects.
Test `POST /api/projects/open` opens a project window.
Test `POST /api/projects/close` closes a project window.

**Step 2: Implement endpoints**

The axum server needs access to Tauri's state. Pass `AppHandle` to the axum router via axum's `State` extractor:

```rust
pub async fn start_server(
    port: u16,
    shutdown_rx: oneshot::Receiver<()>,
    app_handle: tauri::AppHandle,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = create_router(app_handle);
    // ...
}
```

Endpoints:
- `GET /api/projects` — Read from `kiri-settings.json` store + check WindowRegistry for open projects
- `POST /api/projects/open` — Call `create_window_impl()` with project path, then execute startup command
- `POST /api/projects/close` — Find window by project path via WindowRegistry, close it

**Step 3: Run tests, verify pass**

**Step 4: Commit**

```bash
git commit -m "feat(remote): add project list, open, and close API endpoints"
```

---

### Task 6: Terminal status API + WebSocket

**Files:**
- Modify: `src-tauri/src/commands/remote_access.rs`
- Create: `src-tauri/tests/remote_terminals_test.rs`

**Step 1: Write tests**

Test `GET /api/terminals/:projectPath` returns terminal list with active process info.
Test WebSocket `/ws/status` sends real-time updates.

**Step 2: Implement terminal status endpoint**

```rust
// GET /api/terminals/:project_path
async fn get_terminals(
    State(app): State<AppHandle>,
    Path(project_path): Path<String>,
) -> Result<Json<TerminalsResponse>, StatusCode> {
    let decoded = urlencoding::decode(&project_path).map_err(|_| StatusCode::BAD_REQUEST)?;
    let terminal_state = app.state::<TerminalState>();
    let manager = terminal_state.lock().unwrap();
    // Iterate manager.instances, call get_process_name equivalent for each
    // Filter by window registry to match project_path
}
```

**Step 3: Implement WebSocket status push**

```rust
// WS /ws/status
async fn ws_status(
    ws: WebSocketUpgrade,
    State(app): State<AppHandle>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_status_ws(socket, app))
}

async fn handle_status_ws(mut socket: WebSocket, app: AppHandle) {
    // Poll terminal states every 2 seconds
    // Send JSON updates when state changes
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let status = collect_all_status(&app);
                if socket.send(Message::Text(serde_json::to_string(&status).unwrap())).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                if msg.is_none() { break; }
            }
        }
    }
}
```

**Step 4: Run tests, verify pass**

**Step 5: Commit**

```bash
git commit -m "feat(remote): add terminal status API and WebSocket real-time updates"
```

---

### Task 7: Frontend — Remote Access service and store

**Files:**
- Create: `src/lib/services/remoteAccessService.ts`
- Create: `src/lib/services/remoteAccessService.test.ts`
- Create: `src/lib/stores/remoteAccessStore.ts`

**Step 1: Write tests for service**

```typescript
// src/lib/services/remoteAccessService.test.ts
import { describe, it, expect, vi } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { remoteAccessService } from './remoteAccessService';
import { invoke } from '@tauri-apps/api/core';

describe('remoteAccessService', () => {
  it('should call start_remote_server with port', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await remoteAccessService.startServer(9876);
    expect(invoke).toHaveBeenCalledWith('start_remote_server', { port: 9876 });
  });

  it('should call stop_remote_server', async () => {
    vi.mocked(invoke).mockResolvedValue(undefined);
    await remoteAccessService.stopServer();
    expect(invoke).toHaveBeenCalledWith('stop_remote_server');
  });

  it('should call is_remote_server_running', async () => {
    vi.mocked(invoke).mockResolvedValue(true);
    const result = await remoteAccessService.isRunning();
    expect(result).toBe(true);
  });
});
```

**Step 2: Run to verify fail**

**Step 3: Implement service and store**

```typescript
// src/lib/services/remoteAccessService.ts
import { invoke } from '@tauri-apps/api/core';

export const remoteAccessService = {
  startServer: (port: number): Promise<void> =>
    invoke('start_remote_server', { port }),
  stopServer: (): Promise<void> =>
    invoke('stop_remote_server'),
  isRunning: (): Promise<boolean> =>
    invoke('is_remote_server_running'),
  generateQrCode: (): Promise<string> =>
    invoke('generate_remote_qr_code'),
  regenerateToken: (): Promise<string> =>
    invoke('regenerate_remote_token'),
  startTunnel: (token: string): Promise<void> =>
    invoke('start_cloudflare_tunnel', { token }),
  stopTunnel: (): Promise<void> =>
    invoke('stop_cloudflare_tunnel'),
};
```

```typescript
// src/lib/stores/remoteAccessStore.ts
import { writable, derived } from 'svelte/store';

interface RemoteAccessState {
  serverRunning: boolean;
  tunnelRunning: boolean;
  tunnelUrl: string | null;
  port: number;
  hasToken: boolean;
}

function createRemoteAccessStore() {
  const { subscribe, set, update } = writable<RemoteAccessState>({
    serverRunning: false,
    tunnelRunning: false,
    tunnelUrl: null,
    port: 9876,
    hasToken: false,
  });

  return {
    subscribe,
    setServerRunning: (running: boolean) =>
      update((s) => ({ ...s, serverRunning: running })),
    setTunnelRunning: (running: boolean, url?: string) =>
      update((s) => ({ ...s, tunnelRunning: running, tunnelUrl: url ?? null })),
    setPort: (port: number) => update((s) => ({ ...s, port })),
    setHasToken: (has: boolean) => update((s) => ({ ...s, hasToken: has })),
  };
}

export const remoteAccessStore = createRemoteAccessStore();
export const isRemoteActive = derived(remoteAccessStore, ($s) => $s.serverRunning);
```

**Step 4: Run tests, verify pass**

**Step 5: Commit**

```bash
git commit -m "feat(remote): add remoteAccessService and remoteAccessStore"
```

---

### Task 8: Settings UI — Remote Access panel

**Files:**
- Create: `src/lib/components/settings/RemoteAccessSettings.svelte`
- Modify: Settings modal to include Remote Access tab (find existing settings UI entry point)

**Step 1: Create RemoteAccessSettings.svelte**

Follow the pattern from `SearchSettingsPanel.svelte`. Include:

- **Remote Access toggle** — calls `remoteAccessService.startServer()` / `stopServer()`
- **Port input** — number input, default 9876
- **QR Code section** — button to show QR, display as `<img>` from base64
- **Revoke Token button** — calls `regenerateToken()`, shows confirmation
- **Cloudflare Tunnel section:**
  - Toggle ON/OFF
  - Token input field
  - Connection status indicator (green dot / red dot)

Apply kiri Mist design:
- Glass effect background
- Soft borders (`var(--border-glow)`)
- Accent color toggles
- Form input rules (spellcheck="false", autocomplete="off", etc.)

**Step 2: Wire into settings modal**

Add "Remote Access" tab/section to the existing settings UI.

**Step 3: Test manually via `npm run tauri dev`**

Verify: toggle works, QR displays, settings persist after restart.

**Step 4: Commit**

```bash
git commit -m "feat(remote): add Remote Access settings UI panel"
```

---

### Task 9: Cloudflare Tunnel integration

**Files:**
- Create: `src-tauri/src/commands/cloudflare_tunnel.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

**Step 1: Write test**

```rust
// src-tauri/tests/cloudflare_tunnel_test.rs
#[test]
fn test_cloudflared_binary_path() {
    let path = kiri_lib::commands::cloudflare_tunnel::cloudflared_path();
    // Should point to bundled binary location
    assert!(path.ends_with("cloudflared"));
}
```

**Step 2: Implement cloudflare_tunnel.rs**

```rust
// src-tauri/src/commands/cloudflare_tunnel.rs
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TunnelState {
    child: Option<std::process::Child>,
    is_running: bool,
}

pub type TunnelStateType = Arc<Mutex<TunnelState>>;

pub fn cloudflared_path() -> std::path::PathBuf {
    // In bundled app: use resource dir
    // In dev: use system PATH
    if cfg!(debug_assertions) {
        std::path::PathBuf::from("cloudflared")
    } else {
        // Bundled alongside the app binary
        std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join("cloudflared")
    }
}

#[tauri::command]
pub async fn start_cloudflare_tunnel(
    state: tauri::State<'_, TunnelStateType>,
    token: String,
) -> Result<(), String> {
    let mut tunnel = state.lock().await;
    if tunnel.is_running {
        return Err("Tunnel is already running".to_string());
    }

    let child = Command::new(cloudflared_path())
        .args(["tunnel", "run", "--token", &token])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to start cloudflared: {}", e))?;

    tunnel.child = Some(child);
    tunnel.is_running = true;
    Ok(())
}

#[tauri::command]
pub async fn stop_cloudflare_tunnel(
    state: tauri::State<'_, TunnelStateType>,
) -> Result<(), String> {
    let mut tunnel = state.lock().await;
    if let Some(ref mut child) = tunnel.child {
        child.kill().map_err(|e| format!("Failed to stop cloudflared: {}", e))?;
        child.wait().ok();
    }
    tunnel.child = None;
    tunnel.is_running = false;
    Ok(())
}
```

**Step 3: Bundle cloudflared binary**

Add to `src-tauri/tauri.conf.json` under `bundle.resources`:
```json
{
  "bundle": {
    "resources": ["binaries/cloudflared"]
  }
}
```

Download platform-specific cloudflared binary to `src-tauri/binaries/`.

**Step 4: Run tests**

**Step 5: Commit**

```bash
git commit -m "feat(remote): add Cloudflare Tunnel integration with bundled cloudflared"
```

---

### Task 10: PWA mobile UI

**Files:**
- Create: `src-tauri/remote-ui/index.html`
- Create: `src-tauri/remote-ui/app.js`
- Create: `src-tauri/remote-ui/style.css`
- Create: `src-tauri/remote-ui/manifest.json`
- Create: `src-tauri/remote-ui/sw.js`
- Modify: `src-tauri/src/commands/remote_access.rs` (serve static files)

**Step 1: Create mobile UI**

Build a single-page app with vanilla JS:

- **Header:** "kiri remote" + connection status dot
- **Open Projects section:** Cards showing project name, branch, terminal list with active processes
- **Recent Projects section:** Cards showing project name, branch, last opened time, Open button
- **Connection:** WebSocket to `/ws/status` for real-time updates
- **Actions:** Open/Close buttons call REST API with bearer token
- **Design:** kiri Mist theme (dark, glass effects, `#7dd3fc` accent)

```html
<!-- src-tauri/remote-ui/index.html -->
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="theme-color" content="#0d1117">
  <link rel="manifest" href="/manifest.json">
  <title>kiri remote</title>
  <link rel="stylesheet" href="/style.css">
</head>
<body>
  <div id="app">
    <header>
      <h1>kiri remote</h1>
      <span id="status-dot" class="status-dot disconnected"></span>
    </header>
    <main>
      <section id="open-projects"></section>
      <section id="recent-projects"></section>
    </main>
  </div>
  <script src="/app.js"></script>
</body>
</html>
```

**Step 2: Serve static files from axum**

```rust
// In create_router()
use tower_http::services::ServeDir;

Router::new()
    .route("/api/health", get(health_handler))
    // ... other API routes
    .fallback_service(ServeDir::new("remote-ui"))
```

For bundled app, use Tauri's resource dir to locate `remote-ui/`.

**Step 3: Create PWA manifest and service worker**

```json
// manifest.json
{
  "name": "kiri remote",
  "short_name": "kiri",
  "start_url": "/",
  "display": "standalone",
  "background_color": "#0d1117",
  "theme_color": "#0d1117",
  "icons": [{ "src": "/icon-192.png", "sizes": "192x192", "type": "image/png" }]
}
```

**Step 4: Test on smartphone browser**

Start kiri with remote access ON, access from phone on same network.
Verify: project list shows, Open button works, terminal status updates via WebSocket.

**Step 5: Commit**

```bash
git commit -m "feat(remote): add PWA mobile UI with project dashboard"
```

---

### Task 11: Integration testing and polish

**Files:**
- All modified files from previous tasks

**Step 1: Run full test suite**

```bash
npm run test
npm run test:rust
npm run lint
npm run check
```

**Step 2: Fix any issues**

**Step 3: Manual E2E test**

1. Start kiri → open a project
2. Settings → Remote Access → Enable
3. Open phone browser → navigate to `http://<mac-ip>:9876`
4. Verify project list with terminal status
5. Open another project from phone
6. Verify it opens on Mac with startup command
7. Toggle Remote Access OFF → verify phone loses connection
8. Toggle back ON → verify phone reconnects

**Step 4: Run frontend design review**

Use `frontend-design` skill to review the Remote Access settings panel.

**Step 5: Final commit**

```bash
git commit -m "test(remote): add integration tests and polish"
```

---

## Task Dependency Graph

```
Task 1 (deps)
  └─→ Task 2 (settings persistence)
  └─→ Task 3 (HTTP server core)
        └─→ Task 4 (auth)
        └─→ Task 5 (project API)
        └─→ Task 6 (terminal API + WS)
        └─→ Task 10 (PWA UI)
  └─→ Task 7 (frontend service/store) ← depends on Task 3
        └─→ Task 8 (settings UI) ← depends on Task 7
  └─→ Task 9 (Cloudflare Tunnel)
  └─→ Task 11 (integration) ← depends on all above
```

Parallelizable groups:
- **Group A** (Rust backend): Tasks 3 → 4 → 5 → 6
- **Group B** (Frontend): Tasks 2 → 7 → 8
- **Group C** (Infrastructure): Tasks 1, 9, 10
- **Final**: Task 11
