# Terminal Pane Name & Color Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `--name` / `--color` flags to `kiri term split` and surface them as a `●name` label in each terminal pane's header. `kiri term ls` also returns the new fields.

**Architecture:** Frontend `terminalStore` is the source of truth; name/color attach to the new leaf at split time, ride the existing `cli_update_pane_map` invoke to the Rust `PaneMap`, and are returned verbatim by the `ls` handler. Six fixed colors (`sky | iris | jade | amber | coral | rose`) are CSS tokens drawn from kiri's Mist palette.

**Tech Stack:** Rust (clap, serde, tokio, tauri), Svelte 5 + TypeScript, Vitest (unit + browser), CSS variables.

**Spec:** `docs/features/20260513-terminal-pane-name-color.md`

---

## Conventions

- All commits use conventional-commit prefixes (`feat:`, `test:`, `refactor:`). Write commit messages in English.
- Run rust tests via `npm run test:rust` (which calls `cargo test --workspace`) unless a task needs a single crate (`cargo test -p kiri-cli-proto`).
- Run frontend tests via `npm run test -- <file>` (unit) or `npm run test:browser -- <file>` (browser).
- Lint: `npm run lint` before committing. The pre-commit hook runs it anyway.
- Do not amend commits. New change → new commit.

---

## Task 1: Add `PaneColor` enum to the proto crate

**Files:**
- Modify: `crates/kiri-cli-proto/src/types.rs`
- Modify: `crates/kiri-cli-proto/src/lib.rs`

- [ ] **Step 1: Append a failing test at the bottom of the existing `mod tests` block in `types.rs`**

Add inside `mod tests { … }`:

```rust
    #[test]
    fn pane_color_serializes_snake_case() {
        let v = serde_json::to_value(PaneColor::Sky).unwrap();
        assert_eq!(v, serde_json::Value::String("sky".into()));
        let v = serde_json::to_value(PaneColor::Coral).unwrap();
        assert_eq!(v, serde_json::Value::String("coral".into()));
    }

    #[test]
    fn pane_color_deserializes_known() {
        let c: PaneColor = serde_json::from_value(serde_json::json!("amber")).unwrap();
        assert_eq!(c, PaneColor::Amber);
    }

    #[test]
    fn pane_color_rejects_unknown() {
        let r: Result<PaneColor, _> = serde_json::from_value(serde_json::json!("magenta"));
        assert!(r.is_err());
    }
```

- [ ] **Step 2: Run the tests and confirm they fail with `unresolved name PaneColor`**

```
cargo test -p kiri-cli-proto pane_color
```

Expected: build failure, "cannot find type `PaneColor` in this scope".

- [ ] **Step 3: Add `PaneColor` above the existing `PaneRef` definition in `types.rs`**

Add (above `pub enum PaneRef` or at a sensible spot):

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

- [ ] **Step 4: Re-export `PaneColor` from `lib.rs`**

Modify the existing `pub use types::{ErrorCode, PaneRef, SplitDirection};` line to:

```rust
pub use types::{ErrorCode, PaneColor, PaneRef, SplitDirection};
```

- [ ] **Step 5: Run the tests again — they should pass**

```
cargo test -p kiri-cli-proto pane_color
```

Expected: 3 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/kiri-cli-proto/src/types.rs crates/kiri-cli-proto/src/lib.rs
git commit -m "feat(proto): add PaneColor enum for pane labels"
```

---

## Task 2: Extend `Request::Split` with optional `name` and `color`

**Files:**
- Modify: `crates/kiri-cli-proto/src/wire.rs`

- [ ] **Step 1: Write failing tests at the bottom of `mod tests` in `wire.rs`**

```rust
    #[test]
    fn request_split_with_name_and_color_roundtrip() {
        roundtrip(&Request::Split {
            pane: PaneRef::focused(),
            direction: SplitDirection::Horizontal,
            name: Some("build".into()),
            color: Some(crate::PaneColor::Coral),
        });
    }

    #[test]
    fn request_split_without_label_omits_fields() {
        let v = serde_json::to_value(Request::Split {
            pane: PaneRef::focused(),
            direction: SplitDirection::Horizontal,
            name: None,
            color: None,
        })
        .unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("name"));
        assert!(!obj.contains_key("color"));
    }

    #[test]
    fn request_split_back_compat_without_fields_parses() {
        let parsed: Request = serde_json::from_value(
            serde_json::json!({ "type": "split", "pane": "focused", "direction": "vertical" }),
        )
        .unwrap();
        assert_eq!(
            parsed,
            Request::Split {
                pane: PaneRef::focused(),
                direction: SplitDirection::Vertical,
                name: None,
                color: None,
            }
        );
    }
```

- [ ] **Step 2: Run tests and observe failure**

```
cargo test -p kiri-cli-proto request_split
```

Expected: compile errors (missing fields).

- [ ] **Step 3: Extend `Request::Split` in `wire.rs`**

Locate `Request::Split { pane, direction }` (around line 33) and change to:

```rust
    Split {
        pane: PaneRef,
        direction: SplitDirection,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        color: Option<crate::PaneColor>,
    },
```

Also update the existing `request_split_roundtrip` test (around line 140) — add `name: None, color: None,` to make it still compile.

- [ ] **Step 4: Run tests, they should pass**

```
cargo test -p kiri-cli-proto request_split
```

Expected: 4 passed (the 3 new + the existing one).

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-cli-proto/src/wire.rs
git commit -m "feat(proto): add optional name/color to Request::Split"
```

---

## Task 3: Extend `PaneInfo` with optional `name` and `color`

**Files:**
- Modify: `crates/kiri-cli-proto/src/wire.rs`

