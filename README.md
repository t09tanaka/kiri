# kiri 霧

> Light as mist, with only the features you need.

**A terminal-first development environment built for AI Coding Agents.**

Designed for developers who work with Claude Code, Aider, and other AI coding assistants. Seamlessly manage multiple projects with Git Worktree integration and multi-window support.

## Why kiri?

Modern AI coding agents like Claude Code run in the terminal, generating rapid output with spinners and progress bars. Traditional terminal emulators struggle with this, causing flickering and rendering glitches. kiri solves these problems while providing essential features for agent-assisted development:

- **Flicker-free terminal** — Optimized for Ink-based CLI tools
- **Multi-window workflow** — Work on multiple branches simultaneously
- **Git Worktree integration** — One-click branch isolation with automatic port isolation
- **Remote Access** — Monitor and control projects from your phone via QR code
- **Clickable file paths** — Jump to files mentioned in agent output
- **Peek preview** — Quick file inspection without leaving terminal

## Screenshots

<!-- TODO: Add screenshots -->

## Features

### 🤖 AI Coding Agent Optimization

Built from the ground up for AI coding assistants:

| Feature | Description |
|---------|-------------|
| **Synchronized Output Mode** | Buffers rapid updates (DEC Private Mode 2026) to eliminate flickering |
| **Smart Width Limiting** | Caps terminal at 120 columns to prevent spinner glitches |
| **Resize Stability** | Drops partial frames during resize, waits for agent redraw |
| **PTY Row Adjustment** | Prevents Ink full-height scrolling issues ([#450](https://github.com/vadimdemedes/ink/issues/450)) |
| **File Path Links** | `Cmd+Click` on `file.ts:42` in output → opens editor at line 42 |
| **Peek Editor** | Quick preview popup without disrupting your terminal flow |
| **Startup Command** | Auto-launch `claude` or `codex` when opening a project |

### 🪟 Multi-Window & Git Worktree

Parallel development made simple:

| Feature | Description |
|---------|-------------|
| **Multiple Windows** | `Cmd+Shift+N` opens a new window |
| **Git Worktree Panel** | `Cmd+G` to create/manage worktrees |
| **Auto Window Creation** | New worktree → new window automatically |
| **Branch Isolation** | Work on features without stashing or switching |
| **File Copy** | Copies `.env*` and configured files to new worktrees |
| **Port Isolation** | Automatically assigns unique ports to each worktree |
| **Auto Initialization** | Runs `npm install` (or detected package manager) on creation |
| **Recent Projects** | Quick access to your project history |

See [Git Worktree Support](#git-worktree-support) for details.

### 📱 Remote Access

Monitor and control your projects from any device:

| Feature | Description |
|---------|-------------|
| **One-Click Start** | Toggle remote access from the start screen or `Cmd+Shift+R` |
| **QR Code Connection** | Scan with your phone to connect instantly |
| **Live Project Status** | See open projects, branches, and running processes |
| **Remote Control** | Open and close projects from your phone |
| **Cloudflare Tunnel** | Secure access from anywhere via `cloudflared` |
| **Secure by Default** | Token-based authentication with constant-time validation |

See [Remote Access](#remote-access) for details.

### 💻 Terminal

| Feature | Description |
|---------|-------------|
| **Multiple Tabs** | `` Cmd+` `` for new terminal, `Cmd+1-9` to switch |
| **Pane Splitting** | Split horizontally or vertically |
| **Zoom Control** | `Cmd+`/`-` to adjust font size |
| **Independent Sessions** | Each pane runs its own PTY |

### 📝 File Viewer

| Feature | Description |
|---------|-------------|
| **Syntax Highlighting** | 20+ languages (TypeScript, Rust, Svelte, etc.) |
| **Git Diff Gutter** | See changes inline |
| **Diff View Modal** | `Cmd+D` for full diff comparison view |
| **Copy Button** | One-click code copying |
| **Quick Open** | `Cmd+P` for fuzzy file search |
| **Content Search** | `Cmd+Shift+F` for full-text search with regex support |

> **Note**: kiri is a read-only file viewer optimized for AI coding agents. Human editing features are intentionally omitted. For editing, use your preferred external editor or let your AI agent handle it.

### 📊 Git History

| Feature | Description |
|---------|-------------|
| **Commit Graph** | `Cmd+H` for visual branch history with graph visualization |
| **Remote Tracking** | Shows ahead/behind counts vs remote branches |
| **Fetch Polling** | Automatically checks for new remote commits |

### 📁 File Management

| Feature | Description |
|---------|-------------|
| **File Tree** | Collapsible directory navigation |
| **Context Menu** | Right-click for file operations |
| **Git Status** | Visual indicators for modified/staged files |
| **Lazy Loading** | Fast navigation in large repositories |

### ⌨️ Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+O` | Open Directory |
| `Cmd+P` | Quick Open |
| `Cmd+Shift+F` | Content Search |
| `Cmd+B` | Toggle Sidebar |
| `Cmd+D` | Diff View |
| `Cmd+H` | Commit History |
| `Cmd+G` | Git Worktrees |
| `Cmd+Shift+N` | New Window |
| `Cmd+Shift+W` | Close Project |
| `Cmd+Shift+R` | Remote Access Settings |
| `` Cmd+` `` | New Terminal |
| `Cmd+1-9` | Switch Terminal Tab |
| `Cmd+W` | Close Tab |
| `Cmd+/` | Show All Shortcuts |

## Git Worktree Support

Git Worktree lets you work on multiple branches simultaneously, each in its own isolated directory and window. This is essential for AI-assisted development where you might have one agent working on a feature while reviewing another branch.

### How It Works

1. Press `Cmd+G` to open the Worktree Panel
2. Enter or select a branch name
3. kiri creates a worktree directory alongside your repo (e.g., `kiri-feature-auth/`)
4. Files are copied, dependencies are installed, and a new window opens automatically

```
~/projects/
├── kiri/                     ← Main repository (main branch)
├── kiri-feature-auth/        ← Worktree (feature/auth branch)
└── kiri-fix-login/           ← Worktree (fix-login branch)
```

### Automatic Setup

When creating a worktree, kiri handles the tedious setup automatically:

- **File Copy** — Copies `.env*` files and other configured patterns to the new worktree
- **Package Manager Detection** — Detects npm/yarn/pnpm/bun from lock files and runs the appropriate install command
- **Custom Init Commands** — Add your own setup commands (e.g., `docker compose up -d`, `cargo build`)

### Port Isolation (Incremental Replace)

Running multiple worktrees means running multiple dev servers, which causes port conflicts. kiri solves this with automatic port isolation:

| Source | Detection | Example |
|--------|-----------|---------|
| `.env*` files | `PORT=3000`, `DB_PORT=5432` | Transformed |
| `docker-compose.yml` | Host ports in `ports:` section | Transformed (container ports preserved) |
| `package.json` | `--port`, `-p` flags in scripts | Transformed |
| `Dockerfile` | `EXPOSE` directives | Reference only |

Each worktree gets unique ports from the 20000-39999 range. You can enable/disable individual variables and add custom regex rules for project-specific files.

### Worktree Lifecycle

| Action | Behavior |
|--------|----------|
| **Create** | Branch is created if it doesn't exist, worktree + new window opens |
| **Open** | Existing worktree runs init commands and opens in a new window |
| **Close** | Confirmation dialog → worktree directory is cleaned up |

### Settings

Configure worktree behavior from the Settings modal in the Worktree Panel:

- **Copy Patterns** — Glob patterns for files to copy (default: `**/.env*`)
- **Init Commands** — Commands to run after creation (auto-detected + custom)
- **Port Isolation** — Toggle and configure port assignment per variable

## Remote Access

Remote Access lets you monitor and control your kiri projects from any device — your phone, tablet, or another computer. Check what branches are active, which AI agents are running, and open or close projects remotely.

### Quick Start

1. Toggle **Remote Access** on the start screen (or press `Cmd+Shift+R`)
2. A QR code appears — scan it with your phone
3. The remote interface opens in your mobile browser
4. You can now see and control all open projects

### What You Can Do Remotely

- **View open projects** — See project names, branches, and worktree status
- **Monitor terminals** — See which processes are running (e.g., `claude`, `node`)
- **Open projects** — Launch projects from your recent history
- **Close projects** — Close project windows remotely

### How It Connects

Remote Access uses [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) to create a secure connection. `cloudflared` is required.

```bash
brew install cloudflared
```

**Quick Tunnel** (default, no account required):
- Generates a temporary `*.trycloudflare.com` URL
- URL changes each time you restart
- Works immediately after installing `cloudflared`

**Named Tunnel** (persistent URL):
- Uses your own domain with a fixed URL
- Requires a Cloudflare account and tunnel token
- Configure the token in Remote Access settings (`Cmd+Shift+R`)

### Security

- **Token Authentication** — Every URL includes a cryptographic UUID token
- **Constant-Time Validation** — Prevents timing attacks on token comparison
- **Home Directory Boundary** — Cannot open projects outside your home directory
- **No Sensitive Data** — Terminal content is never transmitted, only process names
- **TLS by Default** — Cloudflare Tunnel provides HTTPS automatically

## Installation

### Requirements

- Node.js 20.x or higher
- Rust (latest stable)
- macOS (Windows/Linux support planned)

### Download

Download the latest release from [Releases](https://github.com/your-username/kiri/releases).

### Build from Source

```bash
# Clone the repository
git clone https://github.com/your-username/kiri.git
cd kiri

# Install dependencies
npm install

# Start in development mode
npm run tauri dev

# Production build
npm run tauri build
```

## Tech Stack

| Layer | Technology | Why |
|-------|------------|-----|
| Framework | [Tauri 2.x](https://tauri.app/) | 10x smaller than Electron |
| Frontend | [Svelte 5](https://svelte.dev/) + TypeScript | Near-zero runtime overhead |
| Backend | Rust | Native speed for file/git operations |
| Terminal | [xterm.js](https://xtermjs.org/) | Lightweight terminal emulator |
| File Viewer | [CodeMirror 6](https://codemirror.net/) | Modular, lazy-loadable |
| Git | [git2](https://github.com/rust-lang/git2-rs) | Native Rust bindings |
| Remote Access | [Axum](https://github.com/tokio-rs/axum) + WebSocket | Async HTTP/WS server for remote control |

## Performance

| Metric | Target |
|--------|--------|
| App Size | < 10 MB |
| Startup Time | < 1 second |
| Memory (idle) | < 50 MB |

## Contributing

Issues and Pull Requests are welcome!

### Development Commands

| Command | Description |
|---------|-------------|
| `npm run tauri dev` | Start development |
| `npm run tauri build` | Production build |
| `npm run test` | Run tests |
| `npm run lint` | Lint code |
| `npm run format` | Format code |

### Before Contributing

1. Fork this repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Create a Pull Request

## License

MIT License - See [LICENSE](LICENSE) for details

## Acknowledgments

- [Tauri](https://tauri.app/) — Lightweight desktop app framework
- [Svelte](https://svelte.dev/) — Compiler-based UI framework
- [xterm.js](https://xtermjs.org/) — Terminal emulator for the web
- [CodeMirror](https://codemirror.net/) — Extensible code editor
- [Axum](https://github.com/tokio-rs/axum) — Ergonomic web framework for Rust
- [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) — Secure tunnel for remote access
