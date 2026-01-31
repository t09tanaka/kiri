# kiri 霧

> Light as mist, with only the features you need.

**A terminal-first development environment built for AI Coding Agents.**

Designed for developers who work with Claude Code, Aider, and other AI coding assistants. Seamlessly manage multiple projects with Git Worktree integration and multi-window support.

## Why kiri?

Modern AI coding agents like Claude Code run in the terminal, generating rapid output with spinners and progress bars. Traditional terminal emulators struggle with this, causing flickering and rendering glitches. kiri solves these problems while providing essential features for agent-assisted development:

- **Flicker-free terminal** — Optimized for Ink-based CLI tools
- **Multi-window workflow** — Work on multiple branches simultaneously
- **Git Worktree integration** — One-click branch isolation
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

### 🪟 Multi-Window & Git Worktree

Parallel development made simple:

| Feature | Description |
|---------|-------------|
| **Multiple Windows** | `Cmd+Shift+N` opens a new window |
| **Git Worktree Panel** | `Cmd+Shift+W` to create/manage worktrees |
| **Auto Window Creation** | New worktree → new window automatically |
| **Branch Isolation** | Work on features without stashing or switching |
| **Recent Projects** | Quick access to your project history |

### 💻 Terminal

| Feature | Description |
|---------|-------------|
| **Multiple Tabs** | `` Cmd+` `` for new terminal, `Cmd+1-9` to switch |
| **Pane Splitting** | Split horizontally or vertically |
| **Zoom Control** | `Cmd+`/`-` to adjust font size |
| **Independent Sessions** | Each pane runs its own PTY |

### 📝 Editor

| Feature | Description |
|---------|-------------|
| **Syntax Highlighting** | 20+ languages (TypeScript, Rust, Svelte, etc.) |
| **Git Diff Gutter** | See changes inline as you edit |
| **Diff View Modal** | Full diff comparison view |
| **Copy Button** | One-click code copying |
| **Quick Open** | `Cmd+P` for fuzzy file search |

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
| `Cmd+P` | Quick Open |
| `Cmd+B` | Toggle Sidebar |
| `Cmd+Shift+N` | New Window |
| `Cmd+Shift+W` | Git Worktrees |
| `` Cmd+` `` | New Terminal |
| `Cmd+W` | Close Tab |
| `Cmd+S` | Save File |
| `?` | Show All Shortcuts |

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
| Editor | [CodeMirror 6](https://codemirror.net/) | Modular, lazy-loadable |
| Git | [git2](https://github.com/rust-lang/git2-rs) | Native Rust bindings |

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
