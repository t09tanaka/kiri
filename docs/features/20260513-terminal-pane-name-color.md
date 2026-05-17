# Terminal Pane Name & Color

Status: Spec
Date: 2026-05-13

## Goal

Let the `kiri` CLI tag a newly-split terminal pane with a short **name** and a **color**, both of which appear in the pane's header bar inside the kiri app. The name is plain text; the color is one of six fixed swatches drawn from kiri's Mist palette. Both are optional and independent.

This is primarily a quality-of-life feature for agents (Claude Code, scripts, humans) that drive kiri via `kiri term split`. When multiple panes are open with overlapping responsibilities (`shell`, `build`, `watch`, `agent`, …), a colored dot plus a label makes the right pane obvious at a glance.

## Non-goals

- **Renaming or recoloring an existing pane.** This release only sets name/color at split time. A separate `kiri term label` command can be added later if needed; this spec does not add one.
- **Persistence across app restarts.** Name and color are held in the in-memory `terminalStore` and die with the window.
- **Inheritance.** When a pane is split, the new child pane starts with no name and no color regardless of the parent's settings.
- **Free-form colors.** Only the six tokens listed below are accepted. No hex, no themes, no extensibility surface.

## User-visible surface

### CLI

```bash
kiri term split [--pane X] [--dir h|v] [--name STR] [--color COLOR]
```

| Flag | Required | Notes |
|---|---|---|
| `--name STR` | no | 1–32 characters. Rejected with `invalid_argument` if empty, longer than 32, or containing control characters (`\x00–\x1f`, `\x7f`). |
| `--color COLOR` | no | One of `sky`, `iris`, `jade`, `amber`, `coral`, `rose`. Anything else is rejected by clap before the request leaves the CLI. |

All four combinations are valid:

```bash
kiri term split                                    # no label
kiri term split --name build                       # text only, no dot
kiri term split --color jade                       # dot only, no text
kiri term split --name build --color coral         # dot + text
```

### `kiri term ls` response

`PaneInfo` gains two optional fields. They are omitted entirely (not `null`) when unset, matching the existing `detail`-on-`error` convention.

```json
{
  "type": "ls",
  "panes": [
    {
      "index": 0,
      "id": "pane-1",
      "terminal_id": 1,
      "cwd": "/Users/me/project",
      "process_name": "zsh",
      "running": false,
      "memory_bytes": 5242880,
      "focused": true,
      "name": "build",
      "color": "coral"
    }
  ]
}
```

### Terminal header (UI)

The pane header (`.terminal-controls` in `Terminal.svelte`) gains a new "pane label" group, placed between the split buttons and the worktree tag:

```
[ ⊟ ] [ ⊟ ]   ● build         🌿 worktree-name    [ × ]
└─split──┘   └──pane label──┘  └──existing tag──┘ └close┘
```

- **Dot** — a `8px` circle filled with the chosen `--pane-color-*`, with a soft outer glow (`box-shadow`) matching the swatch. Shown only if `color` is set.
- **Name** — small monospace text. Shown only if `name` is set.
- If both are unset, the entire pane-label group is omitted (no spacer, no empty box).

The header layout still uses `margin-left: auto` on the **close** button to keep close pinned right. The worktree tag's existing `margin-left: auto` is replaced with a normal flex gap so that pane-label, worktree-tag, and close can sit in a single right-aligned row in that order.

## Architecture & data flow

**Split request flow:**

```
  kiri term split ──► Request::Split{name,color} ──► UDS
                                                       │
                                                       ▼
                            handlers.rs::split emits Tauri event
                            "cli:pane-split" {requestId,paneId,direction,name,color}
                                                       │
                                                       ▼
                            cliBridge.ts listens, calls
                            deps.splitPane(target, direction, {name, color})
                                                       │
                                                       ▼
                            terminalStore.splitPane creates new leaf
                            with name/color set on it
                                                       │
                                                       ▼
                            Svelte reactivity →
                            TerminalContainer passes leaf.name/color as props
                                                       │
                                                       ▼
                            Terminal.svelte renders ●name in header
```

**ls response flow** (name/color piggyback on the existing pane-map push, no extra round-trip):

