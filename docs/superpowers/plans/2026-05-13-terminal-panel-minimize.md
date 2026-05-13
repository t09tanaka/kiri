# Terminal Panel Minimize / Restore Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add collapse/expand controls for the per-pane `TerminalShortcutBar` and surface the same state through `kiri term minimize / restore` plus a `kiri term split --minimized` flag, with state owned by the Svelte `terminalStore` and the backend `pane_map` acting as a reflection cache.

**Architecture:** Bottom-up: wire protocol → CLI args → CLI main wiring → backend pane_map + handlers + Tauri command surface → frontend store → UI component → cli bridge listener → skill docs → integration check. State is in-memory only; persistence is intentionally out of scope (see spec §State).

**Tech Stack:** Rust (kiri-cli-proto, kiri-cli, tauri), Svelte 5 + TypeScript, Vitest (unit + browser), `@testing-library/svelte`, Tauri 2 IPC (`emit_to` + `cli_resolve_pending`).

**Spec reference:** `docs/superpowers/specs/2026-05-13-terminal-panel-minimize-design.md`

---

## File map

**Modify:**
- `crates/kiri-cli-proto/src/wire.rs` — `Request::Minimize`, `Request::Restore`, `Request::Split.minimized`, `Response::Minimize`, `Response::Restore`, `PaneInfo.minimized`.
- `crates/kiri-cli/src/cli.rs` — `TermCmd::Minimize`, `TermCmd::Restore`, `SplitArgs.minimized`.
- `crates/kiri-cli/src/main.rs` — translate the new subcommands into `Request` values (Task 3 reads this file to see the exact pattern).
- `crates/kiri-cli/src/render.rs` — pretty-print output for `Response::Minimize` / `Response::Restore` if the existing match needs to cover them.
- `src-tauri/src/commands/cli_server/pane_map.rs` — `PaneEntry.collapsed: bool`.
- `src-tauri/src/commands/cli_server.rs` — accept `collapsed` in `cli_update_pane_map`.
- `src-tauri/src/commands/cli_server/handlers.rs` — `minimize`, `restore`, `Request::Split` minimized branch, `ls` populates `minimized`.
- `src/lib/stores/terminalStore.ts` — collapse map and helpers, cleanup on pane removal, expose in snapshot.
- `src/lib/stores/terminalStore.test.ts` — collapse API tests.
- `src/lib/services/cliBridge.ts` — `cli:pane-minimize` listener, `cli:pane-split` payload extended with `minimized`, deps gain `setPaneCollapsed`.
- `src/lib/services/cliBridge.test.ts` — listener + minimized split tests.
- `src/App.svelte` — `collectPaneEntries` includes `collapsed`, wire `setPaneCollapsed` into `startCliBridge`.
- `src/lib/components/terminal/TerminalShortcutBar.svelte` — `collapsed` prop, collapse button, thin-bar styles.
- `src/lib/components/terminal/TerminalShortcutBar.browser.test.ts` — new test cases.
- `src/lib/components/terminal/Terminal.svelte` — read collapsed from store, pass props, toggle on click.
- `resources/skills/kiri-cli/SKILL.md` — minimize/restore docs + best practice.

**No new files.** Every change lands in an existing module per the spec's principle that each file already has one clear responsibility.

---

## Task 1: Add `minimized` to `PaneInfo` and `Response::Minimize`/`Restore` + new requests in `wire.rs`

