# PR-Worktree Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a PR button to the status bar that opens a panel for browsing GitHub PRs and launching isolated worktree environments per PR.

**Architecture:** Backend Rust commands execute `gh` CLI to fetch PR data. A frontend service wraps those commands, a store manages state, and a slide-in panel (following WorktreePanel patterns) provides the UI. The existing worktree creation flow handles environment isolation.

**Tech Stack:** Rust (Tauri commands, `std::process::Command` for `gh` CLI), Svelte 5, TypeScript, `gh` CLI

---

## File Structure

| File | Responsibility |
|------|----------------|
| `src-tauri/src/commands/github_pr.rs` | Pure logic: execute `gh` CLI, parse JSON output |
| `src-tauri/src/commands/github_pr_commands.rs` | Tauri command wrappers |
| `src/lib/services/prService.ts` | Frontend service wrapping Tauri commands |
| `src/lib/stores/prStore.ts` | PR list state, selected PR, loading/error |
| `src/lib/stores/prViewStore.ts` | Panel open/close state |
| `src/lib/components/pr/PrPanel.svelte` | PR list + detail panel UI |
| `src/lib/components/layout/StatusBar.svelte` | Modify: add PR button |
| `src/App.svelte` | Modify: render PrPanel, add keyboard shortcut |
| `src-tauri/src/commands/mod.rs` | Modify: register new modules |
| `src-tauri/src/lib.rs` | Modify: register new commands |

---

## Task 1: Rust Backend — `gh` CLI Execution and PR Data Parsing

**Files:**
- Create: `src-tauri/src/commands/github_pr.rs`
- Test: `src-tauri/src/commands/github_pr.rs` (inline `#[cfg(test)]`)

- [ ] **Step 1: Write failing test for `check_gh_cli`**

```rust
// src-tauri/src/commands/github_pr.rs
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct GhCliStatus {
    pub installed: bool,
    pub authenticated: bool,
}

pub fn check_gh_cli() -> GhCliStatus {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_gh_cli_returns_status() {
        let status = check_gh_cli();
        // Should not panic, returns a valid struct
        assert!(status.installed || !status.installed);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml check_gh_cli`
Expected: FAIL with "not yet implemented"

- [ ] **Step 3: Implement `check_gh_cli`**

```rust
use std::process::Command;

pub fn check_gh_cli() -> GhCliStatus {
    let installed = Command::new("gh")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !installed {
        return GhCliStatus {
            installed: false,
            authenticated: false,
        };
    }

    let authenticated = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    GhCliStatus {
        installed,
        authenticated,
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml check_gh_cli`
Expected: PASS

- [ ] **Step 5: Write failing test for `parse_pr_list_json`**

```rust
#[derive(Debug, Clone, Serialize)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub author_login: String,
    pub head_ref_name: String,
    pub state: String,
    pub updated_at: String,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
    pub body: String,
    pub review_decision: Option<String>,
    pub status_check_rollup: Vec<CheckStatus>,
    pub labels: Vec<PrLabel>,
    pub files: Vec<PrFile>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckStatus {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrLabel {
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PrFile {
    pub path: String,
    pub additions: u32,
    pub deletions: u32,
}

fn parse_pr_list_json(json_str: &str) -> Result<Vec<PullRequest>, String> {
    todo!()
}

#[cfg(test)]
mod tests {
    // ... existing tests ...

    #[test]
    fn test_parse_pr_list_json_valid() {
        let json = r#"[{
            "number": 42,
            "title": "feat: add auth",
            "author": {"login": "bot"},
            "headRefName": "feat/auth",
            "state": "OPEN",
            "updatedAt": "2026-03-29T00:00:00Z",
            "additions": 100,
            "deletions": 20,
            "changedFiles": 5,
            "body": "Adds authentication",
            "reviewDecision": "APPROVED",
            "statusCheckRollup": {"contexts": [
                {"name": "CI", "status": "COMPLETED", "conclusion": "SUCCESS"}
            ]},
            "labels": {"nodes": [{"name": "enhancement", "color": "a2eeef"}]},
            "files": {"nodes": [{"path": "src/auth.ts", "additions": 80, "deletions": 10}]}
        }]"#;
        let prs = parse_pr_list_json(json).unwrap();
        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 42);
        assert_eq!(prs[0].title, "feat: add auth");
        assert_eq!(prs[0].author_login, "bot");
        assert_eq!(prs[0].head_ref_name, "feat/auth");
        assert_eq!(prs[0].additions, 100);
        assert_eq!(prs[0].status_check_rollup.len(), 1);
        assert_eq!(prs[0].status_check_rollup[0].conclusion, Some("SUCCESS".to_string()));
        assert_eq!(prs[0].labels.len(), 1);
        assert_eq!(prs[0].files.len(), 1);
    }

    #[test]
    fn test_parse_pr_list_json_empty() {
        let prs = parse_pr_list_json("[]").unwrap();
        assert!(prs.is_empty());
    }

    #[test]
    fn test_parse_pr_list_json_invalid() {
        let result = parse_pr_list_json("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_pr_list_json_missing_optional_fields() {
        let json = r#"[{
            "number": 1,
            "title": "fix: typo",
            "author": {"login": "dev"},
            "headRefName": "fix/typo",
            "state": "OPEN",
            "updatedAt": "2026-03-29T00:00:00Z",
            "additions": 1,
            "deletions": 1,
            "changedFiles": 1,
            "body": "",
            "reviewDecision": null,
            "statusCheckRollup": {"contexts": []},
            "labels": {"nodes": []},
            "files": {"nodes": []}
        }]"#;
        let prs = parse_pr_list_json(json).unwrap();
        assert_eq!(prs[0].review_decision, None);
        assert!(prs[0].status_check_rollup.is_empty());
    }
}
```