```
  terminalStore changes ──► App.svelte's reactive pushPaneMap()
                                                       │
                                                       ▼
                            collectPaneEntries() reads name/color
                            from each TerminalPaneLeaf
                                                       │
                                                       ▼
                            invoke('cli_update_pane_map', {label, panes:[
                              {index, paneId, terminalId, focused, name?, color?}
                            ]})
                                                       │
                                                       ▼
                            PaneMap.replace(entries) — Rust side stores
                            name/color on each PaneEntry
                                                       │
                                                       ▼
                            On `kiri term ls`, handlers.rs::ls copies
                            PaneEntry.{name,color} into PaneInfo.{name,color}
```

The key consequence: name/color is the frontend's source of truth. The Rust side only stores what the frontend pushed via `cli_update_pane_map`; it does not maintain a separate label registry.

## Detailed changes

### 1. `crates/kiri-cli-proto`

**`src/types.rs`** — add a new enum:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaneColor {
    Sky,
    Iris,
    Jade,
    Amber,
    Coral,
    Rose,
}
```

Re-export from `lib.rs`.

**`src/wire.rs`** — extend `Request::Split` and `PaneInfo`:

```rust
Split {
    pane: PaneRef,
    direction: SplitDirection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    color: Option<PaneColor>,
},

pub struct PaneInfo {
    pub index: u32,
    pub id: String,
    pub terminal_id: u32,
    pub cwd: Option<String>,
    pub process_name: String,
    pub running: bool,
    pub memory_bytes: u64,
    pub focused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<PaneColor>,
}
```

Roundtrip tests for both Request::Split with/without the new fields, and for PaneInfo emission.

### 2. `crates/kiri-cli`

**`src/cli.rs`** — add flags to `SplitArgs`:

```rust
#[derive(Args, Debug)]
pub struct SplitArgs {
    #[command(flatten)]
    pub pane: PaneOpt,
    #[arg(long, default_value = "h")]
    pub dir: String,
    /// Optional pane label shown in the terminal header (1–32 chars, no control chars).
    #[arg(long)]
    pub name: Option<String>,
    /// Optional pane color shown in the terminal header.
    #[arg(long, value_enum)]
    pub color: Option<PaneColorArg>,
}
```

`PaneColorArg` is a `clap::ValueEnum` mirroring `PaneColor`. Conversion `PaneColorArg → PaneColor` happens in `main.rs` before serializing the wire request.

Validation of `--name`:
- Reject `""` and any string longer than 32 chars (`Err(clap::Error::raw(InvalidValue, ...))`).
- Reject any control char via a clap `value_parser`.
- Whitespace is allowed (e.g. "dev server"); trimming is the user's call.

Parser tests cover: empty, too long, control char, valid 1-char, valid 32-char, missing flag, valid colors, invalid color, missing color.

### 3. `src-tauri/src/commands/cli_server`

**`pane_map.rs`** — `PaneEntry` grows two optional fields:

```rust
pub struct PaneEntry {
    pub index: u32,
    pub pane_id: String,
    pub terminal_id: u32,
    pub focused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<PaneColor>,   // re-exported from kiri-cli-proto
}
```

These are populated by the frontend's `cli_update_pane_map` invoke and overwritten on every push, so the backend never needs to mutate them directly.

**`handlers.rs::split`** — picks up the new `name`/`color` fields off `Request::Split` and adds them to the Tauri-emitted `cli:pane-split` payload:

```rust
let payload = json!({
    "requestId": request_id,
    "paneId": pane_id,
    "direction": dir_str,
    "name": name,    // Option<String> serializes to either the string or omitted/null
    "color": color,  // Option<PaneColor> same
});
app.emit_to(label, "cli:pane-split", payload)?;
```

The downstream "wait for reply" loop is unchanged — the new fields ride alongside.

**`handlers.rs::ls`** — copies `name`/`color` from each `PaneEntry` into the resulting `PaneInfo`:

```rust
panes.push(kiri_cli_proto::PaneInfo {
    // existing fields …
    name: e.name,
    color: e.color,
});
```

**Tests** — extend the existing `pane_map` and `handlers` unit tests to cover the new fields. No new test files.

### 4. Frontend

**`src/lib/stores/terminalStore.ts`**

```ts
export type PaneColor = 'sky' | 'iris' | 'jade' | 'amber' | 'coral' | 'rose';