**Files:**
- Modify: `crates/kiri-cli-proto/src/wire.rs`
- Test: same file (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing tests**

Append to `mod tests` at the bottom of `crates/kiri-cli-proto/src/wire.rs`:

```rust
#[test]
fn pane_info_minimized_defaults_to_false_when_absent() {
    let parsed: PaneInfo = serde_json::from_value(serde_json::json!({
        "index": 0,
        "id": "pane-1",
        "terminal_id": 1,
        "cwd": null,
        "process_name": "zsh",
        "running": false,
        "memory_bytes": 0,
        "focused": true
    }))
    .unwrap();
    assert!(!parsed.minimized);
}

#[test]
fn pane_info_minimized_round_trips() {
    let info = PaneInfo {
        index: 0,
        id: "pane-1".into(),
        terminal_id: 1,
        cwd: None,
        process_name: "zsh".into(),
        running: false,
        memory_bytes: 0,
        focused: true,
        minimized: true,
    };
    roundtrip(&info);
}

#[test]
fn request_minimize_round_trip() {
    roundtrip(&Request::Minimize {
        pane: PaneRef::Index(2),
    });
}

#[test]
fn request_restore_round_trip() {
    roundtrip(&Request::Restore {
        pane: PaneRef::focused(),
    });
}

#[test]
fn response_minimize_serializes_as_unit() {
    let v = serde_json::to_value(Response::Minimize).unwrap();
    assert_eq!(v, serde_json::json!({ "type": "minimize" }));
}

#[test]
fn response_restore_serializes_as_unit() {
    let v = serde_json::to_value(Response::Restore).unwrap();
    assert_eq!(v, serde_json::json!({ "type": "restore" }));
}

#[test]
fn request_split_defaults_minimized_to_false() {
    let parsed: Request = serde_json::from_value(serde_json::json!({
        "type": "split",
        "pane": 0,
        "direction": "horizontal"
    }))
    .unwrap();
    assert_eq!(
        parsed,
        Request::Split {
            pane: PaneRef::Index(0),
            direction: SplitDirection::Horizontal,
            minimized: false,
        }
    );
}

#[test]
fn request_split_with_minimized_round_trip() {
    roundtrip(&Request::Split {
        pane: PaneRef::Index(1),
        direction: SplitDirection::Vertical,
        minimized: true,
    });
}
```

Also implement the new `roundtrip` helper if needed (one already exists in the test module — reuse it).

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test -p kiri-cli-proto wire
```
Expected: compilation errors / missing variants / missing field.

- [ ] **Step 3: Implement the wire changes**

Update `Request`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Request {
    Ls,
    Run { /* unchanged */ },
    Send { /* unchanged */ },
    Read { /* unchanged */ },
    Follow { /* unchanged */ },
    Cancel { /* unchanged */ },
    Split {
        pane: PaneRef,
        direction: SplitDirection,
        #[serde(default)]
        minimized: bool,
    },
    Close { /* unchanged */ },
    Minimize { pane: PaneRef },
    Restore { pane: PaneRef },
}
```

Update `PaneInfo`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaneInfo {
    pub index: u32,
    pub id: String,
    pub terminal_id: u32,
    pub cwd: Option<String>,
    pub process_name: String,
    pub running: bool,
    pub memory_bytes: u64,
    pub focused: bool,
    #[serde(default)]
    pub minimized: bool,
}
```

Update `Response`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Response {
    /* existing variants ... */
    Minimize,
    Restore,
}
```

- [ ] **Step 4: Run the tests**

```bash
cargo test -p kiri-cli-proto wire
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-cli-proto/src/wire.rs
git commit -m "feat(cli-proto): add minimize/restore requests and PaneInfo.minimized"
```

---

## Task 2: Add `minimize` / `restore` subcommands and `--minimized` flag to `cli.rs`

**Files:**
- Modify: `crates/kiri-cli/src/cli.rs`
- Test: same file (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing tests**

Add to `mod tests` at the bottom of `crates/kiri-cli/src/cli.rs`:

```rust
#[test]
fn parse_minimize_subcommand() {
    let cli = Cli::try_parse_from(["kiri", "term", "minimize"]).unwrap();
    match cli.command {
        Top::Term(TermCmd::Minimize(opt)) => {
            assert_eq!(parse_pane(&opt), PaneRef::focused());
        }
        _ => panic!("expected minimize"),
    }
}

#[test]
fn parse_restore_subcommand_with_pane() {
    let cli = Cli::try_parse_from(["kiri", "term", "restore", "--pane", "pane-2"]).unwrap();
    match cli.command {
        Top::Term(TermCmd::Restore(opt)) => {
            assert_eq!(parse_pane(&opt), PaneRef::Id("pane-2".into()));
        }
        _ => panic!("expected restore"),
    }
}

#[test]
fn parse_split_minimized_flag() {
    let cli = Cli::try_parse_from(["kiri", "term", "split", "--minimized"]).unwrap();
    match cli.command {
        Top::Term(TermCmd::Split(args)) => assert!(args.minimized),
        _ => panic!("expected split"),
    }
}

#[test]
fn parse_split_default_minimized_false() {
    let cli = Cli::try_parse_from(["kiri", "term", "split"]).unwrap();
    match cli.command {
        Top::Term(TermCmd::Split(args)) => assert!(!args.minimized),
        _ => panic!("expected split"),
    }
}
```

Add `use clap::Parser` next to the existing test imports if missing.

- [ ] **Step 2: Run tests**

```bash
cargo test -p kiri-cli cli::
```
Expected: FAIL on missing variants / fields.

- [ ] **Step 3: Implement**

Extend `TermCmd`:

```rust
#[derive(Subcommand, Debug)]
pub enum TermCmd {
    Ls,
    Run(RunArgs),
    Send(SendArgs),
    Read(ReadArgs),
    Follow(FollowArgs),
    Cancel(PaneOpt),
    Split(SplitArgs),
    Close(PaneOpt),
    /// Collapse the shortcut bar to a thin strip with only the restore + settings buttons.
    Minimize(PaneOpt),
    /// Expand a minimized shortcut bar back to its full layout.
    Restore(PaneOpt),
}
```

Extend `SplitArgs`:

```rust
#[derive(Args, Debug)]
pub struct SplitArgs {
    #[command(flatten)]
    pub pane: PaneOpt,
    #[arg(long, default_value = "h")]
    pub dir: String,
    /// Create the new pane with its shortcut bar already minimized.
    #[arg(long)]
    pub minimized: bool,
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p kiri-cli cli::
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-cli/src/cli.rs
git commit -m "feat(cli): add term minimize/restore subcommands and split --minimized"
```

---

## Task 3: Wire the new subcommands in `main.rs` (and `render.rs` if needed)

**Files:**
- Modify: `crates/kiri-cli/src/main.rs`
- Modify: `crates/kiri-cli/src/render.rs`

- [ ] **Step 1: Read the existing dispatch and pretty-print paths**

Open both files and locate the `match` that maps a parsed `TermCmd` to a `Request`. The split / close arms are the closest analogues. Identify whether `render.rs` has a `match Response` that needs new arms.

- [ ] **Step 2: Add `Minimize`, `Restore`, and `minimized` to the dispatch**

In `main.rs`, add new arms (the exact pattern depends on the existing code — match what `split` and `close` do):

```rust
TermCmd::Minimize(opt) => Request::Minimize { pane: cli::parse_pane(&opt) },
TermCmd::Restore(opt)  => Request::Restore  { pane: cli::parse_pane(&opt) },
TermCmd::Split(args) => Request::Split {
    pane: cli::parse_pane(&args.pane),
    direction: match args.dir.as_str() {
        "v" | "vertical" => SplitDirection::Vertical,
        _ => SplitDirection::Horizontal,
    },
    minimized: args.minimized,
},
```

If `render.rs` exhaustively matches `Response`, add:

```rust
Response::Minimize => writeln!(out, "minimized")?,
Response::Restore  => writeln!(out, "restored")?,
```
(or whatever pattern the file uses for `Response::Send` / `Response::Close`.)

- [ ] **Step 3: Verify the binary still builds**

```bash
cargo build -p kiri-cli
```
Expected: clean build.

- [ ] **Step 4: Smoke-test the JSON output of the parser**

The wire request is emitted by `kiri` over the socket; without a running backend we can at least ensure parsing succeeds:

```bash
cargo run -p kiri-cli -- term minimize --help
cargo run -p kiri-cli -- term split --help
```
Expected: clap prints help text including `--minimized`.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-cli/src/main.rs crates/kiri-cli/src/render.rs
git commit -m "feat(cli): dispatch minimize/restore/split-minimized to wire requests"
```

---

## Task 4: Add `collapsed` to `PaneEntry` in the backend `pane_map`

**Files:**
- Modify: `src-tauri/src/commands/cli_server/pane_map.rs`
- Test: same file (inline `#[cfg(test)]`)

- [ ] **Step 1: Write the failing test**

Add to `mod tests` near the bottom:

```rust
#[test]
fn pane_entry_collapsed_defaults_to_false_in_json() {
    let parsed: PaneEntry = serde_json::from_value(serde_json::json!({
        "index": 0,
        "paneId": "p",
        "terminalId": 1,
        "focused": true
    }))
    .unwrap();
    assert!(!parsed.collapsed);
}

#[test]
fn pane_entry_collapsed_round_trips() {
    let entry = PaneEntry {
        index: 0,
        pane_id: "p".into(),
        terminal_id: 1,
        focused: true,
        collapsed: true,
    };
    let v = serde_json::to_value(&entry).unwrap();
    assert_eq!(v["collapsed"], serde_json::Value::Bool(true));
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p kiri pane_map
```
Expected: FAIL on missing field.

- [ ] **Step 3: Implement**

Modify `PaneEntry`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaneEntry {
    pub index: u32,
    pub pane_id: String,
    pub terminal_id: u32,
    pub focused: bool,
    #[serde(default)]
    pub collapsed: bool,
}
```

Fix the inline test `entry` helper to take `collapsed: bool` or default it:

```rust
fn entry(index: u32, pane_id: &str, terminal_id: u32, focused: bool) -> PaneEntry {
    PaneEntry {
        index,
        pane_id: pane_id.to_string(),
        terminal_id,
        focused,
        collapsed: false,
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p kiri pane_map
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/cli_server/pane_map.rs
git commit -m "feat(cli-server): track collapsed bit per pane in pane_map"
```

---

## Task 5: Propagate `minimized` through `ls`

**Files:**
- Modify: `src-tauri/src/commands/cli_server/handlers.rs`

- [ ] **Step 1: Read the existing `ls` handler**

Locate the function around line 28 in `handlers.rs`. It constructs `PaneInfo` from each `PaneEntry`.

- [ ] **Step 2: Add the field**

Inside the `panes.push(kiri_cli_proto::PaneInfo { … })` block, after `focused: e.focused`, add:

```rust
minimized: e.collapsed,
```

`e` is the `PaneEntry` returned by `pane_map.snapshot()`.

- [ ] **Step 3: Verify the workspace builds**

```bash
cargo build
```
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/cli_server/handlers.rs
git commit -m "feat(cli-server): reflect collapsed pane state in ls response"
```

---

## Task 6: Implement `minimize` / `restore` handlers

**Files:**
- Modify: `src-tauri/src/commands/cli_server/handlers.rs`

- [ ] **Step 1: Add the dispatch entries**

Inside the `handle` function's `match req`, add:

```rust
Request::Minimize { pane } => vec![set_collapsed(ctx, pane, true).await],
Request::Restore  { pane } => vec![set_collapsed(ctx, pane, false).await],
```

- [ ] **Step 2: Write the handler**

Below `close_pane`, add:

```rust
async fn set_collapsed(ctx: &DispatchContext, p: PaneRef, minimized: bool) -> Response {
    let Some(app) = ctx.app.as_ref() else {
        return internal("no Tauri AppHandle bound to dispatch context");
    };
    let Some(pane) = ctx.pane_map.resolve(&p) else {
        return pane_not_found(p);
    };
    let request_id = format!("minimize-{}", uuid::Uuid::new_v4());
    let rx = ctx.pending.register(request_id.clone());
    let payload = serde_json::json!({
        "requestId": request_id,
        "paneId": pane.pane_id,
        "minimized": minimized,
    });
    if let Err(e) = app.emit_to(ctx.label.as_str(), "cli:pane-minimize", payload) {
        ctx.pending.cancel(&request_id);
        return Response::Error {
            code: ErrorCode::FrontendUnresponsive,
            message: format!("emit failed: {e}"),
            detail: None,
        };
    }
    match timeout(Duration::from_secs(2), rx).await {
        Ok(Ok(value)) => {
            let err_code = value
                .get("error")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let Some(code) = err_code {
                return frontend_error_to_response(
                    &code,
                    value,
                    if minimized { "minimize" } else { "restore" },
                );
            }
            if minimized {
                Response::Minimize
            } else {
                Response::Restore
            }
        }
        _ => {
            ctx.pending.cancel(&request_id);
            Response::Error {
                code: ErrorCode::FrontendUnresponsive,
                message: "frontend did not reply within 2s".into(),
                detail: None,
            }
        }
    }
}
```

- [ ] **Step 3: Verify the workspace builds**

```bash
cargo build
```
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/cli_server/handlers.rs
git commit -m "feat(cli-server): handle minimize/restore requests via frontend bridge"
```

---

## Task 7: Carry `minimized` through `split`

**Files:**
- Modify: `src-tauri/src/commands/cli_server/handlers.rs`

- [ ] **Step 1: Update the dispatch arm**

In `handle`, change:

```rust
Request::Split { pane, direction } => vec![split(ctx, pane, direction).await],
```
to:

```rust
Request::Split { pane, direction, minimized } => {
    vec![split(ctx, pane, direction, minimized).await]
}
```

- [ ] **Step 2: Extend `split` to forward the flag**

Update the signature and event payload of `split` (the function near line 317):

```rust
async fn split(
    ctx: &DispatchContext,
    p: PaneRef,
    direction: SplitDirection,
    minimized: bool,
) -> Response {
    /* ... existing resolve + register ... */
    let payload = serde_json::json!({
        "requestId": request_id,
        "paneId": pane.pane_id,
        "direction": match direction {
            SplitDirection::Horizontal => "horizontal",
            SplitDirection::Vertical => "vertical",
        },
        "minimized": minimized,
    });
    /* rest unchanged */
}
```

The frontend listener will read `minimized` and pre-collapse the new pane before resolving (Task 11).

- [ ] **Step 3: Build**

```bash
cargo build
```
Expected: clean.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/commands/cli_server/handlers.rs
git commit -m "feat(cli-server): forward split --minimized to frontend"
```

---

## Task 8: Allow `cli_update_pane_map` to receive `collapsed`

**Files:**
- Modify: `src-tauri/src/commands/cli_server.rs`

- [ ] **Step 1: Locate the command**

Open `src-tauri/src/commands/cli_server.rs` and find `cli_update_pane_map` (around line 206). It deserializes `Vec<PaneEntry>` from the frontend.

- [ ] **Step 2: Verify nothing else is required**

Because `PaneEntry.collapsed` now exists with `#[serde(default)]` (Task 4), the command should accept old payloads (no `collapsed`) and new payloads (with `collapsed`) without code changes. Confirm by reading the function body — if it round-trips through `PaneEntry`, no further work is needed.

If the command instead uses a private inline struct, mirror `collapsed: Option<bool>` there and default it when calling `PaneMap::replace`.

- [ ] **Step 3: Build**

```bash
cargo build
```
Expected: clean. If you had to add code, add a rust test that asserts the command accepts a payload with `collapsed: true` in `cli_server.rs` `mod tests` (or skip if the existing test file already covers it).

- [ ] **Step 4: Commit (only if code changed)**

```bash
git add src-tauri/src/commands/cli_server.rs
git commit -m "feat(cli-server): accept collapsed in cli_update_pane_map payload"
```

If no code changed, skip this commit entirely.

---

## Task 9: Add the `collapsed` map to `terminalStore`

**Files:**
- Modify: `src/lib/stores/terminalStore.ts`
- Test: `src/lib/stores/terminalStore.test.ts`

- [ ] **Step 1: Write the failing tests**

Append to `src/lib/stores/terminalStore.test.ts`:

```typescript
import { terminalStore } from './terminalStore';

describe('terminalStore collapsed state', () => {
  beforeEach(() => terminalStore.reset());

  it('isCollapsed returns false for unknown panes', () => {
    expect(terminalStore.isCollapsed('pane-unknown')).toBe(false);
  });

  it('setCollapsed and isCollapsed roundtrip', () => {
    terminalStore.setCollapsed('pane-1', true);
    expect(terminalStore.isCollapsed('pane-1')).toBe(true);
    terminalStore.setCollapsed('pane-1', false);
    expect(terminalStore.isCollapsed('pane-1')).toBe(false);
  });

  it('toggleCollapsed flips state', () => {
    terminalStore.toggleCollapsed('pane-1');
    expect(terminalStore.isCollapsed('pane-1')).toBe(true);
    terminalStore.toggleCollapsed('pane-1');
    expect(terminalStore.isCollapsed('pane-1')).toBe(false);
  });

  it('closePane clears the collapsed bit for that pane', () => {
    terminalStore.setCollapsed('pane-1', true);
    terminalStore.closePane('pane-1');
    expect(terminalStore.isCollapsed('pane-1')).toBe(false);
  });

  it('snapshot exposes collapsed map for use by App.svelte', () => {
    terminalStore.setCollapsed('pane-1', true);
    const snap = terminalStore.snapshot();
    expect(snap.collapsedByPaneId).toBeInstanceOf(Map);
    expect(snap.collapsedByPaneId.get('pane-1')).toBe(true);
  });
});
```

If `terminalStore` does not already expose `reset()`, use whatever the existing tests do for cleanup (often re-initialising in `beforeEach`). Look at the top of `terminalStore.test.ts` for the pattern and copy it.

- [ ] **Step 2: Run the failing tests**

```bash
npm run test -- --run terminalStore
```
Expected: FAIL on `setCollapsed is not a function`.

- [ ] **Step 3: Implement**

In `terminalStore.ts`, add private state:

```typescript
let collapsedByPaneId: Map<string, boolean> = new Map();
```

(Use a plain `Map`; the store is not exposed as reactive data in templates — `Terminal.svelte` subscribes through the existing store subscription, see Task 13.)

Add to the store factory's returned object:

```typescript
isCollapsed(paneId: string): boolean {
  return collapsedByPaneId.get(paneId) ?? false;
},
setCollapsed(paneId: string, value: boolean): void {
  if (value) collapsedByPaneId.set(paneId, true);
  else collapsedByPaneId.delete(paneId);
  notify();
},
toggleCollapsed(paneId: string): void {
  this.setCollapsed(paneId, !this.isCollapsed(paneId));
},
```

`notify()` should be whatever the store uses today to push updates to subscribers (look near the existing `subscribe`/`splitPane` calls — there will be a writable store update or a manual emit).

Hook into `closePane` (or the underlying pane removal path) to call `collapsedByPaneId.delete(paneId)` for every pane removed (recursively if a split-with-children is closed). If `closePane` already walks the tree, add a `clearCollapsedRecursive(pane)` helper.

Extend the `snapshot()` return value to include `collapsedByPaneId: new Map(collapsedByPaneId)` (defensive copy).

- [ ] **Step 4: Run the tests**

```bash
npm run test -- --run terminalStore
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/terminalStore.ts src/lib/stores/terminalStore.test.ts
git commit -m "feat(terminal): track collapsed shortcut-bar state per pane in terminalStore"
```

---

## Task 10: Push `collapsed` through `cli_update_pane_map` from `App.svelte`

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Extend `collectPaneEntries`**

Locate the function (around line 91) and change its return-type tuple to include `collapsed: boolean`:

```typescript
function collectPaneEntries(
  root: TerminalPane | null,
  focusedId: string | null
): Array<{
  index: number;
  paneId: string;
  terminalId: number;
  focused: boolean;
  collapsed: boolean;
}> {
  /* unchanged setup */
  const visit = (pane: TerminalPane) => {
    if (pane.type === 'terminal') {
      const terminalId = terminalStore.terminalIdFor(pane.id);
      if (terminalId !== null) {
        out.push({
          index: i++,
          paneId: pane.id,
          terminalId,
          focused: pane.id === focusedId,
          collapsed: terminalStore.isCollapsed(pane.id),
        });
      }
    } else {
      for (const c of pane.children) visit(c);
    }
  };
  /* unchanged */
}
```

The backend's `PaneEntry` is `camelCase`-deserialized, so the key must be `collapsed` (not `collapsedByPaneId`).

- [ ] **Step 2: Verify type-check**

```bash
npm run check
```
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add src/App.svelte
git commit -m "feat(cli-bridge): include collapsed state in cli_update_pane_map payload"
```

---

## Task 11: Wire the `cli:pane-minimize` listener and minimized split into `cliBridge.ts`

**Files:**
- Modify: `src/lib/services/cliBridge.ts`
- Test: `src/lib/services/cliBridge.test.ts`
- Modify: `src/App.svelte` (deps wiring)

- [ ] **Step 1: Write the failing tests**

Append to `src/lib/services/cliBridge.test.ts`:

```typescript
it('on cli:pane-minimize, calls setPaneCollapsed and resolves', async () => {
  const setPaneCollapsed = vi.fn();
  await startCliBridge({
    label: 'main',
    splitPane: vi.fn().mockReturnValue('pane-2'),
    closePane: vi.fn(),
    indexOf: vi.fn().mockReturnValue(1),
    resolveFocusedPaneId: () => 'pane-1',
    setPaneCollapsed,
  });

  listeners.get('cli:pane-minimize')!({
    payload: { requestId: 'r1', paneId: 'pane-1', minimized: true },
  });

  expect(setPaneCollapsed).toHaveBeenCalledWith('pane-1', true);
  expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
    label: 'main',
    requestId: 'r1',
    payload: {},
  });
});

it('on cli:pane-minimize with focused but no focused pane, replies error', async () => {
  const setPaneCollapsed = vi.fn();
  await startCliBridge({
    label: 'main',
    splitPane: vi.fn(),
    closePane: vi.fn(),
    indexOf: vi.fn(),
    resolveFocusedPaneId: () => null,
    setPaneCollapsed,
  });

  listeners.get('cli:pane-minimize')!({
    payload: { requestId: 'r2', paneId: 'focused', minimized: false },
  });

  expect(setPaneCollapsed).not.toHaveBeenCalled();
  expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
    label: 'main',
    requestId: 'r2',
    payload: { error: 'no_focused_pane' },
  });
});