- [ ] **Step 1: Add failing tests to `mod tests`**

```rust
    #[test]
    fn pane_info_with_label_roundtrip() {
        let info = PaneInfo {
            index: 0,
            id: "pane-1".into(),
            terminal_id: 1,
            cwd: Some("/p".into()),
            process_name: "zsh".into(),
            running: false,
            memory_bytes: 0,
            focused: true,
            name: Some("agent".into()),
            color: Some(crate::PaneColor::Iris),
        };
        let s = serde_json::to_string(&info).unwrap();
        let back: PaneInfo = serde_json::from_str(&s).unwrap();
        assert_eq!(back.name.as_deref(), Some("agent"));
        assert_eq!(back.color, Some(crate::PaneColor::Iris));
    }

    #[test]
    fn pane_info_without_label_omits_fields() {
        let info = PaneInfo {
            index: 0,
            id: "pane-1".into(),
            terminal_id: 1,
            cwd: None,
            process_name: "zsh".into(),
            running: false,
            memory_bytes: 0,
            focused: false,
            name: None,
            color: None,
        };
        let v = serde_json::to_value(&info).unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("name"));
        assert!(!obj.contains_key("color"));
    }
```

- [ ] **Step 2: Run, observe failure**

```
cargo test -p kiri-cli-proto pane_info
```

Expected: compile error (missing fields).

- [ ] **Step 3: Add fields to `PaneInfo`**

Modify struct around line 47 to:

```rust
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
    pub color: Option<crate::PaneColor>,
}
```

- [ ] **Step 4: Run, observe pass**

```
cargo test -p kiri-cli-proto
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add crates/kiri-cli-proto/src/wire.rs
git commit -m "feat(proto): add optional name/color to PaneInfo"
```

---

## Task 4: Add `PaneColorArg` clap enum + name validator to CLI

**Files:**
- Modify: `crates/kiri-cli/src/cli.rs`

- [ ] **Step 1: Add failing tests in `crates/kiri-cli/src/cli.rs` `mod tests`**

Locate the existing `mod tests { ... }` at the bottom and add:

```rust
    #[test]
    fn parses_valid_color() {
        let cli = Cli::try_parse_from([
            "kiri", "term", "split", "--color", "coral",
        ])
        .unwrap();
        let Top::Term(TermCmd::Split(a)) = cli.command else {
            panic!("expected split");
        };
        assert_eq!(a.color, Some(PaneColorArg::Coral));
    }

    #[test]
    fn rejects_unknown_color() {
        let err = Cli::try_parse_from([
            "kiri", "term", "split", "--color", "magenta",
        ]);
        assert!(err.is_err(), "should reject unknown color");
    }

    #[test]
    fn parses_valid_name() {
        let cli = Cli::try_parse_from([
            "kiri", "term", "split", "--name", "build",
        ])
        .unwrap();
        let Top::Term(TermCmd::Split(a)) = cli.command else {
            panic!("expected split");
        };
        assert_eq!(a.name.as_deref(), Some("build"));
    }

    #[test]
    fn rejects_empty_name() {
        let err = Cli::try_parse_from(["kiri", "term", "split", "--name", ""]);
        assert!(err.is_err(), "should reject empty name");
    }

    #[test]
    fn rejects_name_over_32_chars() {
        let long = "a".repeat(33);
        let err = Cli::try_parse_from(["kiri", "term", "split", "--name", &long]);
        assert!(err.is_err(), "should reject 33-char name");
    }

    #[test]
    fn accepts_name_at_32_chars() {
        let edge = "a".repeat(32);
        let cli = Cli::try_parse_from(["kiri", "term", "split", "--name", &edge]).unwrap();
        let Top::Term(TermCmd::Split(a)) = cli.command else {
            panic!("expected split");
        };
        assert_eq!(a.name.as_ref().unwrap().chars().count(), 32);
    }

    #[test]
    fn rejects_control_char_name() {
        let err = Cli::try_parse_from(["kiri", "term", "split", "--name", "ab\nc"]);
        assert!(err.is_err(), "should reject name with newline");
    }
```

- [ ] **Step 2: Run and observe failure**

```
cargo test -p kiri-cli
```

Expected: compile error (PaneColorArg undefined, SplitArgs has no name/color).

- [ ] **Step 3: Add the `PaneColorArg` enum and validator in `cli.rs`**

After the existing `use` block at top of `cli.rs`, replace `use clap::{Args, Parser, Subcommand};` with:

```rust
use clap::{Args, Parser, Subcommand, ValueEnum};
use kiri_cli_proto::{PaneColor, PaneRef};
```

Then add (anywhere — e.g. just below the imports):

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum PaneColorArg {
    Sky,
    Iris,
    Jade,
    Amber,
    Coral,
    Rose,
}

impl From<PaneColorArg> for PaneColor {
    fn from(a: PaneColorArg) -> Self {
        match a {
            PaneColorArg::Sky => PaneColor::Sky,
            PaneColorArg::Iris => PaneColor::Iris,
            PaneColorArg::Jade => PaneColor::Jade,
            PaneColorArg::Amber => PaneColor::Amber,
            PaneColorArg::Coral => PaneColor::Coral,
            PaneColorArg::Rose => PaneColor::Rose,
        }
    }
}

