---
name: kiri-cli
description: Use this skill when you are inside a kiri terminal (shell has KIRI_TERMINAL=1 env var) and need to inspect, split, close, or run commands across the kiri app's terminal panes via the `kiri` CLI. Covers `kiri term ls/run/send/read/follow/cancel/split/close`, JSON output schema, pane addressing (index/id/focused), busy-pane detection, and known limitations.
version: 0.1.0
---

# kiri CLI skill

## 1. When to use this skill

Only inside a kiri terminal. Verify with:

```bash
echo $KIRI_TERMINAL   # should print 1
command -v kiri       # should resolve to ~/.kiri/bin/kiri
```

Outside a kiri terminal, `kiri` is not on PATH and `KIRI_SOCKET` is unset — all commands will fail.

## 2. Environment

| Variable | Value | Purpose |
|---|---|---|
| `KIRI_TERMINAL` | `1` | Marker that this PTY was spawned by kiri |
| `KIRI_WINDOW_LABEL` | e.g. `main` | Tauri window label this terminal lives in |
| `KIRI_SOCKET` | e.g. `/tmp/kiri-abc123.sock` | UDS path the CLI uses to reach the backend. If stale (kiri restarted), the CLI auto-falls back to discovery in `~/.kiri/instances/*.sock` |

## 3. Global flags

| Flag | Effect |
|---|---|
| `--pretty` | Human-readable output instead of JSON |

Agents should omit `--pretty`. JSON output is the default and is parseable.

## 4. Pane addressing (`PaneRef`)

Every subcommand except `ls` accepts `--pane`:

- **Omit** — operates on the focused pane (the one the user is looking at); wire sentinel: `"focused"`
- **Integer** — e.g. `--pane 0` — depth-first index from `ls`; serializes as a JSON number
- **String** — e.g. `--pane pane-3` — stable pane id; serializes as a JSON string

Prefer `--pane <id>` over `--pane <index>` once you have the id; indices renumber when panes close.

## 5. Subcommands

### `kiri term ls`

Discover all panes in this window. Always run this first.

```bash
kiri term ls
```

Response shape:

```json
{
  "type": "ls",
  "panes": [
    {
      "index": 0,
      "id": "pane-1",
      "terminal_id": 1,
      "cwd": "/Users/user/project",
      "process_name": "zsh",
      "running": false,
      "memory_bytes": 4096000,
      "focused": true,
      "minimized": false,
      "name": "build",
      "color": "coral"
    }
  ]
}
```

`running: true` means a foreground process is active (not the shell itself). `focused: true` marks the pane the user is currently viewing. `name` and `color` are **omitted entirely** when the pane was created without label flags — do not expect `null`.

---

### `kiri term run [--pane X] [--timeout SECS] [--full] CMD...`

Run a command in the pane and block until completion. Output is captured via sentinel detection.

```bash
kiri term run git status
kiri term run --pane pane-2 --timeout 60 npm test
kiri term run --full cargo build
```

Response shape:

```json
{
  "type": "run",
  "exit_code": 0,
  "output": "On branch main\nnothing to commit",
  "output_truncated": false,
  "lines_omitted": 0,
  "timed_out": false,
  "cursor": 8192
}
```

- `exit_code: null` — timed out or killed by signal
- `output_truncated: true` / `lines_omitted > 0` — last 1000 lines returned; re-run with `--full` to get everything
- `cursor` — ring-buffer position; pass to `read --since` for incremental reads
- Default timeout: 300 seconds

Do NOT use `run` for long-lived processes (`npm run dev`, `docker compose up`, `tail -f`). Use `send` + `read`/`follow` instead.

---

### `kiri term send [--pane X] DATA...`

Write raw bytes to the PTY immediately (no waiting). Use `$'...'` shell quoting for escape sequences.

```bash
kiri term send $'yes\n'
kiri term send --pane pane-2 $'q'
kiri term send npm run dev
```