- [ ] **Step 6: Run test to verify it fails**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parse_pr_list_json`
Expected: FAIL with "not yet implemented"

- [ ] **Step 7: Implement `parse_pr_list_json`**

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct GhPrRaw {
    number: u32,
    title: String,
    author: GhAuthor,
    #[serde(rename = "headRefName")]
    head_ref_name: String,
    state: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    additions: u32,
    deletions: u32,
    #[serde(rename = "changedFiles")]
    changed_files: u32,
    body: String,
    #[serde(rename = "reviewDecision")]
    review_decision: Option<String>,
    #[serde(rename = "statusCheckRollup")]
    status_check_rollup: GhStatusCheckRollup,
    labels: GhNodes<GhLabel>,
    files: GhNodes<GhFile>,
}

#[derive(Deserialize)]
struct GhAuthor {
    login: String,
}

#[derive(Deserialize)]
struct GhStatusCheckRollup {
    contexts: Vec<GhCheckContext>,
}

#[derive(Deserialize)]
struct GhCheckContext {
    name: String,
    status: String,
    conclusion: Option<String>,
}

#[derive(Deserialize)]
struct GhNodes<T> {
    nodes: Vec<T>,
}

#[derive(Deserialize)]
struct GhLabel {
    name: String,
    color: String,
}

#[derive(Deserialize)]
struct GhFile {
    path: String,
    additions: u32,
    deletions: u32,
}

fn parse_pr_list_json(json_str: &str) -> Result<Vec<PullRequest>, String> {
    let raw: Vec<GhPrRaw> =
        serde_json::from_str(json_str).map_err(|e| format!("Failed to parse PR JSON: {}", e))?;

    Ok(raw
        .into_iter()
        .map(|r| PullRequest {
            number: r.number,
            title: r.title,
            author_login: r.author.login,
            head_ref_name: r.head_ref_name,
            state: r.state,
            updated_at: r.updated_at,
            additions: r.additions,
            deletions: r.deletions,
            changed_files: r.changed_files,
            body: r.body,
            review_decision: r.review_decision,
            status_check_rollup: r.status_check_rollup.contexts
                .into_iter()
                .map(|c| CheckStatus {
                    name: c.name,
                    status: c.status,
                    conclusion: c.conclusion,
                })
                .collect(),
            labels: r.labels.nodes
                .into_iter()
                .map(|l| PrLabel {
                    name: l.name,
                    color: l.color,
                })
                .collect(),
            files: r.files.nodes
                .into_iter()
                .map(|f| PrFile {
                    path: f.path,
                    additions: f.additions,
                    deletions: f.deletions,
                })
                .collect(),
        })
        .collect())
}
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test --manifest-path src-tauri/Cargo.toml parse_pr_list_json`
Expected: All 4 tests PASS

- [ ] **Step 9: Implement `list_pull_requests` and `get_pull_request_detail`**

```rust
pub fn list_pull_requests(repo_path: String) -> Result<Vec<PullRequest>, String> {
    let output = Command::new("gh")
        .args([
            "pr", "list",
            "--json", "number,title,author,headRefName,state,updatedAt,additions,deletions,changedFiles,body,reviewDecision,statusCheckRollup,labels,files",
            "--limit", "50",
        ])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("Failed to run gh: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh pr list failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_pr_list_json(&stdout)
}

pub fn get_pull_request_detail(repo_path: String, number: u32) -> Result<PullRequest, String> {
    let output = Command::new("gh")
        .args([
            "pr", "view",
            &number.to_string(),
            "--json", "number,title,author,headRefName,state,updatedAt,additions,deletions,changedFiles,body,reviewDecision,statusCheckRollup,labels,files",
        ])
        .current_dir(&repo_path)
        .output()
        .map_err(|e| format!("Failed to run gh: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh pr view failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let prs = parse_pr_list_json(&format!("[{}]", stdout))?;
    prs.into_iter()
        .next()
        .ok_or_else(|| "No PR data returned".to_string())
}
```

- [ ] **Step 10: Commit**

