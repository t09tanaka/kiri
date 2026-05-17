# Terminal Worktree Tag

**Date:** 2026-04-27
**Status:** Approved (design)

## Summary

Add a small tag to the right side of each terminal panel header that indicates whether the terminal's current working directory is inside a linked git worktree. When inside a linked worktree, show the worktree's directory name. When in the main worktree, in a non-worktree git repo, or outside any git repo, show nothing.

The tag updates dynamically as the user `cd`s in and out of worktrees, but only re-evaluates git state when the shell's actual cwd changes (efficient polling).

## Motivation

The previous git worktree integration was removed in commit 91c8898. Users still create worktrees manually via the terminal and need a passive, lightweight signal of "where am I right now" without re-running `git worktree list` mentally. A tag in the terminal header provides this without taking screen real estate when not relevant (main worktree).

## Scope

**In scope:**
- Detect whether a path is inside a linked git worktree
- Display worktree name in terminal header (right side, before Close button)
- Update on cwd change (detected via existing process polling)
- Tooltip with full worktree root path
- Mist-theme styling

**Out of scope:**
- Re-introducing any worktree management UI (creating/deleting/listing worktrees)
- Showing branch name (worktree directory name is the chosen identifier)
- Indicator for non-worktree state (main worktree, non-git, etc.)
- Click action on the tag (purely informational)

## Architecture

### Backend (Rust)

New command in `src-tauri/src/commands/git.rs`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct WorktreeInfo {
    /// true if path is inside a linked worktree, false if main worktree
    pub is_linked_worktree: bool,
    /// basename of the worktree root directory
    pub name: String,
    /// absolute path to the worktree root
    pub root: String,
}

pub fn get_worktree_info(path: String) -> Option<WorktreeInfo>;
```

Split into pure logic + Tauri wrapper to keep the detection function unit-testable per `.claude/rules/testing.md`:

```rust
// Pure logic — testable
fn detect_worktree_info(path: &Path) -> Option<WorktreeInfo>;

// Tauri wrapper — thin adapter
#[tauri::command]
pub fn get_worktree_info(path: String) -> Option<WorktreeInfo> {
    detect_worktree_info(Path::new(&path))
}
```

Both live inline in `src-tauri/src/commands/git.rs` to match the existing pattern (other git commands like `get_git_status` are also inline in this file — `git_history.rs` uses a separate `git_history_commands.rs` but `git.rs` does not, and we follow the local convention rather than introducing a new split).

Detection logic (`detect_worktree_info`):
1. Use `git2::Repository::discover(path)` to find the enclosing repository.
2. Read `repo.workdir()` to get the worktree's working directory.
3. Determine main vs linked: inspect `<workdir>/.git`.
   - If it is a **directory**, this is the main worktree.
   - If it is a **file** (gitlink format: `gitdir: /path/to/.git/worktrees/<name>`), this is a linked worktree.
4. `name` = basename of the workdir path.
5. Return `None` when:
   - Path is not inside any git repo (`discover` fails)
   - Path does not exist
   - `repo.workdir()` is `None` (bare repo)
   - Workdir basename cannot be extracted (filesystem root edge case)

### Frontend

**`src/lib/services/gitService.ts`** — add typed wrapper:

```typescript
export interface WorktreeInfo {
  is_linked_worktree: boolean;
  name: string;
  root: string;
}

// Inside gitService:
getWorktreeInfo: (path: string): Promise<WorktreeInfo | null> =>
  invoke('get_worktree_info', { path }),
```

**`src/lib/components/terminal/Terminal.svelte`** — add reactive state and polling integration:

```typescript
let worktreeInfo = $state<WorktreeInfo | null>(null);
let lastCwd: string | null = null;

