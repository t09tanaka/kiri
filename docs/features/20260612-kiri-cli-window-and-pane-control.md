# kiri CLI: Multi-Window & Multi-Pane Control

Status: Design
Date: 2026-06-12

## Summary

Extend the `kiri` CLI beyond "operate on my own window" into three new
capabilities:

1. **Open a new window for a directory** — `kiri window open --dir <path>`.
2. **Operate on a *different* window** — a global `--window <project>`
   selector that retargets every existing `term` subcommand at another
   window's socket.
3. **Inspect what an agent in a pane is doing** — `kiri term status`,
   which returns the pane's current on-screen text (plus a light
   `kind`/`busy` structure) by snapshotting the frontend's xterm buffer.

All three reuse the existing per-window UDS + frontend-bridge
architecture. No new transport, no new socket model.

## Background: current architecture

- Each kiri window owns a Unix Domain Socket at
  `~/.kiri/instances/<label>.sock` (`cli_server::spawn_for_window`).
- Every `term` subcommand resolves to exactly **one** socket via
  `resolve_socket()` (priority: `$KIRI_SOCKET` → project-matched
  discovery). All handlers therefore act on that one window.
- `kiri env` already discovers every live window (label, project,
  socket) by scanning the instances dir and sending `WhoAmI`.
- Window creation already exists host-side: `create_window_impl`
  (accepts `project_path`, builds a `WebviewWindowBuilder`).
- UI/main-thread operations (`split`, `close`, `set-label`, `minimize`)
  round-trip through the frontend: a handler `emit_to`s a `cli:*` event,
  the frontend acts and calls back `cli_resolve_pending`. The handler
  awaits a `PendingReplies` oneshot with a timeout.
- The frontend `terminalRegistry` maps `paneId → xterm Terminal`, so the
  frontend can read any pane's live screen via `terminal.buffer.active`.
- The backend ring buffer (`Read`/`Follow`) only captures bytes **from
  first subscription forward** — it does not backfill scrollback, which
  is why reading a freshly-touched idle pane returns nothing.

## Feature 1 — `kiri window open --dir <path>`

### CLI surface

```
kiri window open --dir <path> [--new]
```

- New top-level `window` command namespace with an `open` subcommand.
- `--dir <path>` (required): directory to open. Existence is validated;
  git is **not** required.
- `--new`: force a brand-new window even if one is already open for the
  directory.

Default behavior = focus-or-create: if a window already has this
directory open, focus it; otherwise create a new one. (Mirrors the
existing `focus_or_create_window` command.)

### Socket resolution for `window open`

`window open` is an **app-global** request, not a per-window one, so it
must **bypass the project guard** in `resolve_socket()` and relay through
**any live window's socket**. If zero windows are alive, it errors
clearly (`no_kiri_window`) — the app must already be running, since the
request needs a live `AppHandle` to reach.

### Protocol

```rust
// Request
OpenWindow { dir: String, force_new: bool }

// Response
OpenWindow {
    label: String,        // new (or focused) window label, e.g. "window-3"
    socket: String,       // ~/.kiri/instances/<label>.sock
    project: String,      // canonicalized dir
    created: bool,        // true = newly created, false = focused existing
    socket_ready: bool,   // true = socket accepted a connection before timeout
}
```

### Host handler (`open_window`)

Runs inside `handlers::handle`. Pulls app state directly — **no
`DispatchContext` change needed**:

1. `let app = ctx.app`; `app.state::<WindowRegistryState>()`.
2. Canonicalize `dir`; if it does not exist → `InvalidArgument`.
3. If `!force_new` and the registry has a label for this path **and** the
   webview still exists → `app.run_on_main_thread(set_focus)`,
   `created = false`, reuse its label.
4. Else → `app.run_on_main_thread(create_window_impl(project_path = dir))`,
   `created = true`. **`create_window_impl` is refactored to return the
   generated `label`** (currently returns `()`).
5. `socket = socket_path_for(label)` (deterministic).
6. Poll `socket_alive(socket)` up to **10s** (a new window must boot its
   webview → call `register_window` → start its `cli_server`). Set
   `socket_ready` accordingly; return regardless so the caller always
   gets `label`/`socket`.

`WebviewWindowBuilder::build()` requires the main thread, hence
`run_on_main_thread`. This matches how `menu.rs` already creates windows.

### Open → operate handoff

The returned `socket` is the precise handle for immediate follow-up,
avoiding any project-name ambiguity:

```bash
SOCK=$(kiri window open --dir ~/foo | jq -r .socket)
KIRI_SOCKET="$SOCK" kiri term send 'hello'
```

## Feature 2 — global `--window <project>` selector

### CLI surface

A global flag on `Cli`, usable with any `term` subcommand:

```
kiri term ls   --window myproj
kiri term send --window myproj 'hi'
kiri term status --window /abs/path/to/proj
```

```rust
#[arg(long, global = true)]
pub window: Option<String>,
```

### Resolution

When `--window` is set, `resolve_socket()` is **bypassed**:

1. Scan `~/.kiri/instances/*.sock` for live sockets.
2. `WhoAmI` each → `(label, project_path)`.
3. Match `--window` against each window's project: exact path match, or
   basename (project-name) match. This is **intentionally allowed to
   target a window for a different project than the cwd** — that is the
   whole point of the selector, and it overrides the normal project
   guard.
