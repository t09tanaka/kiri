---
name: kiri-cli
description: Use this skill when you are inside a kiri terminal (shell has KIRI_TERMINAL=1 env var) and need to inspect, split, close, or run commands across the kiri app's terminal panes via the `kiri` CLI. Covers `kiri term ls/run/send/read/follow/cancel/split/close/status/signal`, opening windows for a directory (`kiri window open`), targeting another window (`--window`), reading what an agent in a pane is doing (`kiri term status`), JSON output schema, pane addressing (index/id/focused), required pane labels (`--name`/`--color`), parent↔child signal queues (`kiri term signal send/wait/list`), busy-pane detection, and known limitations.
version: 0.5.0
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
| `--window PROJECT` | Target a **different** window than this one (see below) |

Agents should omit `--pretty`. JSON output is the default and is parseable.

### `--window PROJECT` — target another window

By default every `term` command acts on **this terminal's own window**
(the window open for the current project). Pass `--window PROJECT` to
retarget any `term` subcommand at a different window, addressed by its
open project's **path** or that path's **basename**:

```bash
kiri term ls --window other-project
kiri term send --window /abs/path/to/proj $'echo hi\n'
```

It matches a live window whose open project equals the value or whose
project directory's basename equals it, overriding the normal
current-project guard. If two windows have the same project open the
match is ambiguous and rejected — set `KIRI_SOCKET` explicitly to one of
them instead. `--window` has no effect on `kiri window …` (those reach
the shared app and pick any live window automatically).

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
      "name": "build",
      "color": "coral",
      "ai_kind": "claude"
    }
  ]
}
```

`running: true` means a foreground process is active (not the shell itself). `focused: true` marks the pane the user is currently viewing. `name` and `color` are **omitted entirely** when the pane was created without label flags — do not expect `null`. `ai_kind` is `"claude"` or `"codex"` when the pane's foreground process is a known interactive AI assistant, and is **omitted entirely** otherwise — a cheap way to spot agent panes. To see what such an agent is actually doing, use `kiri term status`.

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

### `kiri term status [--pane X] [--lines N]`

Snapshot a pane's **current on-screen text** — the most reliable way to
see what an agent (claude / codex) running in a pane is doing right now.
Reads the live screen from the frontend, so it works even on an idle
pane that `read` would return nothing for (the ring buffer only captures
output from the first `read`/`follow` onward).

```bash
kiri term status
kiri term status --pane pane-2
kiri term status --pane pane-2 --lines 80
```

- `--lines N` — number of trailing non-blank screen rows to return
  (default 40, max 200).

Response shape:

```json
{
  "type": "agent_status",
  "kind": "claude",
  "busy": true,
  "screen": "✶ Forging… (esc to interrupt · 12s · ↑ 3.2k tokens)\n> "
}
```

- `kind` — `"claude"`, `"codex"`, or `"none"` (from the foreground process).
- `busy` — best-effort: `true` while the agent shows an "esc to interrupt"
  affordance. Treat `screen` as the source of truth and interpret it
  yourself; `busy` is only a hint.
- `screen` — the visible text. Read it directly to judge whether the
  agent is thinking, awaiting input, running a tool, or done.

---

### `kiri term split [--pane X] [--dir h|v] --name STR --color COLOR`

Split the pane. `--dir h` (default) is horizontal; `--dir v` is vertical.

```bash
kiri term split --dir v --name build --color coral
kiri term split --dir v --name agent --color iris
```

Response shape:

```json
{
  "type": "split",
  "new_pane_id": "pane-4",
  "new_pane_index": 2
}
```

**Required label flags** (both must be supplied at split time — there is no
way to rename or recolor an existing pane):

- `--name STR` — 1–32 characters, no control characters. Shown as text
  in the pane's header.
- `--color COLOR` — one of `sky | iris | jade | amber | coral | rose`.
  Shown as a colored dot to the left of the name. Anything else is
  rejected by the CLI.

Omitting either flag causes clap to exit immediately with a usage error
(non-zero exit code).

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

---

### `kiri window open --dir PATH [--new]`

Open a kiri **window** for a directory (this is a `kiri window …` command,
not `kiri term …`). If a window is already open for that directory it is
focused instead of duplicated; pass `--new` to force a brand-new window.

```bash
kiri window open --dir ~/code/other-project
kiri window open --dir ~/code/other-project --new
```

Response shape:

```json
{
  "type": "open_window",
  "label": "window-3",
  "socket": "/Users/user/.kiri/instances/window-3.sock",
  "project": "/Users/user/code/other-project",
  "created": true,
  "socket_ready": true
}
```

- `created` — `true` if a new window was made, `false` if an existing one
  was focused.
- `socket_ready` — `true` once the new window's CLI socket accepts
  connections. For a freshly created window it may be `false` (still
  booting); retry against `socket` before sending commands.
- Requires at least one kiri window already running (the request is relayed
  through any live window to reach the app). Fails with `no_kiri_window`
  otherwise.

To act on the window you just opened, use the returned `socket`
directly — it is unambiguous even if several windows share a project name:

```bash
SOCK=$(kiri window open --dir ~/code/other-project | jq -r .socket)
KIRI_SOCKET="$SOCK" kiri term send $'echo hi\n'
```

---

### `kiri term signal …` (parent ↔ child messaging)

Each pane has its own FIFO queue of named signals. Two panes that have a
parent → child relationship (the child was created by `split`-ing the
parent) can exchange named messages to coordinate work — e.g. a sub-agent
claude in a side pane reports `done` to the orchestrator claude in the
parent pane.

Three subcommands:

#### `kiri term signal send`

Enqueue a signal on one or more pane queues.

```bash
# To a specific pane (by index or id):
kiri term signal send --pane pane-2 --name ready