it('on cli:pane-split with minimized=true, sets new pane collapsed before resolving', async () => {
  const setPaneCollapsed = vi.fn();
  const splitPane = vi.fn().mockReturnValue('pane-new');
  await startCliBridge({
    label: 'main',
    splitPane,
    closePane: vi.fn(),
    indexOf: vi.fn().mockReturnValue(2),
    resolveFocusedPaneId: () => 'pane-1',
    setPaneCollapsed,
  });

  listeners.get('cli:pane-split')!({
    payload: { requestId: 'r3', paneId: 'pane-1', direction: 'horizontal', minimized: true },
  });

  expect(splitPane).toHaveBeenCalledWith('pane-1', 'horizontal');
  expect(setPaneCollapsed).toHaveBeenCalledWith('pane-new', true);
  expect(invokeMock).toHaveBeenCalledWith('cli_resolve_pending', {
    label: 'main',
    requestId: 'r3',
    payload: { newPaneId: 'pane-new', newPaneIndex: 2 },
  });
});

it('on cli:pane-split without minimized, does not touch setPaneCollapsed', async () => {
  const setPaneCollapsed = vi.fn();
  await startCliBridge({
    label: 'main',
    splitPane: vi.fn().mockReturnValue('pane-new'),
    closePane: vi.fn(),
    indexOf: vi.fn().mockReturnValue(2),
    resolveFocusedPaneId: () => 'pane-1',
    setPaneCollapsed,
  });

  listeners.get('cli:pane-split')!({
    payload: { requestId: 'r4', paneId: 'pane-1', direction: 'horizontal' },
  });

  expect(setPaneCollapsed).not.toHaveBeenCalled();
});
```

The existing tests in the file already define `listeners` and `invokeMock` via mocks; reuse them.

- [ ] **Step 2: Run the failing tests**

```bash
npm run test -- --run cliBridge
```
Expected: FAIL on missing `setPaneCollapsed` dep / missing listener.

- [ ] **Step 3: Implement**

Extend `CliBridgeDeps`:

```typescript
export interface CliBridgeDeps {
  label: string;
  splitPane: (paneId: string, direction: 'horizontal' | 'vertical') => string;
  closePane: (paneId: string) => void;
  indexOf: (paneId: string) => number;
  resolveFocusedPaneId: () => string | null;
  setPaneCollapsed: (paneId: string, value: boolean) => void;
}
```

In `startCliBridge`, after the existing `unlistenClose`, add:

```typescript
const unlistenMinimize = await listen<{
  requestId: string;
  paneId: string;
  minimized: boolean;
}>('cli:pane-minimize', (event) => {
  const { requestId, paneId, minimized } = event.payload;
  const target = resolveTarget(paneId);
  if (!target) {
    reply(requestId, { error: 'no_focused_pane' });
    return;
  }
  deps.setPaneCollapsed(target, minimized);
  reply(requestId, {});
});
```

Update the existing `cli:pane-split` listener to handle a `minimized` field:

```typescript
const unlistenSplit = await listen<{
  requestId: string;
  paneId: string;
  direction: 'horizontal' | 'vertical';
  minimized?: boolean;
}>('cli:pane-split', (event) => {
  const { requestId, paneId, direction, minimized } = event.payload;
  const target = resolveTarget(paneId);
  if (!target) {
    reply(requestId, { error: 'no_focused_pane' });
    return;
  }
  const newPaneId = deps.splitPane(target, direction);
  if (minimized) deps.setPaneCollapsed(newPaneId, true);
  reply(requestId, { newPaneId, newPaneIndex: deps.indexOf(newPaneId) });
});
```

Return a teardown that includes `unlistenMinimize()`.

- [ ] **Step 4: Wire deps in `App.svelte`**

Inside `setupCliForProject`, add to the `startCliBridge` options:

```typescript
setPaneCollapsed: (paneId, value) => terminalStore.setCollapsed(paneId, value),
```

- [ ] **Step 5: Run the tests**

```bash
npm run test -- --run cliBridge
```
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/lib/services/cliBridge.ts src/lib/services/cliBridge.test.ts src/App.svelte
git commit -m "feat(cli-bridge): handle pane minimize/restore and split --minimized"
```