```bash
git add -f src-tauri/src/commands/github_pr.rs
git commit -m "feat(pr): add Rust backend for gh CLI PR data fetching"
```

---

## Task 2: Rust Tauri Command Wrappers

**Files:**
- Create: `src-tauri/src/commands/github_pr_commands.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create command wrappers**

```rust
// src-tauri/src/commands/github_pr_commands.rs
use super::github_pr::{GhCliStatus, PullRequest};

#[tauri::command]
pub fn check_gh_cli() -> GhCliStatus {
    super::github_pr::check_gh_cli()
}

#[tauri::command]
pub fn list_pull_requests(repo_path: String) -> Result<Vec<PullRequest>, String> {
    super::github_pr::list_pull_requests(repo_path)
}

#[tauri::command]
pub fn get_pull_request_detail(repo_path: String, number: u32) -> Result<PullRequest, String> {
    super::github_pr::get_pull_request_detail(repo_path, number)
}
```

- [ ] **Step 2: Register modules in `mod.rs`**

Add to `src-tauri/src/commands/mod.rs`:

```rust
pub mod github_pr;
pub mod github_pr_commands;
// ... existing modules ...
pub use github_pr_commands::*;
```

- [ ] **Step 3: Register commands in `lib.rs`**

Add to the `use commands::{...}` block:
```rust
check_gh_cli, list_pull_requests, get_pull_request_detail,
```

Add to the `invoke_handler` block:
```rust
// GitHub PR
check_gh_cli,
list_pull_requests,
get_pull_request_detail,
```

- [ ] **Step 4: Verify compilation**

Run: `cargo build --manifest-path src-tauri/Cargo.toml`
Expected: Build succeeds with no errors

- [ ] **Step 5: Run all Rust tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add -f src-tauri/src/commands/github_pr_commands.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs
git commit -m "feat(pr): register GitHub PR Tauri commands"
```

---

## Task 3: Frontend Service — `prService.ts`

**Files:**
- Create: `src/lib/services/prService.ts`
- Create: `src/lib/services/prService.test.ts`

- [ ] **Step 1: Write failing test for prService**

```typescript
// src/lib/services/prService.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { prService } from './prService';
import type { PullRequest, GhCliStatus } from './prService';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

describe('prService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('checkGhCli', () => {
    it('should call invoke with check_gh_cli', async () => {
      const mockStatus: GhCliStatus = { installed: true, authenticated: true };
      vi.mocked(invoke).mockResolvedValue(mockStatus);

      const result = await prService.checkGhCli();

      expect(invoke).toHaveBeenCalledWith('check_gh_cli');
      expect(result).toEqual(mockStatus);
    });
  });

  describe('listPrs', () => {
    it('should call invoke with list_pull_requests and repo path', async () => {
      const mockPrs: PullRequest[] = [];
      vi.mocked(invoke).mockResolvedValue(mockPrs);

      const result = await prService.listPrs('/repo');

      expect(invoke).toHaveBeenCalledWith('list_pull_requests', { repoPath: '/repo' });
      expect(result).toEqual([]);
    });
  });

  describe('getPrDetail', () => {
    it('should call invoke with get_pull_request_detail and PR number', async () => {
      const mockPr: PullRequest = {
        number: 42,
        title: 'test',
        author_login: 'dev',
        head_ref_name: 'feat/test',
        state: 'OPEN',
        updated_at: '2026-03-29T00:00:00Z',
        additions: 10,
        deletions: 5,
        changed_files: 2,
        body: 'description',
        review_decision: null,
        status_check_rollup: [],
        labels: [],
        files: [],
      };
      vi.mocked(invoke).mockResolvedValue(mockPr);

      const result = await prService.getPrDetail('/repo', 42);

      expect(invoke).toHaveBeenCalledWith('get_pull_request_detail', {
        repoPath: '/repo',
        number: 42,
      });
      expect(result).toEqual(mockPr);
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- --run src/lib/services/prService.test.ts`
Expected: FAIL (module not found)

- [ ] **Step 3: Implement prService**

