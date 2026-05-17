# Terminal Worktree Tag Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a small tag to the right side of each terminal panel header that displays the worktree directory name when the shell's cwd is inside a linked git worktree, and updates dynamically as the user `cd`s.

**Architecture:** A pure Rust detection function in `src-tauri/src/commands/git.rs` returns `Option<WorktreeInfo>` for any path; a `#[tauri::command]` wrapper exposes it. The frontend `Terminal.svelte` rides on the existing 2-second `updateProcessInfo` poll: it reads the shell's actual cwd via `terminalService.getCwd`, and only re-runs worktree detection when the cwd changes. The tag renders only when `is_linked_worktree === true`.

**Tech Stack:** Rust (`git2`), Tauri 2.x, Svelte 5 (`$state`, `$props`), TypeScript, Vitest.

**Spec:** `docs/superpowers/specs/2026-04-27-terminal-worktree-tag-design.md`

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src-tauri/src/commands/git.rs` (modified) | `WorktreeInfo` struct, `detect_worktree_info` (pure), `get_worktree_info` (Tauri command), unit tests |
| `src-tauri/src/lib.rs` (modified) | Register `get_worktree_info` in `invoke_handler` |
| `src/lib/services/gitService.ts` (modified) | Add `WorktreeInfo` interface and `getWorktreeInfo` wrapper |
| `src/lib/services/gitService.test.ts` (new) | Unit test for `getWorktreeInfo` wrapper (mocks `invoke`) |
| `src/lib/components/terminal/Terminal.svelte` (modified) | State (`worktreeInfo`, `lastCwd`), polling integration, tag markup, styles |

---

## Task 1: Backend — `WorktreeInfo` struct + pure detection function (TDD)

**Files:**
- Modify: `src-tauri/src/commands/git.rs` (add struct, function, tests; existing test module at line 313)

- [ ] **Step 1: Add `WorktreeInfo` struct above the existing `find_repo_root` function (around line 41)**

In `src-tauri/src/commands/git.rs`, immediately after the existing `GitRepoInfo` struct (around line 41), add:

```rust
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WorktreeInfo {
    /// True when path is inside a linked worktree, false when in the main worktree
    pub is_linked_worktree: bool,
    /// Basename of the worktree's working directory (e.g. "feat-foo")
    pub name: String,
    /// Absolute path to the worktree's working directory
    pub root: String,
}
```

- [ ] **Step 2: Add failing tests at the bottom of the existing `mod tests` block in `git.rs`**

Add these inside `mod tests { ... }` (after the last existing test, before the closing `}` of the test module). They reference `detect_worktree_info`, which does not yet exist — they MUST fail to compile or fail at run time:

```rust
    fn run_git(dir: &std::path::Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            // Avoid inheriting GIT_DIR / GIT_WORK_TREE from a parent worktree
            // (e.g. when the test suite itself runs inside a kiri worktree).
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .status()
            .expect("git command failed to start");
        assert!(status.success(), "git {:?} failed", args);
    }

    /// Initialize a git repo with one commit so that HEAD exists.
    /// Required because `git worktree add` needs a valid HEAD.
    fn init_repo_with_commit(dir: &std::path::Path) {
        run_git(dir, &["init", "-q", "-b", "main"]);
        run_git(dir, &["config", "user.email", "test@example.com"]);
        run_git(dir, &["config", "user.name", "Test"]);
        // Disable commit signing for hermetic tests
        run_git(dir, &["config", "commit.gpgsign", "false"]);
        fs::write(dir.join("README.md"), "init\n").unwrap();
        run_git(dir, &["add", "README.md"]);
        run_git(dir, &["commit", "-q", "-m", "init"]);
    }

    #[test]
    fn test_detect_worktree_info_outside_repo() {
        let dir = tempdir().unwrap();
        let result = detect_worktree_info(dir.path());
        assert!(result.is_none(), "non-git dir must return None");
    }

    #[test]
    fn test_detect_worktree_info_nonexistent_path() {
        let result = detect_worktree_info(std::path::Path::new("/nonexistent/path/xyz"));
        assert!(result.is_none(), "nonexistent path must return None");
    }

    #[test]
    fn test_detect_worktree_info_main_repo() {
        let dir = tempdir().unwrap();
        init_repo_with_commit(dir.path());

        let result = detect_worktree_info(dir.path()).expect("must detect repo");
        assert!(!result.is_linked_worktree, "main repo must not be linked");
        assert_eq!(
            result.name,
            dir.path().file_name().unwrap().to_string_lossy()
        );
    }

    #[test]
    fn test_detect_worktree_info_linked_worktree() {
        let main_dir = tempdir().unwrap();
        init_repo_with_commit(main_dir.path());

        let wt_parent = tempdir().unwrap();
        let wt_path = wt_parent.path().join("feat-foo");
        run_git(
            main_dir.path(),
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat-foo",
                wt_path.to_str().unwrap(),
            ],
        );

        let result = detect_worktree_info(&wt_path).expect("must detect linked worktree");
        assert!(result.is_linked_worktree, "must be flagged linked");
        assert_eq!(result.name, "feat-foo");
        assert!(result.root.contains("feat-foo"));
    }

    #[test]
    fn test_detect_worktree_info_subdirectory_of_linked_worktree() {
        let main_dir = tempdir().unwrap();
        init_repo_with_commit(main_dir.path());

        let wt_parent = tempdir().unwrap();
        let wt_path = wt_parent.path().join("feat-bar");
        run_git(
            main_dir.path(),
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat-bar",
                wt_path.to_str().unwrap(),
            ],
        );

        // Create a subdirectory inside the worktree and query it
        let sub = wt_path.join("src").join("nested");
        fs::create_dir_all(&sub).unwrap();

        let result = detect_worktree_info(&sub).expect("must detect via subdirectory");
        assert!(result.is_linked_worktree);
        assert_eq!(result.name, "feat-bar");
    }

    #[test]
    fn test_detect_worktree_info_subdirectory_of_main_repo() {
        let dir = tempdir().unwrap();
        init_repo_with_commit(dir.path());
        let sub = dir.path().join("src").join("nested");
        fs::create_dir_all(&sub).unwrap();

        let result = detect_worktree_info(&sub).expect("must detect via subdirectory");
        assert!(!result.is_linked_worktree);
        assert_eq!(
            result.name,
            dir.path().file_name().unwrap().to_string_lossy()
        );
    }