---

## Task 12: Add the collapse button and thin-bar styling to `TerminalShortcutBar.svelte`

**Files:**
- Modify: `src/lib/components/terminal/TerminalShortcutBar.svelte`
- Test: `src/lib/components/terminal/TerminalShortcutBar.browser.test.ts`

- [ ] **Step 1: Write the failing browser tests**

Append to `TerminalShortcutBar.browser.test.ts`:

```typescript
it('hides REPLY/CMD/PICK rows when collapsed=true', () => {
  const { queryByText } = render(TerminalShortcutBar, {
    props: { ...defaultProps, collapsed: true, onToggleCollapse: vi.fn() },
  });
  expect(queryByText('OK')).toBeNull();
  expect(queryByText('Continue')).toBeNull();
});

it('renders collapse button immediately before settings button', () => {
  const { container } = render(TerminalShortcutBar, {
    props: { ...defaultProps, collapsed: false, onToggleCollapse: vi.fn() },
  });
  const actions = container.querySelector('.bar-actions')!;
  const buttons = actions.querySelectorAll('button');
  expect(buttons.length).toBe(2);
  expect(buttons[0].classList.contains('collapse-btn')).toBe(true);
  expect(buttons[1].classList.contains('settings-btn')).toBe(true);
});

it('collapse button calls onToggleCollapse', async () => {
  const onToggleCollapse = vi.fn();
  const { container } = render(TerminalShortcutBar, {
    props: { ...defaultProps, collapsed: false, onToggleCollapse },
  });
  const btn = container.querySelector('.collapse-btn') as HTMLButtonElement;
  await fireEvent.click(btn);
  expect(onToggleCollapse).toHaveBeenCalledTimes(1);
});

it('collapse button title and aria-label swap on collapsed state', () => {
  const { container, rerender } = render(TerminalShortcutBar, {
    props: { ...defaultProps, collapsed: false, onToggleCollapse: vi.fn() },
  });
  const btn = container.querySelector('.collapse-btn') as HTMLButtonElement;
  expect(btn.title).toMatch(/minimize/i);

  rerender({ ...defaultProps, collapsed: true, onToggleCollapse: vi.fn() });
  expect((container.querySelector('.collapse-btn') as HTMLButtonElement).title).toMatch(
    /restore/i
  );
});
```