/// Clap value parser for `--name`.
///
/// Rules: non-empty, ≤32 chars (counting Unicode scalar values), no
/// ASCII control characters (\x00–\x1f or \x7f). Newlines or NULs
/// would otherwise become part of the terminal header.
fn parse_pane_name(s: &str) -> Result<String, String> {
    if s.is_empty() {
        return Err("name must not be empty".into());
    }
    if s.chars().count() > 32 {
        return Err("name must be 32 characters or fewer".into());
    }
    if s.chars().any(|c| (c.is_control())) {
        return Err("name must not contain control characters".into());
    }
    Ok(s.to_owned())
}
```

- [ ] **Step 4: Extend `SplitArgs`**

Locate `pub struct SplitArgs` (around line 99) and replace with:

```rust
#[derive(Args, Debug)]
pub struct SplitArgs {
    #[command(flatten)]
    pub pane: PaneOpt,
    /// Split direction: h (horizontal) or v (vertical).
    #[arg(long, default_value = "h")]
    pub dir: String,
    /// Optional pane label shown in the terminal header (1–32 chars, no control characters).
    #[arg(long, value_parser = parse_pane_name)]
    pub name: Option<String>,
    /// Optional pane color shown in the terminal header.
    #[arg(long, value_enum)]
    pub color: Option<PaneColorArg>,
}
```

- [ ] **Step 5: Run and verify all tests pass**

```
cargo test -p kiri-cli
```

Expected: all green (including the existing `parse_pane_*` tests).

- [ ] **Step 6: Commit**

```bash
git add crates/kiri-cli/src/cli.rs
git commit -m "feat(cli): add --name and --color flags to term split"
```

---

## Task 5: Pass `name`/`color` into the wire request in `main.rs`

**Files:**
- Modify: `crates/kiri-cli/src/main.rs`

- [ ] **Step 1: Update the import line**

Change:

```rust
use kiri_cli_proto::{Request, Response, SplitDirection};
```

to:

```rust
use kiri_cli_proto::{PaneColor, Request, Response, SplitDirection};
```

- [ ] **Step 2: Extend the `TermCmd::Split` arm of `build_request`**

Locate (around line 156):

```rust
        TermCmd::Split(a) => Request::Split {
            pane: cli::parse_pane(&a.pane),
            direction: match a.dir.to_lowercase().as_str() {
                "v" | "vertical" => SplitDirection::Vertical,
                _ => SplitDirection::Horizontal,
            },
        },
```

Replace with:

```rust
        TermCmd::Split(a) => Request::Split {
            pane: cli::parse_pane(&a.pane),
            direction: match a.dir.to_lowercase().as_str() {
                "v" | "vertical" => SplitDirection::Vertical,
                _ => SplitDirection::Horizontal,
            },
            name: a.name.clone(),
            color: a.color.map(PaneColor::from),
        },
```

- [ ] **Step 3: Build the CLI**

```
cargo build -p kiri-cli
```

Expected: clean build.

- [ ] **Step 4: Commit**

```bash
git add crates/kiri-cli/src/main.rs
git commit -m "feat(cli): forward --name/--color into Request::Split"
```

---

## Task 6: Extend `PaneEntry` with optional `name`/`color`

**Files:**
- Modify: `src-tauri/src/commands/cli_server/pane_map.rs`

- [ ] **Step 1: Add a failing test inside the existing `mod tests` block in `pane_map.rs`**

```rust
    #[test]
    fn entry_with_name_color_roundtrips() {
        let e = PaneEntry {
            index: 0,
            pane_id: "pane-1".into(),
            terminal_id: 1,
            focused: true,
            name: Some("build".into()),
            color: Some(kiri_cli_proto::PaneColor::Coral),
        };
        let s = serde_json::to_string(&e).unwrap();
        let back: PaneEntry = serde_json::from_str(&s).unwrap();
        assert_eq!(back.name.as_deref(), Some("build"));
        assert_eq!(back.color, Some(kiri_cli_proto::PaneColor::Coral));
    }

    #[test]
    fn entry_without_label_omits_fields_in_json() {
        let e = entry(0, "pane-1", 1, true);
        let v = serde_json::to_value(&e).unwrap();
        let obj = v.as_object().unwrap();
        assert!(!obj.contains_key("name"));
        assert!(!obj.contains_key("color"));
    }
```

Also update the `entry` test helper (currently around line 68) so it sets `name: None, color: None`. After updating the struct in Step 3 you'll need to add those fields there too — do it in the same edit to keep the test file compiling.

- [ ] **Step 2: Run, observe compile failure**

```
cd src-tauri && cargo test --lib commands::cli_server::pane_map
```

Expected: missing fields.

- [ ] **Step 3: Extend `PaneEntry`**

Change (top of file, around line 12):

```rust
pub struct PaneEntry {
    pub index: u32,
    pub pane_id: String,
    pub terminal_id: u32,
    pub focused: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<kiri_cli_proto::PaneColor>,
}
```

Update the `entry()` test helper at the bottom of the same file (around line 68) to:

```rust
    fn entry(index: u32, pane_id: &str, terminal_id: u32, focused: bool) -> PaneEntry {
        PaneEntry {
            index,
            pane_id: pane_id.into(),
            terminal_id,
            focused,
            name: None,
            color: None,
        }
    }
