# kiri 霧

> Light as mist, with only the features you need.

**A terminal for AI coding agents — one your agent can drive itself.**

When Claude Code or Codex runs in your terminal, it can output anything — but it can't *do* anything outside its own session. It can't spawn another pane to run tests in parallel. It can't peek at a file the user has open. It can't cancel a hung process and try again. Every action requires a "please run this and paste the output" round-trip with you.

kiri changes that. Inside any kiri terminal, the agent has a `kiri` command on PATH. With it, the agent can list panes, split the window, dispatch background work, tail output, and tear panes down — without leaving its session.

```
┌─────────────────────────────┬─────────────────────────────┐
│ pane 0 (focused)            │ pane 1                      │
│                             │                             │
│ $ claude                    │ $ npm test                  │
│ I'll run the tests next     │ ✓ 142 passed                │
│ door while I finish this    │ ✗ 3 failed                  │
│ refactor.                   │                             │
│                             │                             │
│ [splitting pane...]         │                             │
│ [running tests in pane 1]   │                             │
│                             │                             │
│ Tests done — 3 failures.    │                             │
│ Looking at the first one... │                             │
└─────────────────────────────┴─────────────────────────────┘
```

The agent did all of that with `kiri term split`, `kiri term send`, and `kiri term read` — from inside its own shell.

## Status