```typescript
// src/lib/services/prService.ts
import { invoke } from '@tauri-apps/api/core';

export interface GhCliStatus {
  installed: boolean;
  authenticated: boolean;
}

export interface CheckStatus {
  name: string;
  status: string;
  conclusion: string | null;
}

export interface PrLabel {
  name: string;
  color: string;
}

export interface PrFile {
  path: string;
  additions: number;
  deletions: number;
}

export interface PullRequest {
  number: number;
  title: string;
  author_login: string;
  head_ref_name: string;
  state: string;
  updated_at: string;
  additions: number;
  deletions: number;
  changed_files: number;
  body: string;
  review_decision: string | null;
  status_check_rollup: CheckStatus[];
  labels: PrLabel[];
  files: PrFile[];
}

export const prService = {
  checkGhCli: (): Promise<GhCliStatus> => invoke('check_gh_cli'),

  listPrs: (repoPath: string): Promise<PullRequest[]> =>
    invoke('list_pull_requests', { repoPath }),

  getPrDetail: (repoPath: string, number: number): Promise<PullRequest> =>
    invoke('get_pull_request_detail', { repoPath, number }),
};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test -- --run src/lib/services/prService.test.ts`
Expected: All 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/lib/services/prService.ts src/lib/services/prService.test.ts
git commit -m "feat(pr): add prService frontend service"
```

---

## Task 4: Frontend Stores — `prStore.ts` and `prViewStore.ts`

**Files:**
- Create: `src/lib/stores/prStore.ts`
- Create: `src/lib/stores/prStore.test.ts`
- Create: `src/lib/stores/prViewStore.ts`
- Create: `src/lib/stores/prViewStore.test.ts`

- [ ] **Step 1: Write failing test for prViewStore**

```typescript
// src/lib/stores/prViewStore.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import { get } from 'svelte/store';
import { prViewStore, isPrViewOpen } from './prViewStore';

describe('prViewStore', () => {
  beforeEach(() => {
    prViewStore.close();
  });

  it('should start closed', () => {
    const state = get(prViewStore);
    expect(state.isOpen).toBe(false);
    expect(state.projectPath).toBeNull();
  });

  it('should open with project path', () => {
    prViewStore.open('/repo');
    const state = get(prViewStore);
    expect(state.isOpen).toBe(true);
    expect(state.projectPath).toBe('/repo');
  });

  it('should close and reset state', () => {
    prViewStore.open('/repo');
    prViewStore.close();
    const state = get(prViewStore);
    expect(state.isOpen).toBe(false);
    expect(state.projectPath).toBeNull();
  });

  it('should derive isPrViewOpen', () => {
    expect(get(isPrViewOpen)).toBe(false);
    prViewStore.open('/repo');
    expect(get(isPrViewOpen)).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `npm run test -- --run src/lib/stores/prViewStore.test.ts`
Expected: FAIL (module not found)

- [ ] **Step 3: Implement prViewStore**

```typescript
// src/lib/stores/prViewStore.ts
import { writable, derived } from 'svelte/store';

export interface PrViewState {
  isOpen: boolean;
  projectPath: string | null;
}

const initialState: PrViewState = {
  isOpen: false,
  projectPath: null,
};

function createPrViewStore() {
  const { subscribe, set } = writable<PrViewState>(initialState);

  return {
    subscribe,
    open: (projectPath: string) => set({ isOpen: true, projectPath }),
    close: () => set(initialState),
  };
}

export const prViewStore = createPrViewStore();

export const isPrViewOpen = derived(prViewStore, ($prView) => $prView.isOpen);
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `npm run test -- --run src/lib/stores/prViewStore.test.ts`
Expected: All 4 tests PASS

- [ ] **Step 5: Write failing test for prStore**

```typescript
// src/lib/stores/prStore.test.ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { get } from 'svelte/store';
import { prStore, prCount, hasPrs } from './prStore';
import { prService } from '@/lib/services/prService';

vi.mock('@/lib/services/prService', () => ({
  prService: {
    checkGhCli: vi.fn(),
    listPrs: vi.fn(),
    getPrDetail: vi.fn(),
  },
}));

describe('prStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    prStore.clear();
  });

  describe('refresh', () => {
    it('should load PRs on success', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({
        installed: true,
        authenticated: true,
      });
      vi.mocked(prService.listPrs).mockResolvedValue([
        {
          number: 1,
          title: 'PR 1',
          author_login: 'dev',
          head_ref_name: 'feat/one',
          state: 'OPEN',
          updated_at: '2026-03-29T00:00:00Z',
          additions: 10,
          deletions: 5,
          changed_files: 2,
          body: '',
          review_decision: null,
          status_check_rollup: [],
          labels: [],
          files: [],
        },
      ]);

      await prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.prs).toHaveLength(1);
      expect(state.prs[0].number).toBe(1);
      expect(state.isLoading).toBe(false);
      expect(state.error).toBeNull();
      expect(state.ghAvailable).toBe(true);
    });

    it('should set ghAvailable false when gh is not installed', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({
        installed: false,
        authenticated: false,
      });

      await prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.ghAvailable).toBe(false);
      expect(state.prs).toEqual([]);
    });

    it('should set error on fetch failure', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({
        installed: true,
        authenticated: true,
      });
      vi.mocked(prService.listPrs).mockRejectedValue(new Error('network error'));

      await prStore.refresh('/repo');

      const state = get(prStore);
      expect(state.error).toBe('network error');
      expect(state.isLoading).toBe(false);
    });
  });

  describe('selectPr', () => {
    it('should fetch and set selected PR detail', async () => {
      const mockPr = {
        number: 42,
        title: 'PR 42',
        author_login: 'dev',
        head_ref_name: 'feat/test',
        state: 'OPEN',
        updated_at: '2026-03-29T00:00:00Z',
        additions: 50,
        deletions: 10,
        changed_files: 3,
        body: 'description',
        review_decision: 'APPROVED',
        status_check_rollup: [],
        labels: [],
        files: [{ path: 'src/app.ts', additions: 50, deletions: 10 }],
      };
      vi.mocked(prService.getPrDetail).mockResolvedValue(mockPr);

      await prStore.selectPr('/repo', 42);

      const state = get(prStore);
      expect(state.selectedPr).toEqual(mockPr);
    });

    it('should set error when detail fetch fails', async () => {
      vi.mocked(prService.getPrDetail).mockRejectedValue(new Error('not found'));

      await prStore.selectPr('/repo', 999);

      const state = get(prStore);
      expect(state.error).toBe('not found');
      expect(state.selectedPr).toBeNull();
    });
  });

  describe('clearSelection', () => {
    it('should clear selected PR', async () => {
      const mockPr = {
        number: 1,
        title: 'test',
        author_login: 'dev',
        head_ref_name: 'test',
        state: 'OPEN',
        updated_at: '',
        additions: 0,
        deletions: 0,
        changed_files: 0,
        body: '',
        review_decision: null,
        status_check_rollup: [],
        labels: [],
        files: [],
      };
      vi.mocked(prService.getPrDetail).mockResolvedValue(mockPr);
      await prStore.selectPr('/repo', 1);

      prStore.clearSelection();

      const state = get(prStore);
      expect(state.selectedPr).toBeNull();
    });
  });

  describe('derived stores', () => {
    it('prCount should reflect PR list length', async () => {
      vi.mocked(prService.checkGhCli).mockResolvedValue({
        installed: true,
        authenticated: true,
      });
      vi.mocked(prService.listPrs).mockResolvedValue([
        {
          number: 1,
          title: 'a',
          author_login: 'x',
          head_ref_name: 'b',
          state: 'OPEN',
          updated_at: '',
          additions: 0,
          deletions: 0,
          changed_files: 0,
          body: '',
          review_decision: null,
          status_check_rollup: [],
          labels: [],
          files: [],
        },
        {
          number: 2,
          title: 'b',
          author_login: 'x',
          head_ref_name: 'c',
          state: 'OPEN',
          updated_at: '',
          additions: 0,
          deletions: 0,
          changed_files: 0,
          body: '',
          review_decision: null,
          status_check_rollup: [],
          labels: [],
          files: [],
        },
      ]);

      await prStore.refresh('/repo');

      expect(get(prCount)).toBe(2);
      expect(get(hasPrs)).toBe(true);
    });

    it('hasPrs should be false when empty', () => {
      expect(get(hasPrs)).toBe(false);
    });
  });
});
```

- [ ] **Step 6: Run test to verify it fails**

Run: `npm run test -- --run src/lib/stores/prStore.test.ts`
Expected: FAIL (module not found)

- [ ] **Step 7: Implement prStore**

```typescript
// src/lib/stores/prStore.ts
import { writable, derived } from 'svelte/store';
import { prService } from '@/lib/services/prService';
import type { PullRequest } from '@/lib/services/prService';

