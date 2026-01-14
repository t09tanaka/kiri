# Kiri 霧

> Light as mist, with only the features you need.

A lightweight file manager + terminal application.

## Features

- **Lightweight**: Tauri-based, fast startup, low memory usage
- **Simple**: Focused on 3 core features - file tree, terminal, editor
- **Modern**: Built with Svelte 5 + TypeScript + Rust

## Screenshots

<!-- TODO: Add screenshots -->

## Installation

### Requirements

- Node.js 20.x or higher
- Rust (latest stable)
- macOS (as of v0.0.1)

### Build

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

## Development

### Commands

| Command | Description |
|---------|-------------|
| `npm run dev` | Start Vite dev server |
| `npm run tauri dev` | Start as Tauri app (development) |
| `npm run tauri build` | Production build |
| `npm run format` | Format code (Prettier) |
| `npm run lint` | Run linter (ESLint) |
| `npm run lint:fix` | Auto-fix lint issues |
| `npm run check` | TypeScript type check |

### Project Structure

```
kiri/
├── src/                    # Frontend (Svelte 5)
│   ├── lib/
│   │   ├── components/     # UI components
│   │   ├── stores/         # State management
│   │   └── utils/          # Utilities
│   ├── App.svelte
│   └── main.ts
├── src-tauri/              # Backend (Rust)
│   ├── src/
│   │   ├── main.rs
│   │   └── commands/       # Tauri commands
│   └── Cargo.toml
└── docs/                   # Documentation
```

## Tech Stack

| Layer | Technology |
|-------|------------|
| Framework | [Tauri 2.x](https://tauri.app/) |
| Frontend | [Svelte 5](https://svelte.dev/) + TypeScript |
| Backend | Rust |
| Bundler | [Vite](https://vitejs.dev/) |
| Editor | [CodeMirror 6](https://codemirror.net/) |
| Terminal | [xterm.js](https://xtermjs.org/) |

## Terminal: Ink-based CLI Compatibility

The terminal is optimized for [Ink](https://github.com/vadimdemedes/ink)-based CLI tools like Claude Code, Aider, and other AI coding agents.

### Implemented Countermeasures

| Issue | Solution |
|-------|----------|
| Partial frame rendering causes flickering | Synchronized Output Mode (DEC Private Mode 2026) buffering |
| Spinner/progress bar breaks at 140+ columns | Terminal width capped at 120 columns |
| Full-height rendering causes unwanted scrolling | PTY rows reduced by 1 (Ink issue [#450](https://github.com/vadimdemedes/ink/issues/450)) |
| Resize during output causes artifacts | Resize buffering with delayed flush |
| Incorrect initial size breaks layout | Layout completion waiting before PTY creation |

### Why These Are Needed

xterm.js doesn't natively support Synchronized Output Mode (DEC 2026), which Ink uses to batch screen updates. Without manual buffering, rapid output from AI agents appears as chaotic flickering instead of smooth animations.

## Roadmap

### v0.0.1 (Current)
- [x] Basic window structure
- [x] Sidebar + file tree
- [x] Terminal (multiple tabs, pane splitting)
- [x] Editor (multiple tabs, syntax highlighting)
- [x] Mode switching (Terminal ↔ Editor)
- [x] Multiple tabs (terminal/editor)
- [x] Git diff display (gutter + DiffView window)
- [x] Search functionality (Quick Open + content search)
- [x] Multiple windows
- [ ] File operations (new/delete/rename)
- [ ] Settings screen
- [ ] Windows/Linux support

## Contributing

Issues and Pull Requests are welcome.

### Before Contributing

1. Fork this repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Create a Pull Request

### Coding Standards

- Formatting: Prettier (`npm run format`)
- Linting: ESLint (`npm run lint`)
- Run `npm run check` before committing

## License

MIT License - See [LICENSE](LICENSE) for details

## Acknowledgments

- [Tauri](https://tauri.app/) - Lightweight desktop app framework
- [Svelte](https://svelte.dev/) - Compiler-based UI framework