export interface TerminalPaneLeaf {
  type: 'terminal';
  id: string;
  terminalId: number | null;
  cwd?: string | null;
  name?: string;
  color?: PaneColor;
}

// splitPane signature
splitPane: (
  paneId: string,
  direction: 'horizontal' | 'vertical',
  opts?: { name?: string; color?: PaneColor },
) => string;
```

The new pane (the right/bottom child) receives `name` and `color`. The existing target pane (left/top child) is left untouched. `splitPaneInTree` and its recursive helpers thread the opts through so the new leaf is constructed with the label set in one step.

**`src/lib/services/cliBridge.ts`**

`CliBridgeDeps.splitPane` signature widens to accept opts, and the `cli:pane-split` event payload type gains the two fields:

```ts
splitPane: (
  paneId: string,
  direction: 'horizontal' | 'vertical',
  opts?: { name?: string; color?: PaneColor },
) => string;

const unlistenSplit = await listen<{
  requestId: string;
  paneId: string;
  direction: 'horizontal' | 'vertical';
  name?: string;
  color?: PaneColor;
}>('cli:pane-split', (event) => {
  const { requestId, paneId, direction, name, color } = event.payload;
  // … resolveTarget unchanged …
  const newPaneId = deps.splitPane(target, direction, { name, color });
  reply(requestId, { newPaneId, newPaneIndex: deps.indexOf(newPaneId) });
});
```

Tests in `cliBridge.test.ts` extend to cover: event with neither, event with name only, event with color only, event with both. The bridge does not validate name/color values — it forwards what it received. Bad colors are quarantined: see "edge cases".

**`src/App.svelte`** — `collectPaneEntries` adds the two fields:

```ts
const out: Array<{
  index: number;
  paneId: string;
  terminalId: number;
  focused: boolean;
  name?: string;
  color?: PaneColor;
}> = [];
// …
out.push({
  index: i++,
  paneId: pane.id,
  terminalId,
  focused: pane.id === focusedId,
  ...(pane.name ? { name: pane.name } : {}),
  ...(pane.color ? { color: pane.color } : {}),
});
```

`pushPaneMap` is already triggered reactively when the store changes, so no new wiring is needed — name/color flow to the Rust side automatically on every split.

Tests in `terminalStore.test.ts`:
- splitPane with `{ name: 'build' }` → new leaf has `name: 'build'`, no `color`.
- splitPane with `{ color: 'jade' }` → new leaf has `color: 'jade'`, no `name`.
- splitPane with both → new leaf has both.
- splitPane with neither (existing call sites) → unchanged behavior.

**`src/lib/components/terminal/TerminalContainer.svelte`**

The `Terminal` invocation gains two new props sourced from the leaf:

```svelte
<Terminal
  paneId={pane.id}
  cwd={pane.cwd || cwd}
  name={pane.name}
  color={pane.color}
  …