```

- [ ] **Step 3: Run the tests to verify they fail (compile error: `detect_worktree_info` undefined)**

Run from repo root:

```bash
cd src-tauri && cargo test --lib detect_worktree_info 2>&1 | tail -30
```

Expected: compilation fails with `cannot find function 'detect_worktree_info' in this scope`.

- [ ] **Step 4: Implement `detect_worktree_info` in `git.rs`**

Add this pure function in `git.rs` immediately after the existing `find_repo_root` function (around line 55, before `calculate_diff_stats`):

```rust
/// Detect git worktree information for a given path.
///
/// Returns `None` if the path is not inside any git working tree
/// (e.g. non-git directory, nonexistent path, or bare repo).
///
/// Returns `Some(WorktreeInfo)` with `is_linked_worktree = true` when
/// the path lies inside a linked worktree (the worktree's `.git` is a
/// gitlink file pointing to `<common-dir>/worktrees/<name>`), or
/// `is_linked_worktree = false` when inside the main worktree.
fn detect_worktree_info(path: &Path) -> Option<WorktreeInfo> {
    // Repository::discover walks up from `path` looking for a .git, and
    // canonicalizes symlinks. Returns Err for non-git or nonexistent paths.
    let repo = Repository::discover(path).ok()?;

    // Bare repos have no working directory — nothing to display.
    let workdir = repo.workdir()?;

    let name = workdir.file_name()?.to_string_lossy().to_string();
    let root = workdir.to_string_lossy().to_string();

    // Linked worktrees use a gitlink: `<workdir>/.git` is a regular file
    // containing `gitdir: <common-dir>/worktrees/<name>`. The main worktree
    // has `<workdir>/.git` as a directory.
    let dot_git = workdir.join(".git");
    let is_linked_worktree = dot_git.is_file();

    Some(WorktreeInfo {
        is_linked_worktree,
        name,
        root,
    })
}
```

- [ ] **Step 5: Run the tests to verify they pass**

```bash
cd src-tauri && cargo test --lib detect_worktree_info 2>&1 | tail -30
```

Expected: all 6 new tests PASS.

- [ ] **Step 6: Run the full git.rs test suite to confirm no regression**

```bash
cd src-tauri && cargo test --lib commands::git:: 2>&1 | tail -20
```

Expected: all existing tests still pass.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/commands/git.rs
git commit -m "feat(git): add detect_worktree_info for terminal worktree tag"
```