v0.4.0 — early but usable. **macOS Apple Silicon only** for now (binaries on every [release](https://github.com/t09tanaka/kiri/releases)). The CLI surface is stable; expect more verbs over time.

## What kiri is

A small, opinionated terminal-plus-file-viewer built around two ideas:

1. **The terminal must render agent output without breaking** — Ink frames, spinners, 200-lines-per-second log streams.
2. **The terminal must let the agent reach beyond itself** — other panes, files, processes, all addressable from inside the agent's own shell.

That's it. It is not a code editor (the file viewer is read-only on purpose). It is not an IDE. It does not manage projects or compete with VS Code. If your agent doesn't need to spawn helper panes or peek at files, you don't need kiri.

## The loop

A typical Claude-in-kiri session:

1. You type `claude` in pane 0. Claude inherits a shell with `kiri` on PATH.
2. Claude makes a change and wants to verify it.
3. `kiri term split --dir v` → pane 1 opens next to it.
4. `kiri term send --pane 1 npm test` → tests start; Claude doesn't block.
5. Claude keeps editing, polling `kiri term read --pane 1 --since CURSOR` for new output.
6. Tests fail. `kiri term cancel --pane 1`, fix the code, re-dispatch.
7. `kiri term close --pane 1`, report back.

Without the CLI, every step would have been a paste-the-output round-trip with you.

## CLI

`kiri` is installed at `~/.kiri/bin/kiri` on first launch. Inside kiri terminals it is on PATH automatically (via `KIRI_TERMINAL=1` and `KIRI_SOCKET=…`); outside one it errors out cleanly.

### Verbs

| Command | Blocks? | Use for |
|---------|---------|---------|
| `kiri term ls` | — | Discover panes (always do this first) |
| `kiri term run <cmd>` | yes, until exit | One-shot commands you want the exit code of |
| `kiri term send <data>` | no | Start long-running processes, answer prompts |
| `kiri term read [--tail N \| --since CURSOR]` | no | Pull from the pane's ring buffer |
| `kiri term follow` | yes, snapshot | One-shot tail (true streaming is on the roadmap) |
| `kiri term cancel` | no | Send Ctrl-C to the foreground process |
| `kiri term split [--dir h\|v]` | no | Open a sibling pane |
| `kiri term close` | no | Close a pane |

**`run` vs `send`** is the distinction that matters most. `run` is for things that finish (`git status`, `cargo build`) and gives you the exit code. `send` is for things that don't (`npm run dev`, `docker compose up`, replying to an interactive prompt). Mix them up and `run` will hang on its 5-minute default timeout.

### Pane addressing

Every verb except `ls` accepts `--pane`:

- omit → the focused pane (the one the user is looking at)
- `--pane 0` → by depth-first index (renumbers when panes close)
- `--pane pane-3` → by stable id (preferred once you have it)

### Output

JSON by default, `--pretty` for humans. The full schema, error envelope, and best-practice notes are in [`resources/skills/kiri-cli/SKILL.md`](resources/skills/kiri-cli/SKILL.md) — the same file kiri auto-installs into `~/.claude/skills/` so any Claude Code session can use the CLI correctly the first time.

### Errors

Consistent envelope, snake_case codes:

```json
{ "type": "error", "code": "pane_busy", "message": "Pane pane-2 is running 'npm'..." }
```

`pane_busy` is the common one — `run` refuses to dispatch into a pane that already has a foreground process. The agent's documented options: `cancel` if it owns the process, `split` for a clean pane, or pick a free pane from `ls`.

## What kiri does for you (the human)

Beyond hosting the agent, kiri is also a careful terminal-and-viewer:

- **Synchronized Output Mode (DEC 2026)** — Ink-based CLIs render without flicker
- **120-column cap** — spinners stop tearing on resize
- **Cmd+Click on `file.ts:42`** in any output → peek editor opens at that line
- **Cmd+P / Cmd+Shift+F / Cmd+D / Cmd+H** — quick open, content search, diff view, commit graph
- **Cmd+Shift+P** — pull request panel (via `gh`)
- **Cmd+Shift+R** — remote access; QR-code your phone into a live project monitor over Cloudflare Tunnel
- **Cmd+Shift+N** — new window; one process tree per project, fully isolated
- **Shortcut suggestions** — kiri learns the commands you type often and surfaces them

The file viewer is read-only on purpose. Editing is what the agent (or your real editor) is for.

## Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `Cmd+O` | Open Directory |
| `Cmd+P` | Quick Open |
| `Cmd+Shift+F` | Content Search |
| `Cmd+Shift+P` | Pull Requests |
| `Cmd+B` | Toggle Sidebar |
| `Cmd+D` | Diff View |
| `Cmd+H` | Commit History |
| `Cmd+Shift+N` | New Window |
| `Cmd+Shift+W` | Close Project |
| `Cmd+Shift+R` | Remote Access |
| `Cmd+/` | Show All Shortcuts |

## Remote Access

Toggle from the start screen (or `Cmd+Shift+R`); a QR code appears. Scan with your phone — your phone's browser shows every open kiri project, the branch each is on, and which AI agents are running. Tap to open or close projects.

The transport is [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) (`brew install cloudflared`). The default Quick Tunnel gives you a fresh `*.trycloudflare.com` URL each time, no account needed. For a persistent URL, configure a Named Tunnel token in settings.

Security: every URL carries a UUID auth token validated in constant time, the home directory is the hard project boundary, and only process names — never terminal content — leave the machine.

## Install

Pre-built binaries on the [Releases](https://github.com/t09tanaka/kiri/releases) page:

- macOS Apple Silicon (`aarch64.dmg`)

Other platforms (Intel Mac, Windows, Linux) are not currently shipped — build from source if you need them.

### Build from source

Requires Node.js 20+ and a recent stable Rust.

```bash
git clone https://github.com/t09tanaka/kiri.git
cd kiri
npm install
npm run tauri dev    # development
npm run tauri build  # production bundle
```

The kiri CLI binary is built as part of the Cargo workspace and installed into `~/.kiri/bin/` the first time the app launches.

## Architecture

The Tauri 2 host (Rust + Svelte 5) spins up a per-window Unix domain socket when the window registers. The `kiri` CLI binary speaks JSON over that socket; every PTY in the window has its env populated with `KIRI_SOCKET` and `KIRI_TERMINAL=1`, so the agent's shell already knows where to talk. Cross-window addressing is intentionally not supported — each kiri window is its own sandbox.

The server-side code is split into pure modules (`ring_buffer`, `run_logic`, `pane_map`, `frontend_bridge`) that unit-test without Tauri, plus a thin dispatcher and UDS listener that wire them to the frontend over Tauri events. Source lives in [`crates/kiri-cli`](crates/kiri-cli) (the binary), [`crates/kiri-cli-proto`](crates/kiri-cli-proto) (the wire types), and [`src-tauri/src/cli_server`](src-tauri/src) (the in-app server).

## Tech stack

| Layer | Technology |
|-------|------------|
| App framework | [Tauri 2.x](https://tauri.app/) |
| Frontend | [Svelte 5](https://svelte.dev/) + TypeScript |
| Backend | Rust |
| Terminal | [xterm.js](https://xtermjs.org/) |
| File viewer | [CodeMirror 6](https://codemirror.net/) |
| Git | [git2](https://github.com/rust-lang/git2-rs) |
| CLI transport | Unix Domain Sockets + [clap](https://github.com/clap-rs/clap) |
| Remote Access | [Axum](https://github.com/tokio-rs/axum) + WebSocket + [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/) |

## Performance budget

| Metric | Target |
|--------|--------|
| App size | < 10 MB |
| Cold start | < 1 s |
| Idle memory | < 50 MB |

CI fails the build if the bundle exceeds the size budget.

## Contributing

Issues and PRs welcome. Useful commands during development:

| Command | Description |
|---------|-------------|
| `npm run tauri dev` | Run the app in dev mode |
| `npm run test` | Frontend unit tests |
| `npm run test:rust` | Rust tests |
| `npm run lint` / `lint:fix` | ESLint + Svelte check |
| `npm run format` | Prettier + rustfmt |

See [`CLAUDE.md`](CLAUDE.md) for project conventions and [`.claude/rules/`](.claude/rules/) for the design rules CI enforces (testing policy, multi-window data flow, design tokens, etc.).

## License

MIT — see [LICENSE](LICENSE).

## Acknowledgments

[Tauri](https://tauri.app/) · [Svelte](https://svelte.dev/) · [xterm.js](https://xtermjs.org/) · [CodeMirror](https://codemirror.net/) · [Axum](https://github.com/tokio-rs/axum) · [clap](https://github.com/clap-rs/clap) · [git2](https://github.com/rust-lang/git2-rs) · [Cloudflare Tunnel](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/)