```

- [ ] **Step 4: Run, observe pass**

```
cd src-tauri && cargo test --lib commands::cli_server::pane_map
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/cli_server/pane_map.rs
git commit -m "feat(cli-server): add optional name/color to PaneEntry"
```

---

## Task 7: Update `ls` handler to copy `name`/`color` through

**Files:**
- Modify: `src-tauri/src/commands/cli_server/handlers.rs`

- [ ] **Step 1: Add a failing test in `handlers.rs`'s `mod tests` block**

Locate the existing `ls_with_no_panes_returns_empty_list` test as a template. Add after it:

```rust
    #[tokio::test]
    async fn ls_returns_name_and_color_when_present() {
        use kiri_cli_proto::{PaneColor, Response};

        let pane_map = std::sync::Arc::new(super::super::pane_map::PaneMap::new());
        pane_map.replace(vec![super::super::pane_map::PaneEntry {
            index: 0,
            pane_id: "pane-1".into(),
            terminal_id: 1,
            focused: true,
            name: Some("agent".into()),
            color: Some(PaneColor::Iris),
        }]);
        let ctx = DispatchContext {
            label: "test".into(),
            app: None,
            terminals: std::sync::Arc::new(std::sync::Mutex::new(
                crate::commands::terminal::TerminalManager::new(),
            )),
            bus: tokio::sync::broadcast::channel(16).0,
            pane_map,
            pending: std::sync::Arc::new(crate::commands::cli_server::frontend_bridge::PendingReplies::default()),
        };
        let resp = ls(&ctx).await;
        let Response::Ls { panes } = resp else { panic!("expected ls") };
        assert_eq!(panes.len(), 1);
        assert_eq!(panes[0].name.as_deref(), Some("agent"));
        assert_eq!(panes[0].color, Some(PaneColor::Iris));
    }
```

**Note:** If the existing `ls_with_no_panes_returns_empty_list` test already builds a `DispatchContext` in a more direct way, mirror that exact form rather than the one above. Inspect the file first.

- [ ] **Step 2: Run, observe failure**

```
cd src-tauri && cargo test --lib commands::cli_server::handlers::tests::ls_returns_name_and_color
```

Expected: assertion failure (fields are `None`) or compile error if I guessed the constructor wrong — adjust to match the existing test.

- [ ] **Step 3: Update `ls` in `handlers.rs` (around line 28)**

Change the `panes.push(...)` call inside the loop to include the two fields. The complete loop body becomes:

```rust
    for e in entries {
        let (process_name, memory_bytes, running) =
            process_info_for(&ctx.terminals, e.terminal_id);
        let cwd = cwd_for(&ctx.terminals, e.terminal_id);
        panes.push(kiri_cli_proto::PaneInfo {
            index: e.index,
            id: e.pane_id,
            terminal_id: e.terminal_id,
            cwd,
            process_name,
            running,
            memory_bytes,
            focused: e.focused,
            name: e.name,
            color: e.color,
        });
    }
```

- [ ] **Step 4: Run, observe pass**

```
cd src-tauri && cargo test --lib commands::cli_server::handlers
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/cli_server/handlers.rs
git commit -m "feat(cli-server): include name/color in ls response"
```

---

## Task 8: Update `split` handler to forward `name`/`color`

**Files:**
- Modify: `src-tauri/src/commands/cli_server/handlers.rs`

- [ ] **Step 1: Update the dispatch in `handle()`**

Locate (line 22):

```rust
        Request::Split { pane, direction } => vec![split(ctx, pane, direction).await],
```

Replace with:

```rust
        Request::Split {
            pane,
            direction,
            name,
            color,
        } => vec![split(ctx, pane, direction, name, color).await],