---

## Task 2: Backend — `#[tauri::command]` wrapper + register

**Files:**
- Modify: `src-tauri/src/commands/git.rs` (add wrapper after `detect_worktree_info`)
- Modify: `src-tauri/src/lib.rs` (add to `use commands::{ ... }` and to `invoke_handler!` list)

- [ ] **Step 1: Add the Tauri command wrapper in `git.rs`**

Add this immediately after `detect_worktree_info` in `git.rs`:

```rust
#[tauri::command]
pub fn get_worktree_info(path: String) -> Option<WorktreeInfo> {
    detect_worktree_info(Path::new(&path))
}
```

- [ ] **Step 2: Add a wrapper smoke test inside the existing `mod tests` block**

Add one test that exercises the public command on a non-git tempdir (no git2 dependency on the assertion path):

```rust
    #[test]
    fn test_get_worktree_info_command_returns_none_for_non_git() {
        let dir = tempdir().unwrap();
        let result = get_worktree_info(dir.path().to_string_lossy().to_string());
        assert!(result.is_none());
    }
```

- [ ] **Step 3: Register the command in `src-tauri/src/lib.rs`**

In the `use commands::{ ... }` block (lines 3–19 of `lib.rs`), add `get_worktree_info` alphabetically into the existing list. The current line 9 reads:

```rust
    get_branch_ahead_count, get_commit_diff, get_commit_log, get_git_diff, get_git_file_status,
```

Change line 10 from:

```rust
    get_git_status, get_home_directory, get_memory_metrics, get_performance_report,
```

to:

```rust
    get_git_status, get_home_directory, get_memory_metrics, get_performance_report, get_worktree_info,
```

Then in the `invoke_handler` block (lines 49–108), add `get_worktree_info,` immediately after the existing `get_all_git_diffs,` line (line 66):

```rust
            get_all_git_diffs,
            get_worktree_info,
            search_files,
```

- [ ] **Step 4: Build to verify wiring**

```bash
cd src-tauri && cargo build 2>&1 | tail -10
```

Expected: build succeeds with no warnings about unused `get_worktree_info`.

- [ ] **Step 5: Run the wrapper test**

```bash
cd src-tauri && cargo test --lib test_get_worktree_info_command_returns_none_for_non_git 2>&1 | tail -10
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/commands/git.rs src-tauri/src/lib.rs
git commit -m "feat(git): expose get_worktree_info as Tauri command"
```

---

## Task 3: Frontend — `gitService.getWorktreeInfo` wrapper + test

**Files:**
- Modify: `src/lib/services/gitService.ts` (add interface + method)
- Create: `src/lib/services/gitService.test.ts`

- [ ] **Step 1: Write the failing test at `src/lib/services/gitService.test.ts`**

```typescript
import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';
import { gitService } from './gitService';

describe('gitService.getWorktreeInfo', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('invokes get_worktree_info with the given path', async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    await gitService.getWorktreeInfo('/some/path');

    expect(invoke).toHaveBeenCalledWith('get_worktree_info', { path: '/some/path' });
    expect(invoke).toHaveBeenCalledTimes(1);
  });

  it('returns null when path is not in a git repo', async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    const result = await gitService.getWorktreeInfo('/nope');

    expect(result).toBeNull();
  });

  it('returns WorktreeInfo when path is inside a linked worktree', async () => {
    vi.mocked(invoke).mockResolvedValue({
      is_linked_worktree: true,
      name: 'feat-foo',
      root: '/tmp/wt/feat-foo',
    });

    const result = await gitService.getWorktreeInfo('/tmp/wt/feat-foo/src');

    expect(result).toEqual({
      is_linked_worktree: true,
      name: 'feat-foo',
      root: '/tmp/wt/feat-foo',
    });
  });
});
```

- [ ] **Step 2: Run the test to verify it fails (no `getWorktreeInfo` on `gitService`)**