# To the sender pane's parent (the pane that split-ed it):
kiri term signal send --target parent --name done --data '{"step":3}'

# Fan out to every pane the sender has spawned:
kiri term signal send --target children --name shutdown
```

Exactly one of `--pane` and `--target` must be set. Use `--from <ref>` to
override the sender pane (defaults to the focused pane); this matters when
the sender pane is not the focused one — e.g. an agent working in a
non-focused side pane that wants its `--target parent` resolution to use
its own id.

| Flag | Required? | Notes |
|---|---|---|
| `--pane I_OR_ID` | one of pane/target | Deliver to this single pane |
| `--target parent\|children` | one of pane/target | Route via the sender pane's parent/child links |
| `--from I_OR_ID` | optional | Sender pane override; defaults to focused |
| `--name STR` | yes | 1–64 chars, `[a-zA-Z0-9_.-]` only |
| `--data JSON` | optional | Any JSON value; delivered verbatim |

Response shape:

```json
{ "type": "signal_send", "delivered": 1 }
```

`delivered` counts how many pane queues received the signal. `0` means
the route resolved to no pane (no parent on the root pane, or no
children). Specifying a non-existent `--pane` returns
`error: pane_not_found` instead.

#### `kiri term signal wait`

Block the calling pane until a signal with `name` arrives on `--pane`'s
queue (defaults to focused), or until the timeout elapses.

```bash
# Default 60s timeout, focused pane:
kiri term signal wait --name ready

# 120s timeout, print the JSON data payload to stdout on success:
kiri term signal wait --name done --timeout 120 --print-data

# Wait on a specific pane (rarely needed — usually you wait on yourself):
kiri term signal wait --pane pane-2 --name ready
```

| Flag | Required? | Notes |
|---|---|---|
| `--pane I_OR_ID` | optional | Defaults to focused |
| `--name STR` | yes | Same character rules as `send` |
| `--timeout SECS` | optional | Default 60, max 600 |
| `--print-data` | optional | Print the JSON `data` field after the response line |

Success response shape:

```json
{
  "type": "signal_wait",
  "name": "done",
  "data": { "step": 3 },
  "sender_pane_id": "pane-3",
  "sent_at_ms": 1716729123456
}
```

`data` is **omitted entirely** when the signal had no payload — do not
expect a `null`.

Timeout response shape (non-zero CLI exit code):

```json
{
  "type": "error",
  "code": "timeout",
  "message": "no signal named 'done' arrived within 60s",
  "detail": { "timeout_secs": 60, "name": "done" }
}
```

#### `kiri term signal list`

Non-blocking peek at the signals currently queued on `--pane` (defaults
to focused). Does not consume; useful for debugging.

```bash
kiri term signal list
kiri term signal list --pane pane-2
```

Response shape:

```json
{
  "type": "signal_list",
  "signals": [
    {
      "name": "step",
      "data": { "n": 1 },
      "sender_pane_id": "pane-1",
      "sent_at_ms": 1716729123456
    }
  ]
}
```

#### Lifecycle and routing notes

- Parent/child links are recorded only when a pane is created via
  `kiri term split` (or the UI equivalent). The root pane has no parent.
- When a pane closes, its queue and parent/child links are torn down
  automatically. Any `signal wait` still blocked on that pane will fall
  through to its outer timeout.
- A signal sent before any waiter exists is queued — `wait` consumes
  the oldest matching entry whenever it arrives, even if that entry was
  enqueued seconds before the wait started. This is the standard
  FIFO-by-name behavior agents expect.
- Multiple waiters on the same `name` race for each signal; exactly one
  wins per send. Don't rely on broadcast-by-name within a single pane.

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
| `invalid_argument` | Validation failed (bad name characters, oversize, etc.) |
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
      "focused": true
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
      "name": "agent",
      "color": "iris",
      "ai_kind": "claude"
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
- When spawning a new pane via `kiri term split`, always pass
  `--name <purpose> --color <color>` so the pane is labeled in its header
  — e.g. `kiri term split --dir v --name <purpose> --color <color>` for
  agent-owned side panes.
- For parent ↔ child coordination, prefer `kiri term signal send/wait`
  over polling files or terminal output. The wait is `Notify`-backed
  on the server side, so it wakes immediately on send and respects an
  upper bound timeout (default 60s, max 600s).

## 9. Known limitations (v1)

- `follow` is currently a snapshot, not true streaming. Treat it as a one-shot tail.
- PTY output includes ANSI color/cursor escape codes. Strip them before semantic analysis.
- Sentinel-based `run` requires a normal interactive shell. If the pane is inside `vim`, `less`, or another TUI, use `send` + `cancel` instead.
- Each kiri window has its own CLI socket and panes. `term` commands act on
  this window by default; use `--window PROJECT` to address another window,
  or `kiri window open` to open/focus one. To act on a window precisely
  (e.g. one just opened), set `KIRI_SOCKET` to its socket.
- Signal routing assumes the sender pane is the focused pane. Use
  `signal send --from <ref>` to override when sending from a non-focused
  side pane.

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

# 4. Split pane-1 to get a clean side pane
kiri term split --pane pane-1 --dir v --name tests --color iris
# => { "type": "split", "new_pane_id": "pane-3", "new_pane_index": 2 }

# 5. Run tests in the new clean pane
kiri term run --pane pane-3 npm test

# 6. Notify the parent pane that the child is done
kiri term signal send --from pane-3 --target parent --name tests_done --data '{"exit":0}'

# 7. Parent pane is blocking on:
#    kiri term signal wait --name tests_done --timeout 600 --print-data
#    → unblocks and prints the JSON payload.
```