4. Exactly one match → use it. Zero → `no_kiri_window` ("no window open
   for '<arg>'"). Multiple (same project open twice) → `InvalidArgument`
   listing the candidate labels and instructing the user to set
   `KIRI_SOCKET=<socket>` explicitly (the disambiguation escape hatch).

Per the prior decision, `--window` matches **project name / path only**,
not internal labels. For the precise "operate on the window I just
opened" flow, use the `socket` returned by `window open` (above).

## Feature 3 — `kiri term status` (agent / pane activity)

### Problem

`kiri term ls` reports `process_name` (`"claude"`/`"codex"`),
`running`, `memory_bytes`, `cwd` — enough to know *an agent is present*,
but not *what it is doing*. And `read` on a fresh idle pane returns
nothing because the ring buffer only captures forward from first
subscription. We want a reliable "what is on this pane right now".

### Approach: frontend screen snapshot

The frontend xterm `Terminal` holds the live, rendered screen. We
snapshot its visible buffer — this returns exactly what is displayed
(idle prompt box *or* busy status line), independent of ring-buffer
subscription timing. `xterm`'s `buffer.active.getLine(i).translateToString()`
yields **plain text** (escape sequences already decoded into cells), so
**no Rust-side ANSI stripping is required**.

### CLI surface

```
kiri term status [--pane <i|id>] [--window <project>] [--lines <N>]
```

- `--lines N`: number of trailing non-blank screen rows to return.
  Default 40, clamped to a max (e.g. 200).

### Protocol

```rust
// Request
AgentStatus { pane: PaneRef, lines: usize }

// Response
AgentStatus {
    kind: String,     // "claude" | "codex" | "none" (from process_name)
    busy: bool,       // best-effort, light heuristic — see below
    screen: String,   // current visible buffer, trailing N non-blank rows
}
```

`screen` is the source of truth; `kind`/`busy` are convenience signals.
Semantic interpretation (spinner verb, token count, "what step is it
on") is left to the **caller** (typically an LLM agent), which reads
`screen`. This keeps us robust to claude/codex TUI changes.

### Host handler (`agent_status`)

1. Resolve pane via `pane_map`; `kind` from the cheap
   `process_info_for` lookup (reuse `is_ai_process_for_shell_pid`
   allow-list: `claude`/`codex`, else `none`).
2. Frontend-bridge round-trip (same pattern as `split`): register a
   `PendingReplies` waiter, `emit_to(label, "cli:pane-snapshot",
   { requestId, paneId, lines })`, await with a 2s timeout. Frontend
   reads the pane's xterm buffer, collects the last `lines` non-blank
   rows, replies via `cli_resolve_pending` with `{ screen }`. On timeout
   / missing pane instance → `FrontendUnresponsive`.
3. `busy`: light, pure, **testable** heuristic computed in Rust from
   `screen` (e.g. presence of an "esc to interrupt" affordance for
   claude/codex). Best-effort only; never blocks on correctness.

### Frontend bridge addition

`cliBridge.ts` gains a `cli:pane-snapshot` listener alongside the
existing `cli:pane-split` / `-close` / `-minimize` handlers: look up the
`Terminal` in `terminalRegistry` by `paneId`, walk
`terminal.buffer.active` from the bottom collecting non-blank
`translateToString()` rows up to `lines`, join, and
`cli_resolve_pending({ screen })`. If no instance is registered, reply
with `{ error: "no_focused_pane" }`-style payload so the handler maps it
to a clean error.

### Cheap `ls` enrichment

Add `ai_kind: Option<String>` to `PaneInfo`, derived from the existing
`process_name` lookup (no extra round-trip). This lets a single
`kiri term ls` show which panes are agents. `busy` is **not** added to
`ls` — it requires a per-pane frontend snapshot, too heavy for a list.

## Edge cases & notes

- **App not running / zero windows**: `window open` and `--window` both
  fail with `no_kiri_window` — there is no socket/`AppHandle` to relay
  through.
- **New-window socket startup lag**: `window open` returns
  `socket_ready: false` if the new window's `cli_server` is not up
  within 10s; immediate follow-up commands should retry.
- **`--window` ambiguity** (same project open in two windows):
  `InvalidArgument` + candidate labels + `KIRI_SOCKET` guidance.
- **`--dir` outside a git repo / nonexistent**: only existence is
  checked. `current_project_root` is used solely for the cwd project
  guard, not here.
- **`status` on a non-AI pane** (`kind: "none"`): still returns the
  screen snapshot — useful for any pane, not just agents.
- **`status` frontend round-trip**: 2s timeout like `split`; a pane with
  no registered xterm instance yields `FrontendUnresponsive`.
- **`run_on_main_thread`**: required for `WebviewWindowBuilder::build()`;
  keep consistent with `menu.rs`'s existing window-creation path.
- **Testing** (per `.claude/rules/testing.md`): `cli.rs` and `wire.rs`
  are table-test-heavy. Add parse/build tests for the new `window open`
  command, the global `--window` flag, and `term status`; add wire
  round-trip tests for `OpenWindow` / `AgentStatus`. The `busy`
  heuristic and the `--window` matcher are pure functions → unit-tested.
  Frontend `cli:pane-snapshot` handling → `cliBridge.test.ts`. Anything
  requiring a real `AppHandle`/window (the `open_window` main-thread
  path, the xterm buffer read) → E2E.

## Out of scope (YAGNI)

- Backfilling the ring buffer from frontend scrollback for `read`
  (the `status` snapshot covers the "current screen" need; a general
  scrollback backfill is a separate, larger change).
- `--window` matching by internal label (project name/path + returned
  socket cover the use cases).
- Streaming agent status / live activity follow.
