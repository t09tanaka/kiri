# Remote Access Design

## Overview

Enable controlling kiri from a smartphone browser. Open projects, run startup commands, and monitor terminal process status remotely.

**Phase 1**: Available only while kiri is running (background/resident mode deferred to Phase 2).

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Mac (kiri app)                                 │
│                                                 │
│  ┌──────────┐    ┌──────────────────────────┐   │
│  │ Tauri    │    │ Embedded HTTP Server     │   │
│  │ Frontend │    │ (axum, port 9876)        │   │
│  │ (Svelte) │    │                          │   │
│  │          │    │ REST API + WebSocket     │   │
│  └────┬─────┘    │ + Static PWA files       │   │
│       │IPC       │                          │   │
│  ┌────┴─────┐    └──────────┬───────────────┘   │
│  │ Rust     │◄──────────────┘                   │
│  │ Backend  │                                   │
│  │          │    ┌──────────────────────────┐   │
│  │ State:   │    │ cloudflared              │   │
│  │ - TerminalManager                        │   │
│  │ - WindowRegistry                         │   │
│  │ - ProjectStore  │    (child process)     │   │
│  └──────────┘    └──────────────────────────┘   │
└─────────────────────────────────────────────────┘
        ▲
        │ Cloudflare Tunnel (HTTPS)
        ▼
┌───────────────┐
│ Smartphone    │
│ Browser (PWA) │
└───────────────┘
```

## API Design

### REST Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/auth/qr` | Generate QR code for initial pairing |
| POST | `/api/auth/verify` | Verify bearer token |
| GET | `/api/projects` | List open + recent projects |
| POST | `/api/projects/open` | Open project (window + startup command) |
| POST | `/api/projects/close` | Close project window |
| GET | `/api/terminals/:projectPath` | List terminals with active process info |

### WebSocket

| Path | Description |
|------|-------------|
| WS `/ws/status` | Real-time push of project and terminal state changes |

### Authentication

- Initial pairing via QR code displayed in kiri settings
- Subsequent requests use `Authorization: Bearer <token>`
- Token can be invalidated at any time (revokes all devices)
- Cloudflare Tunnel provides HTTPS encryption

### Response Examples

```json
// GET /api/projects
{
  "openProjects": [
    {
      "path": "/Users/user/projects/my-app",
      "name": "my-app",
      "branch": "develop",
      "terminals": [
        { "id": "term-1", "title": "claude", "activeProcess": "claude", "isActive": true },
        { "id": "term-2", "title": "dev", "activeProcess": "npm run dev", "isActive": true }
      ]
    }
  ],
  "recentProjects": [
    {
      "path": "/Users/user/projects/kiri",
      "name": "kiri",
      "branch": "main",
      "lastOpened": "2026-02-27T10:00:00Z"
    }
  ]
}
```

## ON/OFF Control

### Settings Hierarchy

```
Settings > Remote Access
├── [Toggle] Remote Access enabled/disabled    → starts/stops axum server
├── Port: 9876
├── Authentication
│   ├── [Show QR Code] Pairing
│   └── [Button] Revoke token (disconnect all devices)
└── Cloudflare Tunnel
    ├── [Toggle] Tunnel enabled/disabled       → starts/stops cloudflared
    ├── Token input field
    └── Connection status display
```

### Behavior Matrix

| Action | Effect |
|--------|--------|
| Remote Access ON | axum server starts on `0.0.0.0:9876` |
| Remote Access OFF | axum server stops, all connections dropped |
| Tunnel ON | `cloudflared` child process starts |
| Tunnel OFF | `cloudflared` child process killed (SIGTERM) |
| Revoke token | New token generated, old token rejected |
| kiri app quit | Server and Tunnel auto-stop |

### Settings Persistence

Added to existing `kiri-settings.json`:

```json
{
  "remoteAccess": {
    "enabled": false,
    "port": 9876,
    "authToken": "...",
    "cloudflare": {
      "enabled": false,
      "tunnelToken": "..."
    }
  }
}
```

Default: **everything OFF**. HTTP server does not start unless explicitly enabled.

## Smartphone UI (PWA)

### Screen Layout

```
┌─────────────────────────┐
│  kiri remote       🔴   │  ← connection status
├─────────────────────────┤
│                         │
│  Open Projects          │
│  ┌─────────────────┐   │
│  │ 📁 my-app    🟢  │   │
│  │ develop          │   │
│  │ Terminals:       │   │
│  │  ● claude        │   │
│  │  ● npm run dev   │   │
│  │  ○ zsh (idle)    │   │
│  │         [Close]  │   │
│  └─────────────────┘   │
│                         │
│  Recent Projects        │
│  ┌─────────────────┐   │
│  │ 📁 kiri          │   │
│  │ main • 2h ago    │   │
│  │         [Open]   │   │
│  └─────────────────┘   │
└─────────────────────────┘
```

### Technical Choices

- Plain HTML + CSS + vanilla JS (no framework)
- Served as static files by axum from `src-tauri/remote-ui/`
- PWA: `manifest.json` + Service Worker for home screen install
- Mobile-first responsive layout
- kiri "Mist" dark theme

## Cloudflare Tunnel Integration

### Setup Flow

1. User creates Tunnel in Cloudflare dashboard
2. Copies Tunnel token
3. Pastes token in kiri Remote Access settings
4. kiri bundles `cloudflared` binary
5. Toggle Tunnel ON → `cloudflared tunnel run --token <token>` as child process
6. Smartphone accesses `https://kiri.your-domain.com`

### cloudflared Management

| Item | Detail |
|------|--------|
| Binary | Bundled with kiri app |
| Startup | `cloudflared tunnel run --token <token>` as child process |
| Shutdown | SIGTERM for graceful shutdown |
| Logs | Integrated into kiri's log system |

## Data Structures

### RemoteAccessConfig

```typescript
interface RemoteAccessConfig {
  enabled: boolean;
  port: number;
  authToken: string | null;
  cloudflare: CloudflareConfig;
}

interface CloudflareConfig {
  enabled: boolean;
  tunnelToken: string | null;
}
```

### ProjectStatus (API response)

```typescript
interface ProjectStatus {
  path: string;
  name: string;
  branch: string;
  terminals: TerminalStatus[];
}

interface TerminalStatus {
  id: string;
  title: string;
  activeProcess: string | null;
  isActive: boolean;
}
```

## Scope

### Phase 1 (this implementation)

- Embedded axum HTTP server with ON/OFF toggle
- REST API for project management
- WebSocket for real-time status
- QR code pairing + bearer token auth
- PWA smartphone UI
- Cloudflare Tunnel integration (bundled cloudflared)

### Phase 2 (future)

- macOS menu bar resident mode (run without window)
- launchd auto-start on login
- Push notifications to smartphone
- Terminal output streaming to smartphone