Update `defaultProps` at the top of the file to include the new props:

```typescript
const defaultProps = {
  visible: true,
  shortcuts: [/* unchanged */],
  showNumberRow: false,
  collapsed: false,
  onSend: vi.fn(),
  onSettingsClick: vi.fn(),
  onAddClick: vi.fn(),
  onToggleCollapse: vi.fn(),
};
```

- [ ] **Step 2: Run failing tests**

```bash
npm run test:browser -- --run TerminalShortcutBar
```
Expected: FAIL on missing prop / button.

- [ ] **Step 3: Implement props and markup**

Update `Props`:

```typescript
interface Props {
  visible: boolean;
  shortcuts: TerminalShortcut[];
  showNumberRow: boolean;
  collapsed: boolean;
  onSend: (text: string, withEnter: boolean) => void;
  onSettingsClick: () => void;
  onAddClick: (type: ShortcutType) => void;
  onToggleCollapse: () => void;
}
```

Destructure all six. Wrap the three `.shortcut-row` blocks with `{#if !collapsed} ... {/if}`. Replace `.bar-actions` content with:

```svelte
<div class="bar-actions">
  <button
    class="collapse-btn"
    onclick={onToggleCollapse}
    title={collapsed ? 'Restore shortcuts' : 'Minimize shortcuts'}
    aria-label={collapsed ? 'Restore shortcuts' : 'Minimize shortcuts'}
  >
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none"
         stroke="currentColor" stroke-width="2" stroke-linecap="round">
      {#if collapsed}
        <polyline points="6 15 12 9 18 15" />
      {:else}
        <line x1="5" y1="12" x2="19" y2="12" />
      {/if}
    </svg>
  </button>
  <button
    class="settings-btn"
    onclick={onSettingsClick}
    title="Shortcut Settings"
    aria-label="Shortcut Settings"
  >
    <!-- existing settings SVG -->
  </button>
</div>
```