Response shape:

```json
{ "type": "send" }
```

Use this to: start long-running processes you will then `read`/`follow`, answer interactive prompts, or navigate TUI apps.

---

### `kiri term read [--pane X] [--since CURSOR] [--tail N]`

Pull bytes from the pane's ring buffer. `--since` and `--tail` are mutually exclusive.

```bash
kiri term read --pane pane-1
kiri term read --pane pane-1 --since 8192
kiri term read --pane pane-1 --tail 50
```

Response shape:

```json
{
  "type": "read",
  "output": "... recent terminal output ...",
  "cursor": 9500,
  "bytes_dropped": 0
}
```

- `bytes_dropped > 0` — ring buffer overflowed since `since`; some output was lost
- Use `--since CURSOR` to incrementally collect new output without re-reading what you already have

---

### `kiri term follow [--pane X]`

Stream output from a pane until the connection closes. Frames:

```json
{ "type": "follow_chunk", "data": "...", "cursor": 9600 }
{ "type": "follow_end" }
```

**Known v1 limitation**: `follow` currently emits a snapshot then ends — treat it as a one-shot tail, not a true `tail -f`.

---

### `kiri term cancel [--pane X]`

Send Ctrl-C (SIGINT) to the foreground process.

```bash
kiri term cancel
kiri term cancel --pane pane-2
```

Response shape:

```json
{ "type": "cancel" }
```

---

### `kiri term split [--pane X] [--dir h|v] [--name STR] [--color COLOR] [--minimized]`

Split the pane. `--dir h` (default) is horizontal; `--dir v` is vertical.
`--minimized` creates the new pane with its shortcut bar already
collapsed — useful when the agent is spawning a side pane for its own
use and does not want to push the user's primary view down.

```bash
kiri term split
kiri term split --dir v
kiri term split --pane pane-1 --dir h
kiri term split --dir v --minimized
kiri term split --name build --color coral
kiri term split --name agent --color iris --minimized
```

Response shape:

```json
{
  "type": "split",
  "new_pane_id": "pane-4",
  "new_pane_index": 2
}
```

**Label flags** (both optional, both apply only at split time — there is no
way to rename or recolor an existing pane):

- `--name STR` — 1–32 characters, no control characters. Shown as text
  in the pane's header.
- `--color COLOR` — one of `sky | iris | jade | amber | coral | rose`.
  Shown as a colored dot to the left of the name. Anything else is
  rejected by the CLI.

Either, both, or neither may be supplied. A pane created without these
flags has no header label.

---

### `kiri term minimize [--pane X]`

Collapse the pane's shortcut bar to a thin strip with only restore and
settings buttons. The PTY itself is untouched — only the helper UI bar.

```bash
kiri term minimize
kiri term minimize --pane pane-2
```

Response shape:

```json
{ "type": "minimize" }
```

---

### `kiri term restore [--pane X]`

Expand a previously minimized shortcut bar back to its full layout.

```bash
kiri term restore --pane pane-2
```

Response shape:

```json
{ "type": "restore" }
```

---

### `kiri term close [--pane X]`

Close the pane.

```bash
kiri term close --pane pane-3
```

Response shape:

```json
{ "type": "close" }
```

## 6. Errors

All errors share this envelope. Keys `type`, `code`, and `message` are always present. `detail` is optional (arbitrary JSON value) and is **omitted entirely** when not populated — do not expect a `null` value.

Without `detail`:

```json
{
  "type": "error",
  "code": "pane_busy",
  "message": "Pane 0 is currently running a process"
}
```

With `detail`:

```json
{
  "type": "error",
  "code": "timeout",
  "message": "Command exceeded the 60s timeout",
  "detail": { "timeout_secs": 60, "cursor": 12288 }
}
```

Key error codes (snake_case):

