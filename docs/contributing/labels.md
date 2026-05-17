# Labels

This is the canonical reference for the labels we apply to issues and PRs.
Apply at most one label from **Type**, one from **Area**, and one from
**Priority** (when it applies). Labels without a category below are
informal and can be added or removed freely.

## Type

| Label | Meaning |
|---|---|
| `bug` | The current behaviour is broken or contradicts the docs. |
| `enhancement` | New feature or improvement to an existing feature. |
| `performance` | Slower than the documented budget, or perceptibly slow. |
| `docs` | Documentation-only change or gap. |
| `dx` | Developer experience: tooling, scripts, hooks, lint rules. |
| `chore` | Tooling, dependency bumps, CI plumbing that ships no behaviour. |
| `security` | Reproducible security defect (use private disclosure first when in doubt). |
| `regression` | Worked in a previous release, broken now. Pair with `bug`. |

## Area

| Label | Surface |
|---|---|
| `area:cli` | The `kiri` CLI binary and the UDS protocol. |
| `area:frontend` | Svelte / TypeScript / Vite layer. |
| `area:backend` | Tauri host (Rust under `src-tauri/`). |
| `area:terminal` | PTY pipeline, xterm.js rendering. |
| `area:fileviewer` | CodeMirror-based file panel. |
| `area:worktree` | Worktree integration. |
| `area:ci` | GitHub Actions workflows and CI-only scripts. |
| `area:docs` | Files under `docs/`, `CONTRIBUTING.md`, `README.md`. |

## Priority

| Label | When to use |
|---|---|
| `priority:critical` | Data loss, crash, or security defect affecting current users. Drop other work. |
| `priority:high` | Blocks a current release or a documented workflow. Pick up next. |
| `priority:medium` | Default for everything else worth doing. |
| `priority:low` | Worth doing eventually; safe to defer past several releases. |

If you cannot decide, leave the priority off. `priority:medium` is the
implicit default; it's only worth applying explicitly to override a
different assumption.

## Lifecycle

| Label | Meaning |
|---|---|
| `good first issue` | Small, well-scoped, no project context required. |
| `help wanted` | We will accept a PR from anyone for this. |
| `needs-repro` | Reporter has not yet provided a reproducible case. |
| `needs-design` | Implementation is blocked on a UX or API decision. |
| `blocked` | Blocked on an external dependency. Note the blocker in a comment. |
| `wontfix` | We have decided not to address this. Closing comment should explain why. |
| `duplicate` | Another issue covers the same ground; link it before closing. |
| `stale` | Auto-applied after long inactivity. Removing a `stale` label resets the timer. |

## Applying labels

- **One Type label** per issue or PR. If you would add two, the issue is
  probably two issues.
- **One Area label** is the norm; a second is fine for cross-cutting work
  (e.g. `area:cli` + `area:backend`).
- **Priority is optional** but please apply it for `bug` and `regression`
  reports so triage can scan a list.
- **Lifecycle labels** are additive — `good first issue` + `area:docs`
  is a useful combination.

If you find yourself wanting a label that doesn't exist, open a PR to
this file with the new entry and a one-line justification.