export interface PrState {
  prs: PullRequest[];
  selectedPr: PullRequest | null;
  isLoading: boolean;
  error: string | null;
  ghAvailable: boolean;
}

const initialState: PrState = {
  prs: [],
  selectedPr: null,
  isLoading: false,
  error: null,
  ghAvailable: false,
};

function createPrStore() {
  const { subscribe, set, update } = writable<PrState>(initialState);

  return {
    subscribe,

    refresh: async (repoPath: string) => {
      update((state) => ({ ...state, isLoading: true, error: null }));

      try {
        const status = await prService.checkGhCli();
        if (!status.installed || !status.authenticated) {
          update((state) => ({
            ...state,
            ghAvailable: false,
            prs: [],
            isLoading: false,
          }));
          return;
        }

        const prs = await prService.listPrs(repoPath);
        update((state) => ({
          ...state,
          prs,
          ghAvailable: true,
          isLoading: false,
        }));
      } catch (e) {
        update((state) => ({
          ...state,
          error: e instanceof Error ? e.message : String(e),
          isLoading: false,
        }));
      }
    },

    selectPr: async (repoPath: string, number: number) => {
      update((state) => ({ ...state, error: null }));
      try {
        const pr = await prService.getPrDetail(repoPath, number);
        update((state) => ({ ...state, selectedPr: pr }));
      } catch (e) {
        update((state) => ({
          ...state,
          selectedPr: null,
          error: e instanceof Error ? e.message : String(e),
        }));
      }
    },

    clearSelection: () => {
      update((state) => ({ ...state, selectedPr: null }));
    },

    clear: () => set(initialState),
  };
}

export const prStore = createPrStore();

export const prCount = derived(prStore, ($prStore) => $prStore.prs.length);
export const hasPrs = derived(prStore, ($prStore) => $prStore.prs.length > 0);
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `npm run test -- --run src/lib/stores/prStore.test.ts`
Expected: All 7 tests PASS

