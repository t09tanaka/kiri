# Terminal Panel Minimize / Restore

Status: Draft (awaiting user review)
Date: 2026-05-13
Owner: Takuto Tanaka

## Summary

Add the ability to collapse the per-terminal shortcut bar (the
`REPLY` / `CMD` / `PICK` rows that appears while `claude` or `codex`
is running) into a thin horizontal strip that shows only a restore
button and the existing settings button. Expose the same operation
through the `kiri` CLI so that agents and scripts can minimize, restore,
or open a pane already in the minimized state.

State is purely UI presentation and lives in memory for the duration of
the session — no persistence to disk.

## Motivation

- The shortcut bar adds visible weight under every AI terminal. Users
  who do not need it on a particular pane still pay for the screen real
  estate.
- Agents spawning side-effect panes via `kiri term split` (e.g. a
  background dev server) should be able to avoid pushing the user's
  primary view down, while still letting the user reach the controls if
  they want.

## Non-goals

- Persisting minimized state across app restarts.
- Hiding the terminal body itself; only the shortcut bar collapses.
- Inheriting minimized state from a parent pane on a UI-initiated split.
  UI splits always produce expanded panes; only `kiri term split
  --minimized` produces minimized panes.

## UX

### Layout

Normal (today):

```
+-------------------------------------------------+
|  REPLY  [OK] [Continue] [LGTM]            [+]   |
|  CMD    [/commit] [/pr-complete] ...      [+]   |
|  PICK   [1] [2] [3]                  [−] [⚙]    |   ← collapse left of settings
+-------------------------------------------------+
```

Minimized:

```
+-------------------------------------------------+
|                                       [↑] [⚙]   |   ← thin ~28px strip
+-------------------------------------------------+
```

- The collapse / restore button sits **immediately to the left of the
  settings cog**. Both stay in the existing `.bar-actions` wrapper.
- Icon set: `−` (minus) → collapse; `↑` (chevron-up) → restore.
- The bar transitions height and opacity using the project's existing
  `--transition-normal` token; no bespoke easing.

### Behavior

- The bar is only mounted while `isAiRunning` is true, exactly as today.
  Toggling collapse does not affect that gating.
- When `claude` / `codex` exits, the bar unmounts. If the user restarts
  the AI, the bar reappears expanded (no persistence).
- A UI-initiated split produces an expanded child pane regardless of the
  parent's state.

## State

A new derived structure on `terminalStore`:

```ts
// One bit per pane id. Missing key = expanded.
collapsedByPaneId: Map<paneId, boolean>
```

Operations:

- `terminalStore.isCollapsed(paneId): boolean`
- `terminalStore.setCollapsed(paneId, value: boolean): void`
- `terminalStore.toggleCollapsed(paneId): void`
- Entry is removed when the pane is closed (already handled inside
  `terminalStore`'s existing pane-removal path; we just need to drop the
  key).

This is the single source of truth. The CLI reaches it by sending an
event to the frontend and waiting for resolution, identical to the
existing `Split` / `Close` pattern.

## Components

### `TerminalShortcutBar.svelte`

New props:

```ts
collapsed: boolean;
onToggleCollapse: () => void;
```

Rendering rules:

- `{#if !collapsed}` wraps the three shortcut rows.
- `.bar-actions` always renders. Inside it, the new
  `.collapse-btn` is placed before `.settings-btn` so it appears to the
  left.
- The button label / icon / `title` swap based on `collapsed`:
  - expanded: title `Minimize shortcuts`, icon `−`, `aria-label` matches
  - collapsed: title `Restore shortcuts`, icon `↑`, `aria-label` matches
- When `collapsed === true`, the `.shortcut-bar` style switches to a
  thin variant: ~28px height, no top/bottom padding for rows, the
  `padding-right: 36px` reserved for the settings button is no longer
  needed since the bar has no row content.

### `Terminal.svelte`

- Subscribes to `terminalStore.isCollapsed(paneId)` via
  `$derived` and passes `collapsed` + `onToggleCollapse` to
  `TerminalShortcutBar`.
- Listens for the new event `cli:pane-minimize` (see CLI section),
  updates the store, and resolves the pending request via
  `cli_resolve_pending`.

### `terminalStore`

Add the `collapsedByPaneId` map and helper methods listed above. Hook
into the existing `removePane` (or equivalent) to delete the entry. No
persistence.

## CLI

### Wire protocol (`crates/kiri-cli-proto/src/wire.rs`)

Additions to `Request`:

```rust
Minimize { pane: PaneRef },
Restore  { pane: PaneRef },
Split    { pane: PaneRef, direction: SplitDirection,
           #[serde(default)] minimized: bool },
```

Additions to `Response`:

```rust
Minimize,    // unit variant, mirrors Response::Send
Restore,     // unit variant
```

Additions to `PaneInfo`:

```rust
pub minimized: bool,
```

### Pane map (`cli_server/pane_map.rs`)