```

- [ ] **Step 2: Update the `split` fn signature and payload**

Locate `async fn split(ctx: &DispatchContext, p: PaneRef, direction: SplitDirection)` (around line 317) and change to:

```rust
async fn split(
    ctx: &DispatchContext,
    p: PaneRef,
    direction: SplitDirection,
    name: Option<String>,
    color: Option<kiri_cli_proto::PaneColor>,
) -> Response {
```

Inside the function, replace the `payload` construction (around line 326) with:

```rust
    let payload = serde_json::json!({
        "requestId": request_id,
        "paneId": pane.pane_id,
        "direction": match direction {
            SplitDirection::Horizontal => "horizontal",
            SplitDirection::Vertical => "vertical",
        },
        "name": name,
        "color": color,
    });
```

(`serde_json::json!` already encodes `None` as `null` and `Some(PaneColor::Coral)` as `"coral"` via the enum's existing Serialize impl. Frontend treats both `null` and undefined as "no label".)

- [ ] **Step 3: Build**

```
cd src-tauri && cargo build --lib
```

Expected: clean build. If a previously-existing split handler test still passes `split(ctx, pane, dir)` with the old signature, update it to add `None, None`.

- [ ] **Step 4: Run the existing tests**

```
cd src-tauri && cargo test --lib commands::cli_server::handlers
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/cli_server/handlers.rs
git commit -m "feat(cli-server): forward name/color in cli:pane-split event"
```

---

## Task 9: Add `PaneColor` type + extend `splitPane` in `terminalStore`

**Files:**
- Modify: `src/lib/stores/terminalStore.ts`
- Modify: `src/lib/stores/terminalStore.test.ts`

- [ ] **Step 1: Add failing tests at the bottom of the existing `describe('splitPane', ...)` block in `terminalStore.test.ts`**

```ts
    it('attaches name to the new pane when opts.name is given', () => {
      terminalStore.init();
      const state1 = get(terminalStore);
      const rootPaneId = (state1.rootPane as TerminalPaneLeaf).id;
      const newId = terminalStore.splitPane(rootPaneId, 'vertical', { name: 'build' });
      const state2 = get(terminalStore);
      const ids = getAllPaneIds(state2.rootPane!);
      expect(ids).toContain(newId);
      const split = state2.rootPane as TerminalPaneSplit;
      const newLeaf = split.children.find(
        (c) => c.type === 'terminal' && c.id === newId,
      ) as TerminalPaneLeaf;
      expect(newLeaf.name).toBe('build');
      expect(newLeaf.color).toBeUndefined();
    });

    it('attaches color to the new pane when opts.color is given', () => {
      terminalStore.init();
      const state1 = get(terminalStore);
      const rootPaneId = (state1.rootPane as TerminalPaneLeaf).id;
      const newId = terminalStore.splitPane(rootPaneId, 'vertical', { color: 'jade' });
      const state2 = get(terminalStore);
      const split = state2.rootPane as TerminalPaneSplit;
      const newLeaf = split.children.find(
        (c) => c.type === 'terminal' && c.id === newId,
      ) as TerminalPaneLeaf;
      expect(newLeaf.color).toBe('jade');
      expect(newLeaf.name).toBeUndefined();
    });

    it('leaves the original pane unlabeled even when child has name/color', () => {
      terminalStore.init();
      const state1 = get(terminalStore);
      const rootPaneId = (state1.rootPane as TerminalPaneLeaf).id;
      terminalStore.splitPane(rootPaneId, 'vertical', { name: 'build', color: 'coral' });
      const state2 = get(terminalStore);
      const split = state2.rootPane as TerminalPaneSplit;
      const original = split.children.find(
        (c) => c.type === 'terminal' && c.id === rootPaneId,
      ) as TerminalPaneLeaf;
      expect(original.name).toBeUndefined();
      expect(original.color).toBeUndefined();
    });
```

If `TerminalPaneSplit` isn't already imported in this test file, add it to the existing import from `'./terminalStore'`.

- [ ] **Step 2: Run, observe failure**

```
npm run test -- src/lib/stores/terminalStore.test.ts
```

Expected: type error `Argument of type … is not assignable` and/or property access errors.

- [ ] **Step 3: Update `terminalStore.ts`**

Near the top of the file (after imports), add:

```ts
export type PaneColor = 'sky' | 'iris' | 'jade' | 'amber' | 'coral' | 'rose';
```

Extend `TerminalPaneLeaf`:

```ts
export interface TerminalPaneLeaf {
  type: 'terminal';
  id: string;
  terminalId: number | null;
  cwd?: string | null;
  name?: string;
  color?: PaneColor;
}
```

Change `splitPaneInTree` signature so it can carry opts down. Replace (line 56-105 region):

```ts
function splitPaneInTree(
  pane: TerminalPane,
  targetPaneId: string,
  direction: 'horizontal' | 'vertical',
  newPaneId: string,
  newPaneOpts: { name?: string; color?: PaneColor } = {},
): TerminalPane {
  if (pane.type === 'terminal') {
    if (pane.id === targetPaneId) {
      return {
        type: 'split',
        id: generateSplitId(),
        direction,
        children: [
          pane,
          { type: 'terminal', id: newPaneId, terminalId: null, ...newPaneOpts },
        ],
        sizes: [50, 50],
      };
    }
    return pane;
  }

  if (pane.direction === direction) {
    const targetIndex = pane.children.findIndex(
      (child) => child.type === 'terminal' && child.id === targetPaneId,
    );

    if (targetIndex !== -1) {
      const newChildren = [...pane.children];
      newChildren.splice(targetIndex + 1, 0, {
        type: 'terminal',
        id: newPaneId,
        terminalId: null,
        ...newPaneOpts,
      });

      const equalSize = 100 / newChildren.length;
      const newSizes = newChildren.map(() => equalSize);

      return {
        ...pane,
        children: newChildren,
        sizes: newSizes,
      };
    }
  }

  return {
    ...pane,
    children: pane.children.map((child) =>
      splitPaneInTree(child, targetPaneId, direction, newPaneId, newPaneOpts),
    ),
  };
}
```

Update the public `splitPane` (around line 247):

```ts
    splitPane: (
      paneId: string,
      direction: 'horizontal' | 'vertical',
      opts: { name?: string; color?: PaneColor } = {},
    ): string => {
      const newPaneId = generatePaneId();
      update((state) => {
        if (!state.rootPane) return state;
        return {
          rootPane: splitPaneInTree(state.rootPane, paneId, direction, newPaneId, opts),
        };
      });
      return newPaneId;
    },
```

- [ ] **Step 4: Run tests, expect pass**

```
npm run test -- src/lib/stores/terminalStore.test.ts
```

Expected: all green.

- [ ] **Step 5: Commit**

```bash
git add src/lib/stores/terminalStore.ts src/lib/stores/terminalStore.test.ts
git commit -m "feat(store): support name/color on terminal pane leaves"
```

---

## Task 10: Forward `name`/`color` through `cliBridge`

**Files:**
- Modify: `src/lib/services/cliBridge.ts`
- Modify: `src/lib/services/cliBridge.test.ts`

- [ ] **Step 1: Add failing tests at the end of `cliBridge.test.ts`**

```ts
  it('on cli:pane-split with name/color, passes them to splitPane', async () => {
    const splitPane = vi.fn(() => 'np');
    const handler = await captureSplitListener({
      splitPane,
      resolveFocusedPaneId: () => 'fp',
    });
    handler({
      payload: {
        requestId: 'r1',
        paneId: 'focused',
        direction: 'vertical',
        name: 'build',
        color: 'coral',
      },
    });
    expect(splitPane).toHaveBeenCalledWith('fp', 'vertical', { name: 'build', color: 'coral' });
  });

  it('on cli:pane-split with neither name nor color, omits opts cleanly', async () => {
    const splitPane = vi.fn(() => 'np');
    const handler = await captureSplitListener({
      splitPane,
      resolveFocusedPaneId: () => 'fp',
    });
    handler({
      payload: {
        requestId: 'r1',
        paneId: 'focused',
        direction: 'horizontal',
      },
    });
    expect(splitPane).toHaveBeenCalledWith('fp', 'horizontal', { name: undefined, color: undefined });
  });
```

**Note:** The existing test file already has a helper-ish pattern. Look at `cli:pane-split` test setup (line ~25) and either reuse the existing scaffolding or extract a small `captureSplitListener` helper. If extracting, keep the helper local to the test file.

- [ ] **Step 2: Update the other existing splitPane assertions** that call `expect(splitPane).toHaveBeenCalledWith('p0', 'horizontal')` to expect the 3rd arg too:

```ts
expect(splitPane).toHaveBeenCalledWith('p0', 'horizontal', { name: undefined, color: undefined });
```

(There are two such assertions, lines ~42 and ~88. Update both.)

- [ ] **Step 3: Run, observe failure**

```
npm run test -- src/lib/services/cliBridge.test.ts
```

Expected: type errors (3rd arg) and assertion mismatches.

- [ ] **Step 4: Update `cliBridge.ts`**

Change the `CliBridgeDeps.splitPane` type:

```ts
export interface CliBridgeDeps {
  label: string;
  splitPane: (
    paneId: string,
    direction: 'horizontal' | 'vertical',
    opts?: { name?: string; color?: PaneColor },
  ) => string;
  closePane: (paneId: string) => void;
  indexOf: (paneId: string) => number;
  resolveFocusedPaneId: () => string | null;
}
```

Add an import at the top:

```ts
import type { PaneColor } from '@/lib/stores/terminalStore';
```

Update the listener payload type and the call to `deps.splitPane` (around lines 41–54):

```ts
  const unlistenSplit = await listen<{
    requestId: string;
    paneId: string;
    direction: 'horizontal' | 'vertical';
    name?: string;
    color?: PaneColor;
  }>('cli:pane-split', (event) => {
    const { requestId, paneId, direction, name, color } = event.payload;
    const target = resolveTarget(paneId);
    if (!target) {
      reply(requestId, { error: 'no_focused_pane' });
      return;
    }
    const newPaneId = deps.splitPane(target, direction, { name, color });
    reply(requestId, { newPaneId, newPaneIndex: deps.indexOf(newPaneId) });
  });
```

- [ ] **Step 5: Run, expect pass**

```
npm run test -- src/lib/services/cliBridge.test.ts
```

Expected: all green.

- [ ] **Step 6: Commit**

```bash
git add src/lib/services/cliBridge.ts src/lib/services/cliBridge.test.ts
git commit -m "feat(cli-bridge): forward name/color from split event to store"
```

---

## Task 11: Include `name`/`color` in pushed pane map (App.svelte)

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Update `collectPaneEntries` (around line 91)**

Replace the function body with:

```ts
  function collectPaneEntries(
    root: TerminalPane | null,
    focusedId: string | null,
  ): Array<{
    index: number;
    paneId: string;
    terminalId: number;
    focused: boolean;
    name?: string;
    color?: import('@/lib/stores/terminalStore').PaneColor;
  }> {
    if (!root) return [];
    const out: Array<{
      index: number;
      paneId: string;
      terminalId: number;
      focused: boolean;
      name?: string;
      color?: import('@/lib/stores/terminalStore').PaneColor;
    }> = [];
    let i = 0;
    const visit = (pane: TerminalPane) => {
      if (pane.type === 'terminal') {
        const terminalId = terminalStore.terminalIdFor(pane.id);
        if (terminalId !== null) {
          out.push({
            index: i++,
            paneId: pane.id,
            terminalId,
            focused: pane.id === focusedId,
            ...(pane.name !== undefined ? { name: pane.name } : {}),
            ...(pane.color !== undefined ? { color: pane.color } : {}),
          });
        }
      } else {
        for (const c of pane.children) visit(c);
      }
    };
    visit(root);
    return out;
  }
```

- [ ] **Step 2: Run frontend tests + typecheck**

```
npm run check
npm run test -- src/lib/services/cliBridge.test.ts src/lib/stores/terminalStore.test.ts
```

Expected: no type errors, tests still pass.

- [ ] **Step 3: Commit**

```bash
git add src/App.svelte
git commit -m "feat(app): include pane name/color in cli_update_pane_map"
```

---

## Task 12: Add pane-color CSS variables to `app.css`

**Files:**
- Modify: `src/app.css`

- [ ] **Step 1: Open `src/app.css` and find the `:root` block that defines `--accent-color`, `--bg-primary`, etc.**

- [ ] **Step 2: Add the six pane-color tokens inside that same `:root` block, after the existing `--accent3-color` line (or after the warning color section if separated)**

```css
  /* Pane identification colors (kiri term split --color) */
  --pane-color-sky: #7dd3fc;
  --pane-color-iris: #c4b5fd;
  --pane-color-jade: #86efac;
  --pane-color-amber: #fcd34d;
  --pane-color-coral: #fb923c;
  --pane-color-rose: #f9a8d4;
```

- [ ] **Step 3: Commit**

```bash
git add src/app.css
git commit -m "feat(theme): add six --pane-color-* tokens to root palette"
```

---

## Task 13: Wire `name`/`color` props through `TerminalContainer.svelte`

**Files:**
- Modify: `src/lib/components/terminal/TerminalContainer.svelte`

- [ ] **Step 1: Update the `<Terminal …/>` invocation (around line 130)**

Change from:

```svelte
  {#if pane.type === 'terminal'}
    <Terminal
      paneId={pane.id}
      cwd={pane.cwd || cwd}
      showControls={true}
      onSplitHorizontal={() => handleSplitHorizontal(pane.id)}
      onSplitVertical={() => handleSplitVertical(pane.id)}
      onClose={isOnlyPane ? undefined : () => handleClose(pane.id)}
    />
```

to:

```svelte
  {#if pane.type === 'terminal'}
    <Terminal
      paneId={pane.id}
      cwd={pane.cwd || cwd}
      name={pane.name}
      color={pane.color}
      showControls={true}
      onSplitHorizontal={() => handleSplitHorizontal(pane.id)}
      onSplitVertical={() => handleSplitVertical(pane.id)}
      onClose={isOnlyPane ? undefined : () => handleClose(pane.id)}
    />
```

- [ ] **Step 2: Typecheck**

```
npm run check
```

Expected: error pointing at `name`/`color` props on `Terminal.svelte` (not yet defined). That's fine — Task 14 adds them. Continue.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/terminal/TerminalContainer.svelte
git commit -m "feat(terminal): plumb pane name/color into Terminal component"
```

---

## Task 14: Add `name`/`color` props and pane-label markup to `Terminal.svelte`

**Files:**
- Modify: `src/lib/components/terminal/Terminal.svelte`

- [ ] **Step 1: Extend the `Props` interface (around line 37)**

Find:

```ts
  interface Props {
    paneId: string;
    cwd?: string | null;
    showControls?: boolean;
    onSplitHorizontal?: () => void;
    onSplitVertical?: () => void;
    onClose?: () => void;
  }
```

Replace with:

```ts
  import type { PaneColor } from '@/lib/stores/terminalStore';

  interface Props {
    paneId: string;
    cwd?: string | null;
    name?: string;
    color?: PaneColor;
    showControls?: boolean;
    onSplitHorizontal?: () => void;
    onSplitVertical?: () => void;
    onClose?: () => void;
  }
```

(If `PaneColor` is already exported, just add the prop fields and the import; don't double-add.)

- [ ] **Step 2: Update the `let { … }: Props = $props();` destructure (around line 46)**

```ts
  let {
    paneId,
    cwd = null,
    name = undefined,
    color = undefined,
    showControls = true,
    onSplitHorizontal,
    onSplitVertical,
    onClose,
  }: Props = $props();
```

- [ ] **Step 3: Add the pane-label markup inside `.terminal-controls`**

Find the `.terminal-controls` block (around line 913). After the second split button (the Split Horizontal one, ending around line 949) and before the `{#if worktreeInfo?.is_linked_worktree}` block, insert:

```svelte
      {#if name || color}
        <span class="pane-label" style:--pane-color={color ? `var(--pane-color-${color})` : 'transparent'}>
          {#if color}<span class="pane-dot" aria-hidden="true"></span>{/if}
          {#if name}<span class="pane-name">{name}</span>{/if}
        </span>
      {/if}
```

- [ ] **Step 4: Add the pane-label styles**

Find the `.worktree-tag` style block (around line 1093). The current `.worktree-tag` rule has `margin-left: auto;`. We need close-button to take that role and worktree-tag plus pane-label to sit before it.

Remove `margin-left: auto;` from `.worktree-tag` and **keep** `.control-btn.close-btn { margin-left: auto; }` as-is (it already exists at line 1085).

Then add new styles (alongside the existing `.worktree-tag` block):

```css
  .pane-label {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    font-family: 'IBM Plex Mono', 'JetBrains Mono', monospace;
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
    flex-shrink: 0;
  }

  .pane-name {
    white-space: nowrap;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
  }
```

- [ ] **Step 5: Typecheck and lint**

```
npm run check
npm run lint
```

Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/terminal/Terminal.svelte
git commit -m "feat(terminal): render pane name/color label in header"
```

---

## Task 15: Browser test for the pane-label rendering

**Files:**
- Create: `src/lib/components/terminal/Terminal.browser.test.ts`

(If a `Terminal.browser.test.ts` already exists, **add** these tests there instead of creating a new file. Check first with `ls src/lib/components/terminal/`. If only `Terminal.svelte` is present, create the new file.)

- [ ] **Step 1: Write the tests**

```ts
import { render } from '@testing-library/svelte';
import { describe, expect, test } from 'vitest';
import Terminal from './Terminal.svelte';

describe('Terminal pane-label header', () => {
  test('renders pane-name when name is set', () => {
    const { container } = render(Terminal, {
      props: { paneId: 'p1', name: 'build' },
    });
    expect(container.querySelector('.pane-name')?.textContent).toBe('build');
    expect(container.querySelector('.pane-dot')).toBeNull();
  });

  test('renders pane-dot with color variable when color is set', () => {
    const { container } = render(Terminal, {
      props: { paneId: 'p1', color: 'jade' },
    });
    const label = container.querySelector('.pane-label') as HTMLElement;
    expect(label).not.toBeNull();
    expect(label.style.getPropertyValue('--pane-color')).toBe('var(--pane-color-jade)');
    expect(container.querySelector('.pane-dot')).not.toBeNull();
    expect(container.querySelector('.pane-name')).toBeNull();
  });

  test('renders both dot and name when both are set', () => {
    const { container } = render(Terminal, {
      props: { paneId: 'p1', name: 'agent', color: 'iris' },
    });
    expect(container.querySelector('.pane-dot')).not.toBeNull();
    expect(container.querySelector('.pane-name')?.textContent).toBe('agent');
  });

  test('omits pane-label entirely when neither name nor color is set', () => {
    const { container } = render(Terminal, { props: { paneId: 'p1' } });
    expect(container.querySelector('.pane-label')).toBeNull();
  });
});
```

- [ ] **Step 2: Run the browser tests**

```
npm run test:browser -- Terminal.browser.test.ts
```

Expected: 4 passed. If `Terminal.svelte` tries to invoke Tauri APIs on mount and fails the test, the existing browser-test setup probably already stubs them — check what other `*.browser.test.ts` files in `src/lib/components/ui/` do and replicate any required mocks (or skip those tests with a clear note if the Tauri integration is too invasive). Adjust tests until they pass without modifying the component.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/terminal/Terminal.browser.test.ts
git commit -m "test(terminal): cover pane-label rendering variants"
```

---

## Task 16: Manual integration smoke test

This is not a TDD task — it verifies end-to-end behavior with the real kiri app, before opening the PR.

- [ ] **Step 1: Clean the Tauri build cache (we're in a worktree)**

```
cd src-tauri && cargo clean && cd ..
```

- [ ] **Step 2: Start the app**

```
npm run tauri dev
```

Wait for the kiri window to come up.

- [ ] **Step 3: Open a kiri pane and run the new commands via @hypothesi/tauri-mcp-server**

Use the MCP tooling (`mcp__hypothesi_tauri-mcp-server__webview_*`, `read_logs`, etc.) — or, if those aren't available, just observe in the GUI:

```
kiri term split --name build --color coral
kiri term split --name watch --color amber --dir v
kiri term ls
```

Verify:
- Each new pane's header shows the colored dot + name immediately after split.
- `kiri term ls` JSON includes `name` and `color` fields for the labeled panes and omits them for the original unlabeled pane.

If anything is broken, **stop** and fix it before opening the PR. Add a regression test capturing the bug.

- [ ] **Step 4: Stop the dev server**

Ctrl-C in the dev terminal (or the relevant kiri pane).

- [ ] **Step 5: Run the full local CI gate before pushing**

```
/run-github-actions-locally
```

Or manually:

```
npm run lint
npm run check
npm run test
npm run test:browser
npm run test:rust
```

Everything must pass. If any of these fail, fix and re-run before continuing.

---

## Task 17: Update the kiri-cli skill documentation

**Files:**
- Modify: `~/.claude/skills/kiri-cli/SKILL.md` (outside the repo)

- [ ] **Step 1: Update the `kiri term split` section**

Replace the current code block in Section 5 (`kiri term split [--pane X] [--dir h|v]`) with:

```bash
kiri term split [--pane X] [--dir h|v] [--name STR] [--color COLOR]

kiri term split
kiri term split --dir v
kiri term split --pane pane-1 --dir h
kiri term split --name build --color coral
kiri term split --name agent --color iris
```

After the existing `Response shape:` block, add a "Label flags" subsection:

```markdown
**Label flags** (both optional, both apply only at split time):

- `--name STR` — 1–32 chars, no control characters. Shown as text in the pane header.
- `--color COLOR` — one of `sky | iris | jade | amber | coral | rose`. Shown as a colored dot in the pane header.

Either, both, or neither may be supplied. A pane created without these flags has no header label.
```

- [ ] **Step 2: Update the `ls` response example**

In Section 5 under `kiri term ls`, extend the example JSON to include the new optional fields:

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
      "color": "coral"
    }
  ]
}
```

Add a sentence underneath: "`name` and `color` are omitted entirely when the pane has no label."

- [ ] **Step 3: Update the larger `ls` example in Section 7 as well, adding a labeled pane**

Inside the existing `### `ls` response` block, mark one of the two panes with `"name": "agent", "color": "iris"` to demonstrate the field.

- [ ] **Step 4: This file is outside the repo; no commit needed in the kiri repo.**

(The kiri-cli skill ships with Claude, not with the kiri source tree. If the user has a separate sync workflow for it, follow that; otherwise leaving the local edit in place is sufficient.)

---

## Task 18: Run the full local CI gate once more, then push

- [ ] **Step 1: From the worktree root, run**

```
/run-github-actions-locally
```

- [ ] **Step 2: If anything fails, fix in a new commit (do not amend)**

- [ ] **Step 3: Hand off to /pr-complete**

`/pr-complete` will push the branch, open a PR against `main`, watch CI, auto-fix CI failures, run code review, and converge. The current task is done once `/pr-complete` reports a green PR with no outstanding review comments.

---

## Self-Review Checklist (done during plan writing)

**Spec coverage:** Every spec section maps to at least one task —

- Proto / `PaneColor` → Task 1
- `Request::Split` proto change → Task 2
- `PaneInfo` proto change → Task 3
- CLI `--name --color` + validation → Task 4
- CLI `main.rs` wiring → Task 5
- `PaneEntry` change → Task 6
- `ls` handler → Task 7
- `split` handler event payload → Task 8
- Frontend types + `splitPane` → Task 9
- `cliBridge` → Task 10
- `App.svelte` pane map push → Task 11
- CSS variables → Task 12
- `TerminalContainer` props → Task 13
- `Terminal.svelte` props + UI + styles → Task 14
- Browser test → Task 15
- Manual E2E smoke → Task 16
- Skill doc update → Task 17
- Final CI gate + push → Task 18

**Placeholder scan:** No "TODO", "TBD", or vague-handwave steps. Every code step ships exact code.

**Type consistency:** `PaneColor` (Rust) and `PaneColor` (TS) share the same string set (`sky/iris/jade/amber/coral/rose`). `splitPane` opts type is `{ name?: string; color?: PaneColor }` everywhere. CLI's `PaneColorArg → PaneColor` conversion is a one-shot `From` impl.

---

## Out of scope (deferred)

- `kiri term label` for renaming existing panes.
- Persistence across kiri restarts.
- E2E test via webdriver/Tauri integration test harness — manual smoke only.
- Naming the *initial* pane that the app spawns on window open (no CLI surface yet).
