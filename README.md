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

## Roadmap

### v0.0.1 (Current)
- [ ] Basic window structure
- [ ] Sidebar + file tree
- [ ] Terminal (single tab)
- [ ] Editor (single tab)
- [ ] Mode switching (Terminal ↔ Editor)

### v0.0.2
- [ ] Multiple tabs (terminal/editor)
- [ ] Git diff display

### v0.0.3
- [ ] Search functionality
- [ ] Multiple windows

### v0.1.0
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