`PaneEntry` gains a `collapsed: bool` field. This is a cache reflecting
state owned by the frontend; the frontend pushes it through the
existing `cli_update_pane_map` Tauri command alongside every other
pane update, and `ls` reads it back into `PaneInfo.minimized`. The
backend never mutates `collapsed` on its own.

### CLI args (`crates/kiri-cli/src/cli.rs`)

```text
kiri term minimize [--pane X]
kiri term restore  [--pane X]
kiri term split    [--pane X] [--dir h|v] [--minimized]
```

### Handlers (`cli_server/handlers.rs`)

Two new handlers, modeled after `split` / `close_pane`:

```text
fn minimize(ctx, pane) -> Response
fn restore (ctx, pane) -> Response
```

Each:

1. Resolves the pane via `pane_map.resolve`.
2. `pending.register(request_id)`.
3. `app.emit_to(label, "cli:pane-minimize", { requestId, paneId,
   minimized: bool })`. Both `Minimize` and `Restore` use this single
   event name, differing only in the `minimized` boolean. The frontend
   has one listener that branches on the payload.
4. Awaits the receiver with the existing 2-second timeout.
5. Returns `Response::Minimize` / `Response::Restore` on success or maps
   to a `FrontendUnresponsive` / forwarded error.

`split` learns the optional `minimized` parameter. After the frontend
acknowledges the split (returns `newPaneId`), the handler calls the
same minimize emit for that new pane id when the flag is set. The two-
phase approach keeps the existing split handshake unchanged.

### Frontend event handling

`Terminal.svelte` (or a small dedicated listener module if `Terminal`
already feels crowded) subscribes to `cli:pane-minimize`. On receipt:

1. If `event.payload.paneId` matches this pane, update the store via
   `terminalStore.setCollapsed(paneId, payload.minimized)`.
2. Call `cli_resolve_pending({ requestId, value: {} })`.
3. If the pane does not exist locally, return `{ error:
   "no_focused_pane" }` (mirrors current pattern for unknown pane).

## kiri-cli skill

`resources/skills/kiri-cli/SKILL.md` updates:

- §5 add `kiri term minimize` and `kiri term restore` subsections with
  example invocations and the `{ "type": "minimize" }` /
  `{ "type": "restore" }` response shapes.
- §5 `split` subsection: add `--minimized` flag and update the response
  example to show that the new pane comes up minimized.
- §5 `ls` response example: include `"minimized": false`.
- §8 Best practices: add a bullet
  > When the agent creates a side pane via `kiri term split` for its
  > own use (background dev server, log tail, etc.), prefer
  > `--minimized` so the user's primary view is not pushed down. The
  > user can restore it from the bar's `↑` button or via
  > `kiri term restore --pane <id>`.

## Tests

### Rust

- `wire.rs` round-trip tests for `Request::Minimize`, `Request::Restore`,
  `Request::Split { minimized: true }`, `Response::Minimize`,
  `Response::Restore`, and `PaneInfo { minimized: true }`.
- `cli.rs` clap parser tests for `term minimize`, `term restore`,
  `term split --minimized`.
- `handlers.rs` unit tests for `minimize` / `restore` mirroring the
  existing `split` / `close_pane` tests: ensure pane resolution failure
  returns `PaneNotFound`, frontend timeout returns
  `FrontendUnresponsive`, success path returns the right variant.

### Frontend

- `TerminalShortcutBar.browser.test.ts`:
  - `collapsed=true` hides REPLY / CMD / PICK rows.
  - Collapse button appears immediately before the settings button in
    DOM order.
  - Clicking the collapse button calls `onToggleCollapse`.
  - Icon / title swap based on `collapsed`.
- `terminalStore` unit tests for `setCollapsed` / `toggleCollapsed` /
  `isCollapsed` / cleanup on pane removal.

## Risks and edge cases

- **Race between `kiri term split --minimized` and pane mount**: the
  emit-after-acknowledge ordering guarantees the new pane is in the
  `terminalStore` before the minimize event fires. Confirmed by the
  existing `split` handshake.
- **Bar height change triggering xterm resize**: collapsing the bar
  shrinks the wrapper, which fires the existing `ResizeObserver`. No
  new wiring needed, but verify in dev that the resize is debounced as
  today and that Ink-based AIs do not redraw mid-frame.
- **Old `cli_update_pane_map` payloads**: if a frontend (e.g. during
  hot-reload) sends entries without `collapsed`, the backend should
  default to `false` via `#[serde(default)]`.
- **Multiple windows**: each window has its own CLI socket and pane
  store, so minimize is naturally scoped per window. No cross-window
  considerations.

## Out of scope / future

- Persisting minimized state per project or per pane id (intentionally
  excluded; revisit if user feedback asks for it).
- Bulk operations like `kiri term minimize --all`.
- Keyboard shortcut to toggle minimize. Easy to add later by routing the
  same `toggleCollapsed` action.