/>
```

Local split buttons (`handleSplitHorizontal`/`handleSplitVertical`) keep their current behavior — they call `splitPane(id, dir)` with no opts, so user-driven UI splits stay unnamed/uncolored. Only CLI-driven splits set name/color.

**`src/lib/components/terminal/Terminal.svelte`**

Props extended:

```ts
interface Props {
  paneId: string;
  cwd?: string | null;
  showControls?: boolean;
  name?: string;
  color?: PaneColor;
  onSplitHorizontal?: () => void;
  onSplitVertical?: () => void;
  onClose?: () => void;
}
```

Header markup (inside `{#if showControls}`):

```svelte
{#if name || color}
  <span class="pane-label">
    {#if color}<span class="pane-dot" style:--pane-color={`var(--pane-color-${color})`}></span>{/if}
    {#if name}<span class="pane-name">{name}</span>{/if}
  </span>
{/if}
```

The element sits immediately after the second split button and immediately before the worktree tag. The worktree tag loses its `margin-left: auto` (the close button keeps `margin-left: auto`, which still right-aligns the trailing group correctly).

Styles:

```css
.pane-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 8px;
  font-family: 'IBM Plex Mono', monospace;
  font-size: 11px;
  color: var(--text-secondary);
  letter-spacing: 0.04em;
}

.pane-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--pane-color);
  box-shadow: 0 0 6px 0.5px color-mix(in srgb, var(--pane-color) 60%, transparent);
}

.pane-name {
  white-space: nowrap;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
}
```

Browser test (`Terminal.browser.test.ts`):
- Renders with `name='build'` only → name visible, no dot.
- Renders with `color='jade'` only → dot visible with `jade` background, no name.
- Renders with both → both visible.
- Renders with neither → `.pane-label` not in DOM.

**`src/app.css`** — add the six CSS variables under `:root` (alongside the existing `--accent-*` tokens):

```css
:root {
  --pane-color-sky:   #7dd3fc;
  --pane-color-iris:  #c4b5fd;
  --pane-color-jade:  #86efac;
  --pane-color-amber: #fcd34d;
  --pane-color-coral: #fb923c;
  --pane-color-rose:  #f9a8d4;
}
```

### 5. `~/.claude/skills/kiri-cli/SKILL.md`

(Outside the repo, but updated as part of this change.)

- `kiri term split` section gains `--name` and `--color` flags with the allowed color list.
- `ls` JSON example gains the two optional fields.
- A short "Pane labels" subsection explains that labels are display-only and are set at split time only.

## Edge cases

- **Name overflow** — UI clips with `text-overflow: ellipsis` at `max-width: 200px`. Backend still stores the full string (up to 32 chars).
- **Unicode in name** — allowed. The 32-char cap is applied to the Rust `String` `.chars().count()`, not byte length.
- **Empty `--name=""`** — rejected at the CLI layer (`invalid_argument`).
- **Whitespace-only name** — allowed (user's choice); not auto-trimmed.
- **`--color foo`** — clap rejects with its standard "invalid value" error. The wire is never hit.
- **Backend receives a Split with `name` longer than 32 chars** (shouldn't happen via the CLI, but the wire is forgiving) → backend re-validates and responds with `Response::Error { code: invalid_argument, … }`.
- **Frontend pushes a `PaneEntry` with an unknown `color`** — the Rust side uses `serde(deserialize_with = …)` (or an `Option<PaneColor>` whose `Deserialize` falls back to `None` on unrecognized strings) so the bad value is discarded and treated as no-color. This protects `ls` from returning corrupt JSON if the frontend ever drifts.
- **Closing a labeled pane** — no special handling; the leaf is dropped along with its label.
- **App restart** — labels are gone; `ls` will return panes without `name`/`color`.

## Test plan

| Layer | Test |
|---|---|
| `kiri-cli-proto` | Roundtrip of `Request::Split` with and without `name`/`color`. Roundtrip of `PaneInfo` with and without the new fields. `PaneColor` serializes to snake_case. |
| `kiri-cli` | clap parses each color enum value. Empty `--name`, 33-char `--name`, control-char `--name` are rejected. 1-char and 32-char `--name` are accepted. Invalid `--color foo` is rejected. |
| `terminalStore` (Vitest) | `splitPane` with `{ name }`, `{ color }`, `{ name, color }`, and no opts; each produces the expected leaf state. |
| `cliBridge` (Vitest) | `cli:pane-split` events with each of the four name/color combinations are forwarded to `splitPane` with matching opts. |
| `Terminal.svelte` (browser test) | Renders pane-label with name only, color only, both, neither; dot color reflects the requested `--pane-color-*` variable. |
| Backend handler (Rust unit) | `split` handler emits name/color on the Tauri event. `ls` handler returns name/color in `PaneInfo` when present on `PaneEntry`, and omits them when absent. Round-trip of `PaneEntry` with optional name/color. |
| E2E (Tauri MCP) | `kiri term split --name build --color coral` from inside a kiri pane → the new pane's header shows the dot + name. `kiri term ls` returns the labeled pane with `name` and `color`. |

## Out of scope (future work)

- `kiri term label --pane X [--name STR] [--color COLOR] [--clear]` to retag existing panes.
- Persistence across restarts (would belong with broader pane-state persistence).
- Configurable colors / theming.
- Naming the *first* pane of a window (which is opened by the app, not by a CLI split).