In the `<style>` block, add a thin-bar variant and the collapse button:

```css
.shortcut-bar.collapsed {
  padding-top: 2px;
  padding-bottom: 2px;
  padding-right: 0;
  min-height: 28px;
}

.collapse-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 24px;
  height: 24px;
  padding: 0;
  color: var(--text-muted);
  background: transparent;
  border: 1px solid transparent;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition:
    color var(--transition-fast),
    background var(--transition-fast),
    border-color var(--transition-fast);
}

.collapse-btn:hover {
  color: var(--text-secondary);
  background: rgba(125, 211, 252, 0.08);
  border-color: var(--border-color);
}

.collapse-btn:active {
  transform: scale(0.92);
}
```

Bind the modifier class:

```svelte
<div class="shortcut-bar" class:collapsed>
```

- [ ] **Step 4: Run tests**

```bash
npm run test:browser -- --run TerminalShortcutBar
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/terminal/TerminalShortcutBar.svelte \
       src/lib/components/terminal/TerminalShortcutBar.browser.test.ts
git commit -m "feat(terminal-shortcut-bar): add collapse button and thin-bar layout"
```

---

## Task 13: Subscribe `Terminal.svelte` to collapsed state and pass through to the bar

**Files:**
- Modify: `src/lib/components/terminal/Terminal.svelte`

