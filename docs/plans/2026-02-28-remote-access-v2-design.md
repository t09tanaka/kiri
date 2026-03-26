# Remote Access v2 Design

## Goal

Simplify the remote access architecture: URL-based auth, WebSocket-only communication, Cloudflare Tunnel with Quick/Named modes, and a start screen shortcut.

## Architecture

### URL Structure

All paths are scoped under `/{token}/`:

```
http://192.168.1.5:9876/{token}/        → PWA (index.html)
http://192.168.1.5:9876/{token}/ws      → WebSocket
http://192.168.1.5:9876/{token}/app.js  → Static files
http://192.168.1.5:9876/{token}/*       → Static files
```

- Token is a UUID v4 auto-generated on first server start
- Any request with an invalid token prefix returns 404
- No Bearer token headers, no REST API endpoints
- `/api/health` remains at root level (no auth) for health checks

### WebSocket Protocol

Single WebSocket connection handles both status push and user actions.

**Server → Client (every 2 seconds):**
```json
{
  "type": "status",
  "openProjects": [{ "path": "...", "name": "...", "branch": "..." }],
  "terminals": [{ "id": 1, "isAlive": true, "processName": "claude" }],
  "recentProjects": [{ "path": "...", "name": "...", "lastOpened": 1700000000 }],
  "timestamp": 1700000000
}
```

**Client → Server (user actions):**
```json
{ "action": "openProject", "path": "/Users/.../kiri" }
{ "action": "closeProject", "path": "/Users/.../kiri" }
```

**Optimistic UI:** Client updates UI immediately on action, then overwrites with next status push for consistency.

No individual action responses — the next status push reflects the result.

### Cloudflare Tunnel

Two modes, selected automatically based on configuration:

| Mode | Condition | Command | URL |
|------|-----------|---------|-----|
| Quick Tunnel | No token configured | `cloudflared tunnel --url http://localhost:{port}` | Random `*.trycloudflare.com` (parsed from stderr) |
| Named Tunnel | Token + URL configured | `cloudflared tunnel run --token {token}` | User-configured fixed URL |

Quick Tunnel requires no account or setup — just cloudflared installed.

### QR Code

QR code encodes the full access URL (including auth token in path):
- LAN: `http://192.168.1.5:9876/{token}/`
- Quick Tunnel: `https://random-words.trycloudflare.com/{token}/`
- Named Tunnel: `https://kiri.example.com/{token}/`

## What Changes

### Remove
- Bearer token auth middleware (`auth_middleware`)
- All REST API endpoints (`/api/auth/verify`, `/api/projects`, `/api/terminals`, etc.)
- WebSocket query parameter auth (`?token=...`)
- PWA auth screen (token input page)
- Settings UI "Authentication" section
- `remoteAccessService.regenerateToken()` frontend method (keep backend, repurpose for token refresh)

### Modify
- **Router:** Token validation as path prefix middleware, serve static files under `/{token}/`
- **WebSocket handler:** Move from `/ws/status` to `/{token}/ws`, add incoming message handling for actions
- **PWA (app.js):** Remove auth logic, connect WebSocket relative to current path, add optimistic UI
- **PWA (index.html):** Remove auth screen
- **Cloudflare Tunnel (cloudflare_tunnel.rs):** Add Quick Tunnel mode (parse URL from stderr), keep Named Tunnel
- **QR code generation:** Encode full URL with token path instead of JSON with separate token field
- **Settings UI:** Remove Authentication section, move QR code to Server Control section
- **persistenceService:** Remove `CloudflareConfig.enabled`, add `tunnelUrl` field for Named Tunnel URL
- **remoteAccessStore:** Remove `tunnelRunning`/`tunnelUrl`, simplify to server + tunnel state

### Add
- **Start screen shortcut:** Remote Access row below Startup Command with ON/OFF toggle + settings icon
- **Path-based token middleware:** Validates `/{token}/` prefix, strips it before routing

## UI Changes

### Start Screen (StartScreen.svelte)

Add a row below Startup Command:

```
         Startup Command  [None] [Claude] [Codex]
         Remote Access     ● OFF  [⚙]
```

- Toggle directly starts/stops the server
- Gear icon opens RemoteAccessSettings panel (same as Cmd+Shift+R)
- Status dot: green when running, red/dim when stopped

### Settings Panel (RemoteAccessSettings.svelte)

**Section 1: Server Control**
- ON/OFF toggle + port input (unchanged)
- QR code display (moved from removed Authentication section)
- QR shown when server is running

**Section 2: Cloudflare Tunnel**
- ON/OFF toggle
- Tunnel Token input (optional — empty = Quick Tunnel)
- Tunnel URL input (shown only when token is set, for Named Tunnel)
- Current tunnel URL display (for both modes)

## Data Structures

### RemoteAccessSettings (persistenceService)

```typescript
interface RemoteAccessSettings {
  enabled: boolean;
  port: number;
  authToken: string | null;       // Server auth token (UUID in URL path)
  tunnelToken: string | null;     // Cloudflare tunnel token (null = Quick Tunnel)
  tunnelUrl: string | null;       // Named Tunnel URL (null when using Quick Tunnel)
}
```

`CloudflareConfig` interface is removed — fields are flattened into `RemoteAccessSettings`.

### RemoteAccessState (remoteAccessStore)

```typescript
interface RemoteAccessState {
  serverRunning: boolean;
  tunnelRunning: boolean;
  tunnelUrl: string | null;       // Active tunnel URL (Quick or Named)
  port: number;
  hasToken: boolean;
}
```