- [ ] **Step 9: Commit**

```bash
git add src/lib/stores/prViewStore.ts src/lib/stores/prViewStore.test.ts src/lib/stores/prStore.ts src/lib/stores/prStore.test.ts
git commit -m "feat(pr): add prStore and prViewStore"
```

---

## Task 5: Status Bar PR Button

**Files:**
- Modify: `src/lib/components/layout/StatusBar.svelte`

- [ ] **Step 1: Import PR stores in StatusBar**

Add imports at the top of `StatusBar.svelte`:

```typescript
import { prViewStore } from '@/lib/stores/prViewStore';
import { prCount, hasPrs } from '@/lib/stores/prStore';
```

- [ ] **Step 2: Add PR button handler**

Add after the `handleWorktreesClick` function:

```typescript
function handlePrClick() {
  if (!$currentProjectPath) return;
  if ($isSubdirectoryOfRepo) {
    toastStore.warning('PRs can only be viewed from the repository root.', 4000);
    return;
  }
  prViewStore.open($currentProjectPath);
}
```

- [ ] **Step 3: Add PR button to template**

Add after the worktrees button block (after line 237 `{/if}`), before the git changes button:

```svelte
{#if gitInfo?.branch && !$isWorktree}
  <button
    class="status-item pr-btn"
    class:has-prs={$hasPrs}
    class:disabled={$isSubdirectoryOfRepo}
    onclick={handlePrClick}
    title={$isSubdirectoryOfRepo
      ? 'PRs unavailable (open from repo root)'
      : `Pull Requests (${$prCount}) - ⌘⇧P`}
  >
    <svg
      width="12"
      height="12"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
    >
      <circle cx="18" cy="18" r="3"></circle>
      <circle cx="6" cy="6" r="3"></circle>
      <path d="M13 6h3a2 2 0 0 1 2 2v7"></path>
      <line x1="6" y1="9" x2="6" y2="21"></line>
    </svg>
    <span>PRs</span>
    {#if $prCount > 0}
      <span class="pr-count">{$prCount}</span>
    {/if}
    <span class="shortcut-key">⌘⇧P</span>
  </button>
{/if}
```

- [ ] **Step 4: Add CSS styles for PR button**

Add to the `<style>` section:

```css
.pr-btn.has-prs {
  color: var(--accent-color);
}

.pr-count {
  font-size: 9px;
  font-weight: 600;
  background: var(--accent-color);
  color: var(--bg-primary);
  border-radius: 8px;
  padding: 0 4px;
  min-width: 14px;
  text-align: center;
  line-height: 14px;
}
```

- [ ] **Step 5: Commit**

```bash
git add src/lib/components/layout/StatusBar.svelte
git commit -m "feat(pr): add PR button to status bar with badge"
```

---

## Task 6: PR Panel Component — `PrPanel.svelte`

**Files:**
- Create: `src/lib/components/pr/PrPanel.svelte`

This is a large UI component. Use the `frontend-design` skill when implementing this component to follow kiri's Mist design concept.

- [ ] **Step 1: Create PrPanel component skeleton**

Create `src/lib/components/pr/PrPanel.svelte` with the following structure:

```svelte
<script lang="ts">
  import { prStore, prCount, hasPrs } from '@/lib/stores/prStore';
  import { worktreeService } from '@/lib/services/worktreeService';
  import { toastStore } from '@/lib/stores/toastStore';
  import type { PullRequest } from '@/lib/services/prService';

  interface Props {
    projectPath: string;
    onClose: () => void;
  }

  let { projectPath, onClose }: Props = $props();

  let view: 'list' | 'detail' = $state('list');

  onMount(async () => {
    await prStore.refresh(projectPath);
  });

  function handleSelectPr(pr: PullRequest) {
    prStore.selectPr(projectPath, pr.number);
    view = 'detail';
  }

  function handleBack() {
    prStore.clearSelection();
    view = 'list';
  }

  async function handleRefresh() {
    await prStore.refresh(projectPath);
  }

  async function handleOpenLocally() {
    const pr = $prStore.selectedPr;
    if (!pr) return;

    try {
      await worktreeService.create(projectPath, pr.head_ref_name.replace(/\//g, '-'), pr.head_ref_name, false);
      toastStore.success(`Worktree created for PR #${pr.number}`);
    } catch (e) {
      toastStore.error(`Failed to create worktree: ${e instanceof Error ? e.message : String(e)}`);
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      if (view === 'detail') {
        handleBack();
      } else {
        onClose();
      }
      e.preventDefault();
    }
  }

  function getRelativeTime(dateStr: string): string {
    const now = Date.now();
    const date = new Date(dateStr).getTime();
    const diffMs = now - date;
    const diffMin = Math.floor(diffMs / 60000);
    if (diffMin < 60) return `${diffMin}m ago`;
    const diffH = Math.floor(diffMin / 60);
    if (diffH < 24) return `${diffH}h ago`;
    const diffD = Math.floor(diffH / 24);
    return `${diffD}d ago`;
  }

  function getCiStatusIcon(pr: PullRequest): { icon: string; color: string } {
    if (pr.status_check_rollup.length === 0) return { icon: '○', color: 'var(--text-muted)' };
    const hasFailure = pr.status_check_rollup.some((c) => c.conclusion === 'FAILURE');
    if (hasFailure) return { icon: '✕', color: 'var(--color-error, #f85149)' };
    const allSuccess = pr.status_check_rollup.every((c) => c.conclusion === 'SUCCESS');
    if (allSuccess) return { icon: '✓', color: 'var(--color-success, #3fb950)' };
    return { icon: '◔', color: 'var(--accent3-color)' };
  }
