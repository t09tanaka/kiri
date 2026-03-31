# PR-Worktree Integration Design

## Problem

AI coding agents (Claude Code, etc.) produce multiple PRs rapidly. Reviewing them requires manually checking out branches, managing port conflicts, and cleaning up after. Existing terminal emulators like Ghostty have no concept of PR-driven workflows.

kiri already has git worktree management with automatic port isolation. This feature connects PRs directly to that workflow, making kiri the tool for "AI agent PR review."

## Target User

Developers who use AI coding agents and receive multiple PRs to review daily. They want to quickly spin up isolated environments per PR, verify changes, and tear down cleanly.

## User Flow

```
Status bar PR button (badge shows count)
  → Click → PR List view (panel)
    → Select PR → PR Detail view
      → "Open locally" button → Worktree created → New window opens
        → Review, test, verify
          → Close window → Worktree auto-deleted
```

## Components

### 1. Status Bar PR Button

- **Visibility**: Main worktree only (hidden in worktree windows)
- **Badge**: Shows count of open PRs for the current repository
- **Color**: Accent color when PR count >= 1, muted/gray when 0
- **Click action**: Opens PR List view
- **Data source**: `gh pr list --json number,title,author,state,labels,headRefName,statusCheckRollup,updatedAt`

### 2. PR List View (Panel)

Slide-in panel similar to WorktreePanel.

**Each PR row displays:**
| Field | Source |
|-------|--------|
| PR number | `#123` |
| Title | PR title (truncated) |
| Author | GitHub username + avatar |
| CI status | Pass / Fail / Pending icon |
| Updated | Relative time (e.g., "2h ago") |

**Behavior:**
- Click a row to navigate to PR Detail view (in-panel transition)
- Back button to return to list
- Refresh button to re-fetch
- No auto-refresh (manual refresh only in v1)

**Keyboard shortcuts:**
| Shortcut | Action |
|----------|--------|
| `Cmd+Shift+P` | Toggle PR panel (tentative) |
| `Escape` | Close panel |
| `↑/↓` | Navigate list |
| `Enter` | Open detail |

### 3. PR Detail View (In-panel)

**Header section:**
- PR title and `#number`
- Author, branch name, labels
- CI status with individual check names
- Created/updated timestamps

**Body section:**
- PR description (markdown rendered)
- Changed files list with +/- line counts
- Review status (approved, changes requested, pending)

**Actions:**
- **"Open locally" button** — Creates worktree from PR branch and opens new window
- **"View on GitHub" link** — Opens PR in browser

### 4. PR Workspace (New Window)

Extends the existing worktree window:

- **Header bar**: Shows PR title, `#number`, CI status, branch name
- Terminal starts in worktree directory
- Closing the window triggers worktree deletion (existing behavior)

### 5. Multi-PR Overview (WorktreePanel Enhancement)

The existing worktree list gains a PR info column:
- If a worktree's branch matches a PR, show the PR number and title
- Visual indicator distinguishing PR-linked worktrees from manual ones

## Technical Design

### Dependencies

- **`gh` CLI**: Required for GitHub API access. If not installed, show a setup guide in the PR panel.
- **GitHub authentication**: `gh auth status` must be valid.

### Data Flow

```
gh CLI → prService (frontend) → prStore (Svelte store) → UI components
```

### New Service: `prService.ts`

```typescript
interface PullRequest {
  number: number;
  title: string;
  author: { login: string; avatarUrl: string };
  headRefName: string;
  state: string;
  labels: { name: string; color: string }[];
  statusCheckRollup: CheckStatus[];
  updatedAt: string;
  body: string;
  additions: number;
  deletions: number;
  changedFiles: number;
  reviewDecision: string | null;
  files: PrFile[];
}

interface PrFile {
  path: string;
  additions: number;
  deletions: number;
}

interface CheckStatus {
  name: string;
  status: string;
  conclusion: string | null;
}
```

**Methods:**
- `listPrs(repoPath: string): Promise<PullRequest[]>` — List open PRs
- `getPrDetail(repoPath: string, number: number): Promise<PullRequest>` — Get full PR details including files
- `checkGhInstalled(): Promise<boolean>` — Verify gh CLI availability
- `checkGhAuth(): Promise<boolean>` — Verify GitHub authentication

### New Store: `prStore.ts`

| State | Type | Description |
|-------|------|-------------|
| `prs` | `PullRequest[]` | List of open PRs |
| `selectedPr` | `PullRequest \| null` | Currently viewed PR detail |
| `loading` | `boolean` | Fetch in progress |
| `error` | `string \| null` | Error message |
| `ghAvailable` | `boolean` | Whether gh CLI is installed and authed |

**Derived stores:**
- `prCount` — Number of open PRs
- `hasPrs` — Whether there are any open PRs

### Backend: `gh` CLI Execution

Use existing Rust command execution infrastructure to run `gh` commands:

```rust
// New command in src-tauri/src/commands/github_pr.rs
#[tauri::command]
pub async fn list_pull_requests(repo_path: String) -> Result<Vec<PullRequest>, String>

#[tauri::command]
pub async fn get_pull_request_detail(repo_path: String, number: u32) -> Result<PullRequest, String>

#[tauri::command]
pub async fn check_gh_cli() -> Result<GhCliStatus, String>
```

### Worktree Integration

When "Open locally" is clicked:
1. Call `worktreeService.create()` with the PR's `headRefName`
2. Port isolation applies automatically (existing behavior)
3. Gitignored file copying applies (existing behavior)
4. Init commands run (existing behavior)
5. New window opens with PR metadata passed via URL params
6. PR info stored in window context for header display

## Scope

### In Scope (v1)
- Status bar PR button with badge
- PR list panel with list → detail navigation
- "Open locally" button creating worktree
- PR info header in worktree window
- PR column in existing worktree list

### Out of Scope (v1)
- Approve / comment / merge from kiri
- PR creation
- CI re-trigger
- Review thread display
- Draft PR filtering
- Cross-repository PRs
- Notifications / auto-refresh

## UI Design

Follow kiri's "Mist" design concept:
- Glass effect panel with frosted backdrop
- Soft borders and glow accents
- Gentle slide-in animation for panel
- CI status uses color coding: green (pass), red (fail), yellow (pending)
- Use `frontend-design` skill for implementation

## Performance Considerations

- `gh pr list` can be slow on large repos — cache results, show stale data while refreshing
- PR detail fetches file list which can be large — paginate or limit display
- Avoid polling; use manual refresh + fetch on panel open