```bash
npm run test -- src/lib/services/gitService.test.ts 2>&1 | tail -20
```

Expected: tests fail with `gitService.getWorktreeInfo is not a function`.

- [ ] **Step 3: Add `WorktreeInfo` interface and `getWorktreeInfo` to `gitService.ts`**

In `src/lib/services/gitService.ts`, add the interface near the top with the other exported interfaces (after `BehindAheadCount`, before `PullResult` — around line 47):

```typescript
export interface WorktreeInfo {
  is_linked_worktree: boolean;
  name: string;
  root: string;
}
```

Then add this method to the `gitService` object, immediately after `pullCommits` (the last current method, around line 120):

```typescript
  /**
   * Get worktree info for a path.
   * Returns null if the path is not inside a git working tree.
   */
  getWorktreeInfo: (path: string): Promise<WorktreeInfo | null> =>
    invoke('get_worktree_info', { path }),
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
npm run test -- src/lib/services/gitService.test.ts 2>&1 | tail -10
```

Expected: all 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/lib/services/gitService.ts src/lib/services/gitService.test.ts
git commit -m "feat(gitService): add getWorktreeInfo wrapper"
```

---

## Task 4: Frontend — Wire cwd polling and worktree state in `Terminal.svelte`

**Files:**
- Modify: `src/lib/components/terminal/Terminal.svelte`

- [ ] **Step 1: Add the `gitService` and `WorktreeInfo` imports**

In the `<script lang="ts">` block of `src/lib/components/terminal/Terminal.svelte`, add this import alongside the existing imports (after line 15, near the other service imports):

```typescript
  import { gitService, type WorktreeInfo } from '@/lib/services/gitService';
```

- [ ] **Step 2: Add reactive state for worktree info and the cwd cache**

Inside the `<script>` block, add these declarations near the other `$state` declarations (around line 70, after `let numberRowEnabled = $state(false);`):

```typescript
  let worktreeInfo = $state<WorktreeInfo | null>(null);
  // Plain (non-reactive) cache: only used to decide whether to refetch.
  // Updating this should NOT trigger re-renders.
  let lastCwd: string | null = null;
```

- [ ] **Step 3: Extend `updateProcessInfo` to refresh worktree info on cwd change**

Replace the existing `updateProcessInfo` function (around line 760) with:

```typescript
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

- [ ] **Step 4: Type-check the file**

```bash
npm run check 2>&1 | tail -15
```

Expected: no type errors related to `Terminal.svelte`, `gitService`, or `WorktreeInfo`.

- [ ] **Step 5: Run the existing test suites to confirm nothing broke**

```bash
npm run test 2>&1 | tail -10
```