- [ ] **Step 1: Subscribe via existing `terminalStore.subscribe`**

Near the top of `<script lang="ts">`, alongside the other reactive sources:

```typescript
let collapsed = $state(false);
const unsubscribeCollapsed = terminalStore.subscribe(() => {
  collapsed = terminalStore.isCollapsed(paneId);
});
```

Tear down in the existing `onDestroy` block:

```typescript
unsubscribeCollapsed();
```

- [ ] **Step 2: Pass props to `TerminalShortcutBar`**

Inside the existing usage, add:

```svelte
<TerminalShortcutBar
  visible={isAiRunning}
  shortcuts={shortcutState.allShortcuts}
  showNumberRow={numberRowEnabled}
  {collapsed}
  onSend={handleShortcutSend}
  onSettingsClick={() => {
    shortcutFocusSection = null;
    showShortcutSettings = true;
  }}
  onAddClick={handleShortcutAddClick}
  onToggleCollapse={() => terminalStore.toggleCollapsed(paneId)}
/>
```

- [ ] **Step 3: Type-check**

```bash
npm run check
```
Expected: 0 errors.

- [ ] **Step 4: Run the existing browser tests to confirm no regressions**

```bash
npm run test:browser -- --run TerminalShortcutBar
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/terminal/Terminal.svelte
git commit -m "feat(terminal): wire shortcut-bar collapse toggle through Terminal.svelte"
```