async function updateProcessInfo() {
  if (terminalId === null) return;
  try {
    const info = await terminalService.getProcessInfo(terminalId);
    processName = info.name;

    const cwd = await terminalService.getCwd(terminalId);
    if (cwd !== lastCwd) {
      lastCwd = cwd;
      worktreeInfo = cwd ? await gitService.getWorktreeInfo(cwd) : null;
    }
  } catch {
    // Terminal may have been closed
  }
}
```

The existing 2-second polling loop (`processPollInterval`) already calls `updateProcessInfo`. Cwd lookup and worktree detection ride on top of it. Worktree detection only runs when cwd actually changes — typically once at startup, then only after `cd`.

### UI

In `Terminal.svelte`'s `.terminal-controls` block, render the tag immediately before the Close button. When `worktreeInfo?.is_linked_worktree === true`:

```svelte
{#if worktreeInfo?.is_linked_worktree}
  <span class="worktree-tag" title={worktreeInfo.root}>
    <svg width="11" height="11" viewBox="0 0 24 24" fill="none"
         stroke="currentColor" stroke-width="2">
      <!-- git branch icon -->
      <circle cx="6" cy="6" r="2" />
      <circle cx="6" cy="18" r="2" />
      <circle cx="18" cy="9" r="2" />
      <path d="M6 8v8" />
      <path d="M18 11c0 4-6 4-6 7" />
    </svg>
    {worktreeInfo.name}
  </span>
{/if}
```

Layout: tag uses `margin-left: auto` to push to the right. The Close button (currently `margin-left: auto`) is updated to use a small left gap so it sits adjacent to the tag.

Style (Mist theme):
- Background: `rgba(125, 211, 252, 0.08)`
- Border: `1px solid rgba(125, 211, 252, 0.2)`
- Text: `var(--accent-color)`
- Border radius: `var(--radius-sm)`
- Padding: `2px 8px`
- Font: `'IBM Plex Mono'` monospace, 11–12px
- Hover: background `rgba(125, 211, 252, 0.14)`, transition `var(--transition-fast)`
- Display: inline-flex, align-items center, gap 4px (between icon and text)

Tooltip (native `title` attribute) shows the full worktree root path.

## Data Flow

```
[shell cd $WORKTREE_PATH]
        │
        ▼ (next 2-second poll tick)
updateProcessInfo()
        │
        ├─ terminalService.getCwd(id)  ──► new cwd
        │   │
        │   ▼ cwd changed?
        ├─ gitService.getWorktreeInfo(cwd)
        │   │
        │   ▼ Tauri invoke
        └─ Rust: git_commands::get_worktree_info
              │
              ▼ git2::Repository::discover + .git inspection
            Option<WorktreeInfo>
              │
              ▼
        worktreeInfo = $state — Svelte re-renders header
```

## Edge Cases

| Case | Behavior |
|------|----------|
| Path not in any git repo | `None` → tag hidden |
| Path is a bare repo | `None` (no workdir) → tag hidden |
| Path is in main worktree | `is_linked_worktree: false` → tag hidden |
| Path is in linked worktree | `is_linked_worktree: true` → tag shown with name |
| Path is in subdirectory of linked worktree | Same as above (workdir is the worktree root) |
| Symlinked path | Resolve via `git2::Repository::discover` (it canonicalizes) |
| Worktree directory name contains spaces or unicode | Display as-is (font supports both) |
| `getCwd` returns `null` (process info unavailable) | Treat as "no cwd known" → tag hidden, do not call `getWorktreeInfo` |
| Rust command fails (deleted dir, permission error) | Return `None` → tag hidden, error logged but not surfaced |
| Terminal split: each pane independently polls | Each pane shows its own tag based on its own cwd ✓ |
| Reattached terminal (registry path) | Polling restarts on remount → tag refreshes within 2s |

## Testing

**Backend (Rust unit tests in `git.rs`):**
- `test_get_worktree_info_main_repo` — temp git repo, expect `is_linked_worktree: false`
- `test_get_worktree_info_linked_worktree` — temp main repo + `git worktree add`, expect `is_linked_worktree: true` and correct `name`
- `test_get_worktree_info_subdirectory_of_worktree` — same as above but query a subdir, still detected
- `test_get_worktree_info_outside_repo` — temp non-git dir, expect `None`
- `test_get_worktree_info_nonexistent_path` — expect `None`

**Frontend:**
- `gitService.test.ts` — mock `invoke('get_worktree_info', ...)`, verify wrapper passes path correctly and unwraps `null`
- Browser test for `Terminal.svelte` is heavy (xterm-dependent) — skip dedicated test for the UI itself; rely on the unit test for service + manual verification per project's `.claude/rules/verification.md`

**Manual verification (required by project rules):**
- `npm run tauri dev` (after `cargo clean` if in a worktree)
- Open terminal in main worktree → no tag
- `cd` into a linked worktree → tag appears within 2s with the worktree name
- Hover the tag → tooltip shows full path
- `cd` back to main worktree → tag disappears
- Split pane → each pane shows its own tag based on its own cwd

## Files Changed

**New:**
- (none)

**Modified:**
- `src-tauri/src/commands/git.rs` — add `WorktreeInfo` struct, `detect_worktree_info` (pure), `#[tauri::command] get_worktree_info` (wrapper), unit tests
- `src-tauri/src/lib.rs` — import + register `get_worktree_info` in `invoke_handler`
- `src/lib/services/gitService.ts` — add `WorktreeInfo` interface and `getWorktreeInfo` wrapper
- `src/lib/services/gitService.test.ts` (if exists, else add) — wrapper test
- `src/lib/components/terminal/Terminal.svelte` — state, polling integration, tag markup, styles