</script>

<svelte:window onkeydown={handleKeyDown} />

<div class="pr-panel">
  <div class="pr-panel-glow"></div>
  <div class="pr-panel-content">
    <div class="pr-panel-content-inner">
      <!-- Header -->
      <div class="panel-header">
        {#if view === 'detail'}
          <button class="back-btn" onclick={handleBack}>← Back</button>
        {/if}
        <h2>{view === 'list' ? 'Pull Requests' : `PR #${$prStore.selectedPr?.number}`}</h2>
        {#if view === 'list'}
          <button class="refresh-btn" onclick={handleRefresh} title="Refresh">↻</button>
        {/if}
      </div>

      <!-- Content -->
      {#if !$prStore.ghAvailable}
        <div class="gh-not-available">
          <p><code>gh</code> CLI is required for PR integration.</p>
          <p>Install: <code>brew install gh</code></p>
          <p>Then: <code>gh auth login</code></p>
        </div>
      {:else if $prStore.isLoading}
        <div class="loading-state">Loading...</div>
      {:else if $prStore.error}
        <div class="error-state">{$prStore.error}</div>
      {:else if view === 'list'}
        <!-- PR List View -->
        {#if $prStore.prs.length === 0}
          <div class="empty-state">No open pull requests</div>
        {:else}
          <div class="pr-list">
            {#each $prStore.prs as pr}
              {@const ci = getCiStatusIcon(pr)}
              <button class="pr-row" onclick={() => handleSelectPr(pr)}>
                <span class="pr-number">#{pr.number}</span>
                <span class="pr-title">{pr.title}</span>
                <span class="pr-meta">
                  <span class="pr-author">{pr.author_login}</span>
                  <span class="pr-ci" style="color: {ci.color}">{ci.icon}</span>
                  <span class="pr-updated">{getRelativeTime(pr.updated_at)}</span>
                </span>
              </button>
            {/each}
          </div>
        {/if}
      {:else if view === 'detail' && $prStore.selectedPr}
        <!-- PR Detail View -->
        {@const pr = $prStore.selectedPr}
        {@const ci = getCiStatusIcon(pr)}
        <div class="pr-detail">
          <div class="pr-detail-header">
            <h3>{pr.title}</h3>
            <div class="pr-detail-meta">
              <span class="pr-branch">{pr.head_ref_name}</span>
              <span class="pr-ci-detail" style="color: {ci.color}">{ci.icon}</span>
              <span>by {pr.author_login}</span>
              <span>{getRelativeTime(pr.updated_at)}</span>
              {#if pr.review_decision}
                <span class="pr-review-status">{pr.review_decision}</span>
              {/if}
            </div>
          </div>

          {#if pr.body}
            <div class="pr-body">{pr.body}</div>
          {/if}

          <div class="pr-stats">
            <span class="additions">+{pr.additions}</span>
            <span class="deletions">-{pr.deletions}</span>
            <span>{pr.changed_files} files</span>
          </div>

          {#if pr.files.length > 0}
            <div class="pr-files">
              <h4>Changed files</h4>
              {#each pr.files as file}
                <div class="pr-file-row">
                  <span class="file-path">{file.path}</span>
                  <span class="file-changes">
                    <span class="additions">+{file.additions}</span>
                    <span class="deletions">-{file.deletions}</span>
                  </span>
                </div>
              {/each}
            </div>
          {/if}

          <div class="pr-actions">
            <button class="open-locally-btn" onclick={handleOpenLocally}>
              Open locally
            </button>
            <a
              class="view-github-link"
              href="#"
              onclick|preventDefault={() => {
                // Open in browser via Tauri opener
              }}
            >
              View on GitHub ↗
            </a>
          </div>
        </div>
      {/if}

      <!-- Footer -->
      <div class="panel-footer">
        <span class="footer-item">
          <kbd>Esc</kbd>
          <span>{view === 'detail' ? 'back' : 'close'}</span>
        </span>
        {#if view === 'list'}
          <span class="footer-item">
            <kbd>↵</kbd>
            <span>open</span>
          </span>
        {/if}
      </div>
    </div>
  </div>
</div>
```

Follow the WorktreePanel CSS patterns for styling: glass effect, glow, shine line, slide-in animation. The CSS will be substantial — implement it using the `frontend-design` skill for kiri's Mist design concept.

- [ ] **Step 2: Commit**

```bash
git add src/lib/components/pr/PrPanel.svelte
git commit -m "feat(pr): add PrPanel component with list and detail views"
```

---

## Task 7: App Integration — Render Panel and Keyboard Shortcut

**Files:**
- Modify: `src/App.svelte`

- [ ] **Step 1: Import PR stores and component in App.svelte**

Add imports:

```typescript
import PrPanel from '@/lib/components/pr/PrPanel.svelte';
import { prViewStore } from '@/lib/stores/prViewStore';
import { prStore } from '@/lib/stores/prStore';
```

- [ ] **Step 2: Add keyboard shortcut handler**

Add in the `handleKeyDown` function, alongside the existing Cmd+G handler:

```typescript
// Cmd+Shift+P: Toggle PR panel (only when project is open and not in worktree)
if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'p' && $isProjectOpen && !$isWorktree) {
  e.preventDefault();
  const path = projectStore.getCurrentPath();
  if (path) {
    if ($isSubdirectoryOfRepo) {
      toastStore.warning('PRs can only be viewed from the repository root.', 4000);
      return;
    }
    if ($prViewStore.isOpen) {
      prViewStore.close();
    } else {
      prViewStore.open(path);
    }
  }
  return;
}
```

- [ ] **Step 3: Add PR panel rendering**

Add in the template, near the WorktreePanel rendering (around line 608-612):

```svelte
{#if $prViewStore.isOpen && $prViewStore.projectPath}
  <PrPanel
    projectPath={$prViewStore.projectPath}
    onClose={() => prViewStore.close()}
  />
{/if}
```

- [ ] **Step 4: Trigger PR count fetch on project open**

In the project initialization logic, add PR data fetch alongside existing git data loading:

```typescript
// After project is loaded and git info is available
prStore.refresh(projectPath);
```

This should be called where `worktreeStore.refresh()` is called, so the PR count is available for the status bar badge.

- [ ] **Step 5: Commit**

```bash
git add src/App.svelte
git commit -m "feat(pr): integrate PrPanel into App with Cmd+Shift+P shortcut"
```

---

## Task 8: Worktree Window PR Header

**Files:**
- Modify: Worktree window component (the component that renders in worktree windows)

When a worktree is created from the PR detail view, pass PR metadata (number, title, CI status, branch) via URL parameters to the new window. In the worktree window, read these params and display a compact PR info header bar above the terminal.

- [ ] **Step 1: Pass PR info when creating worktree**

In `PrPanel.svelte`'s `handleOpenLocally`, after worktree creation, open the new window with PR metadata encoded in URL params. Follow the existing `create_window` pattern from `windowService`.

- [ ] **Step 2: Read PR params in worktree window**

Add logic to read `pr_number`, `pr_title`, `pr_ci_status` from URL params on mount.

- [ ] **Step 3: Render PR header bar conditionally**

If PR params are present, show a compact header: `PR #42: feat: add auth  ✓ CI passed  feat/auth`

- [ ] **Step 4: Commit**

```bash
git commit -am "feat(pr): show PR info header in worktree windows"
```

---

## Task 9: WorktreePanel PR Column Enhancement

**Files:**
- Modify: `src/lib/components/git/WorktreePanel.svelte`

- [ ] **Step 1: Cross-reference worktrees with PRs**

When the worktree list loads, check if any worktree's branch matches a PR's `head_ref_name`. If so, display the PR number and title next to the worktree entry.

- [ ] **Step 2: Add PR info display to worktree list rows**

For each worktree row, if it has a matching PR, show: `PR #42` badge next to the branch name.

- [ ] **Step 3: Commit**

```bash
git add src/lib/components/git/WorktreePanel.svelte
git commit -m "feat(pr): show PR info in worktree list"
```

---

## Task 10: Final Integration Test and Cleanup

- [ ] **Step 1: Run all frontend tests**

Run: `npm run test`
Expected: All tests pass

- [ ] **Step 2: Run all Rust tests**

Run: `npm run test:rust`
Expected: All tests pass

- [ ] **Step 3: Run lint and type check**

Run: `npm run lint && npm run check`
Expected: No errors

- [ ] **Step 4: Manual verification**

Run: `npm run tauri dev`
Verify:
1. PR button visible in status bar (main worktree only)
2. Badge shows PR count with accent color when > 0
3. Cmd+Shift+P opens PR panel
4. PR list loads and displays correctly
5. Clicking a PR shows detail view
6. "Open locally" creates worktree and opens new window
7. Escape navigates back / closes panel
8. Worktree list shows PR info for matching branches

- [ ] **Step 5: Final commit if needed**

```bash
git commit -am "chore: cleanup PR integration"
```