---

## Task 14: Update the `kiri-cli` skill documentation

**Files:**
- Modify: `resources/skills/kiri-cli/SKILL.md`

- [ ] **Step 1: Add subsections to §5**

Insert these blocks before the `close` subsection (preserve the `---` separators used by the surrounding doc):

```markdown
### `kiri term minimize [--pane X]`

Collapse the pane's shortcut bar to a thin strip with only restore and
settings buttons.

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

Expand a previously minimized shortcut bar.

```bash
kiri term restore --pane pane-2
```

Response shape:

```json
{ "type": "restore" }
```
```

- [ ] **Step 2: Update the `split` subsection**

In the existing `### kiri term split` block, replace the synopsis line and add the new flag:

```markdown
### `kiri term split [--pane X] [--dir h|v] [--minimized]`

Split the pane. `--dir h` (default) is horizontal; `--dir v` is vertical.
`--minimized` creates the new pane with its shortcut bar already
collapsed.

```bash
kiri term split
kiri term split --dir v
kiri term split --pane pane-1 --dir h
kiri term split --dir v --minimized
```
```

- [ ] **Step 3: Update the `ls` response example**

In §5 `### kiri term ls` AND in §7 `### ls response`, add `"minimized": false` to each PaneInfo block. Example (for both spots):

```json
{
  "index": 0,
  "id": "pane-1",
  "terminal_id": 1,
  "cwd": "/Users/user/project",
  "process_name": "zsh",
  "running": false,
  "memory_bytes": 4096000,
  "focused": true,
  "minimized": false
}
```

- [ ] **Step 4: Add a best-practice bullet to §8**

Append to the bullet list:

```markdown
- When spawning a new pane via `kiri term split` for the agent's own
  use (background dev server, log tail, parallel run), prefer
  `--minimized` so the new pane comes up with its shortcut bar
  collapsed. This keeps the user's primary view from being pushed
  down. The user (or `kiri term restore --pane <id>`) can expand it
  at any time.
```

- [ ] **Step 5: Commit**

```bash
git add resources/skills/kiri-cli/SKILL.md
git commit -m "docs(kiri-cli): document minimize/restore commands and split --minimized"
```

---

## Task 15: Manual integration check

**Files:** none (verification only)

- [ ] **Step 1: Run the full test suite**

```bash
npm run check
npm run lint
npm run test -- --run
npm run test:browser -- --run
cargo test
```
All five must pass.

- [ ] **Step 2: Run the dev app and exercise the feature**

```bash
npm run tauri dev
```

Inside the dev window:

1. Open a project, start `claude` (the bar should appear expanded).
2. Click the new `−` button — bar collapses to a thin strip with `↑` then `⚙`.
3. Click the `↑` — bar expands, REPLY/CMD/PICK rows return.
4. Open a kiri terminal pane (Cmd+T or whatever the existing shortcut is) and run, in the running `claude` session:
   ```bash
   kiri term ls
   ```
   Each `panes[i]` should include `"minimized": false|true` matching reality.
5. Run `kiri term minimize --pane pane-1`. The pane's bar collapses.
6. Run `kiri term restore --pane pane-1`. The pane's bar expands.
7. Run `kiri term split --dir v --minimized`. A new pane appears with its bar already collapsed.

If any step fails, note which task touches the failing path and fix in a follow-up commit — do not amend prior commits.

- [ ] **Step 3: Stop the dev app cleanly**

`Ctrl-C` in the dev terminal, or close the window from the OS UI.

- [ ] **Step 4: No commit needed** — verification only.

---

## Self-review checklist (performed by plan author)

- Spec §UX → Task 12 (collapse button + thin strip layout).
- Spec §State → Task 9 (terminalStore collapsed map + cleanup) + Task 13 (Terminal.svelte subscribe).
- Spec §CLI wire protocol → Task 1.
- Spec §CLI args → Task 2 + Task 3.
- Spec §Pane map cache → Task 4 + Task 8.
- Spec §Handlers (minimize/restore) → Task 6.
- Spec §Handlers (split --minimized) → Task 7.
- Spec §Frontend event handling → Task 11.
- Spec §ls reflection → Task 5 + Task 10.
- Spec §Skill → Task 14.
- Spec §Tests (Rust) → Tasks 1, 2, 4. The handler tests are intentionally exercised end-to-end in Task 15's manual run; adding `handlers::minimize` unit tests requires a fake `AppHandle` and bridges that don't exist yet — out of scope, captured under spec's "Risks and edge cases".
- Spec §Tests (Frontend) → Tasks 9, 11, 12.
- Placeholder scan: no TBD/TODO/"add appropriate" patterns.
- Type consistency: `setPaneCollapsed` (cliBridge dep), `setCollapsed` (store method), `collapsedByPaneId` (store snapshot), `collapsed` (PaneEntry / wire / Svelte prop), `minimized` (wire PaneInfo / Tauri event payload / CLI flag). The wire field is `minimized` everywhere it crosses a process boundary; `collapsed` is the in-Rust / in-store internal name. This split is intentional and consistently applied — confirmed.