| Code | Meaning |
|---|---|
| `pane_not_found` | No pane matches the given index or id |
| `pane_busy` | Pane has a foreground process; `run` refused |
| `timeout` | Operation exceeded the timeout |
| `pty_error` | PTY write/read failure |
| `frontend_unresponsive` | kiri frontend did not respond in time |
| `protocol_error` | Malformed message on the wire |
| `internal_error` | Unexpected server-side error |
| `no_kiri_window` | No kiri window found (socket discovery failed) |
| `cwd_outside_project` | Working directory is outside the project root |

`pane_busy` from `run` means the foreground process is not the shell. Options: (a) `cancel` if you own the process, (b) `split` to get a clean pane, (c) pick another pane from `ls` where `running: false`.

## 7. JSON examples

### `ls` response

```json
{
  "type": "ls",
  "panes": [
    {
      "index": 0,
      "id": "pane-1",
      "terminal_id": 1,
      "cwd": "/Users/user/project",
      "process_name": "zsh",
      "running": false,
      "memory_bytes": 5242880,
      "focused": true,
      "minimized": false
    },
    {
      "index": 1,
      "id": "pane-2",
      "terminal_id": 2,
      "cwd": "/Users/user/project",
      "process_name": "npm",
      "running": true,
      "memory_bytes": 52428800,
      "focused": false,
      "minimized": true,
      "name": "agent",
      "color": "iris"
    }
  ]
}
```

### `run` response

```json
{
  "type": "run",
  "exit_code": 0,
  "output": "M  src/App.svelte\n?? src/lib/new-file.ts\n",
  "output_truncated": false,
  "lines_omitted": 0,
  "timed_out": false,
  "cursor": 16384
}
```

### Error response

```json
{
  "type": "error",
  "code": "pane_busy",
  "message": "Pane pane-2 is running 'npm'; use cancel or split to get a free pane"
}
```

## 8. Best practices for agents

- Always start with `kiri term ls`. Never assume an index.
- Prefer `--pane <id>` over `--pane <index>` once you have the id — indices renumber when panes close.
- For long-lived processes (`docker compose up`, dev servers, `tail -f`): use `send` to start, then `read --since CURSOR` or `follow` to observe. Do not use `run` (blocks until completion).
- Use `read --since CURSOR` to incrementally collect output without re-reading what you already have.
- On `pane_busy`: (a) `cancel` if you own the process, (b) `split` for a clean pane, (c) pick a pane from `ls` with `running: false`.
- Surface `lines_omitted > 0` from `run` to the user — important context may be truncated. Re-run with `--full` if needed.
- When spawning a new pane via `kiri term split` for the agent's own
  use (background dev server, log tail, parallel run), prefer
  `--minimized` so the new pane comes up with its shortcut bar
  collapsed. This keeps the user's primary view from being pushed
  down. The user (or `kiri term restore --pane <id>`) can expand it
  at any time.

## 9. Known limitations (v1)

- `follow` is currently a snapshot, not true streaming. Treat it as a one-shot tail.
- PTY output includes ANSI color/cursor escape codes. Strip them before semantic analysis.
- Sentinel-based `run` requires a normal interactive shell. If the pane is inside `vim`, `less`, or another TUI, use `send` + `cancel` instead.
- Cross-window addressing is not supported. Each kiri window has its own CLI socket and panes.

## 10. Example workflow

```bash
# 1. Discover panes
kiri term ls

# Response shows pane-1 (zsh, idle, focused) and pane-2 (npm, running)

# 2. Try to run git status on focused pane (pane-1)
kiri term run git status

# Success — exit_code 0, output contains status lines

# 3. Try to run tests on pane-2 (which is busy)
kiri term run --pane pane-2 npm test
# => { "type": "error", "code": "pane_busy", ... }

# 4. Split pane-1 to get a clean pane
kiri term split --pane pane-1
# => { "type": "split", "new_pane_id": "pane-3", "new_pane_index": 2 }

# 5. Run tests in the new clean pane
kiri term run --pane pane-3 npm test
```