Expected: all tests pass (no `Terminal.svelte` tests exist; this just guards against regressions in services).

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/terminal/Terminal.svelte
git commit -m "feat(terminal): poll worktree info on cwd change"
```

---

## Task 5: Frontend — Render the worktree tag in the terminal header

**Files:**
- Modify: `src/lib/components/terminal/Terminal.svelte` (markup + styles)

- [ ] **Step 1: Insert the tag markup before the Close button in `.terminal-controls`**

In the template section of `Terminal.svelte` (around lines 971–1029), insert the tag block immediately **before** the existing `{#if onClose}` block. The relevant region currently looks like:

```svelte
      </button>
      {#if onClose}
        <button
          class="control-btn close-btn"
```

Change it to:

```svelte
      </button>
      {#if worktreeInfo?.is_linked_worktree}
        <span
          class="worktree-tag"
          title={worktreeInfo.root}
          aria-label={`Worktree: ${worktreeInfo.name}`}
        >
          <svg
            width="11"
            height="11"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            aria-hidden="true"
          >
            <circle cx="6" cy="6" r="2" />
            <circle cx="6" cy="18" r="2" />
            <circle cx="18" cy="9" r="2" />
            <path d="M6 8v8" />
            <path d="M18 11c0 4-6 4-6 7" />
          </svg>
          <span class="worktree-tag-name">{worktreeInfo.name}</span>
        </span>
      {/if}
      {#if onClose}
        <button
          class="control-btn close-btn"
```

- [ ] **Step 2: Add worktree-tag styles and override close-button margin when tag is present**

In the `<style>` block, leave the existing `.control-btn.close-btn { margin-left: auto; }` rule (around line 1122) untouched — it still applies when the tag is hidden, keeping the current "close on the far right" behavior.

Add these new rules immediately after the existing `.control-btn.close-btn:hover` rule (around line 1129):

```css
  .worktree-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-left: auto;
    padding: 2px 8px;
    background: rgba(125, 211, 252, 0.08);
    border: 1px solid rgba(125, 211, 252, 0.2);
    border-radius: var(--radius-sm);
    color: var(--accent-color);
    font-family: 'IBM Plex Mono', 'SF Mono', monospace;
    font-size: 11px;
    line-height: 1.4;
    white-space: nowrap;
    max-width: 240px;
    overflow: hidden;
    transition: background var(--transition-fast);
  }

  .worktree-tag:hover {
    background: rgba(125, 211, 252, 0.14);
  }

  .worktree-tag-name {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  /* When the worktree tag is rendered (always immediately before close-btn),
     the tag claims the auto margin; close-btn just needs a small gap. */
  .worktree-tag + .control-btn.close-btn {
    margin-left: 4px;
  }
```

Behavior:
- Tag absent → existing `.control-btn.close-btn { margin-left: auto; }` pushes close to the right (current behavior preserved).
- Tag present → `.worktree-tag { margin-left: auto; }` pushes the tag to the right, and the adjacent-sibling override gives close-btn a small `4px` gap so they sit together at the right edge.

- [ ] **Step 3: Type-check and lint**

```bash
npm run check 2>&1 | tail -10
npm run lint 2>&1 | tail -10
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/lib/components/terminal/Terminal.svelte
git commit -m "feat(terminal): show worktree tag in panel header when in linked worktree"
```

---

## Task 6: Manual verification (per `.claude/rules/verification.md`)

**No code changes — verification only. Do not commit.**

- [ ] **Step 1: Confirm branch and clear Tauri cache if working in a worktree**

```bash
git branch --show-current
```

If the working directory is anything other than `~/Documents/GitHub/kiri`, run:

```bash
cd src-tauri && cargo clean && cd ..
```

- [ ] **Step 2: Start the app**

```bash
npm run tauri dev
```

Wait for the window to open.

- [ ] **Step 3: Verify "no tag in main worktree"**

In the kiri terminal panel that opens by default, confirm the controls bar shows only Split-V, Split-H, and Close. **No worktree tag should be visible** (you are in `~/Documents/GitHub/kiri` — the main worktree).

- [ ] **Step 4: Verify "tag appears after `cd` into a linked worktree"**

If a linked worktree of kiri exists (e.g. under `/tmp` or a parallel directory), `cd` into it from the kiri terminal:

```bash
cd /path/to/some/kiri-worktree
```

Within ~2 seconds, the worktree tag should appear at the right side of the controls bar showing the worktree's directory basename (e.g. `feat-foo`).

If no worktree exists, create one first from a separate terminal:

```bash
cd ~/Documents/GitHub/kiri
git worktree add /tmp/kiri-test-wt -b test-worktree-tag
```

Then `cd /tmp/kiri-test-wt` from the kiri terminal.

- [ ] **Step 5: Verify the tooltip**

Hover the tag. The native tooltip should display the full absolute path of the worktree root.

- [ ] **Step 6: Verify "tag disappears after `cd` back to main worktree"**

In the same terminal:

```bash
cd ~/Documents/GitHub/kiri
```

Within ~2 seconds, the tag should disappear.

- [ ] **Step 7: Verify split panes are independent**

Click the Split-V button to open a second terminal pane. In the new pane, `cd` into the linked worktree. Confirm:
- The new pane shows the tag.
- The original pane (still in the main worktree) does NOT show the tag.

Each pane reflects its own cwd.

- [ ] **Step 8: Cleanup test worktree (if you created one)**

```bash
git worktree remove /tmp/kiri-test-wt
git branch -d test-worktree-tag
```

- [ ] **Step 9: Stop the dev server**

`Ctrl+C` in the `npm run tauri dev` terminal.

---

## Done

All tasks complete. The terminal panel now shows a worktree tag on the right side of the controls bar when, and only when, the shell's current cwd is inside a linked git worktree.
