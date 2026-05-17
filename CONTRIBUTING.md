# Contributing to kiri

Thanks for your interest in kiri. This document is the single entry point for
human contributors. It covers the local setup, the commands you will run while
iterating, our test policy, branch and commit conventions, and what reviewers
expect to see in a pull request.

If you are an AI coding agent (Claude, Codex, etc.) operating in this repo, see
the agent-only notes under `.claude/` (gitignored, intentionally local). The
contents of this file are the canonical contributor guidelines; AI agents
should also follow them.

## Project at a glance

- App framework: Tauri 2 (Rust + Svelte 5)
- Frontend: Svelte 5 + TypeScript (Vite)
- Backend: Rust (`src-tauri/`) plus standalone crates in `crates/`
- Terminal: xterm.js
- File viewer: CodeMirror 6
- CLI transport: Unix Domain Sockets + clap (`crates/kiri-cli`)

Supported platform today: macOS Apple Silicon. Other platforms build from
source but are not regularly smoke-tested. See
[`docs/platform-support.md`](docs/platform-support.md).

## Setup

Requirements:

- Node.js 20+
- A recent stable Rust toolchain (`rustup default stable`)
- macOS Apple Silicon for the full app experience (see platform support doc
  for other targets)

Then:

```bash
git clone https://github.com/t09tanaka/kiri.git
cd kiri
npm install
```

`npm install` runs `husky` once via the `prepare` script, which wires up the
git hooks in `.husky/`.

## Day-to-day commands

| Command | What it does |
|---|---|
| `npm run tauri dev` | Run the app in dev mode (hot reload) |
| `npm run build:cli` | Build the standalone `kiri-cli` binary |
| `npm run build:app` | Build the production app bundle (also builds the CLI) |
| `npm run install:app` | Build and install the macOS app into `/Applications` |
| `npm run check` | `svelte-check` (TypeScript + Svelte) |
| `npm run lint` / `npm run lint:fix` | ESLint over JS/TS/Svelte sources |
| `npm run format` / `npm run format:check` | Prettier + rustfmt |
| `npm run test` | Frontend unit tests (Vitest) |
| `npm run test:browser` | Vitest browser-mode tests |
| `npm run test:rust` | Rust tests via `cargo test` |
| `npm run test:all` | All Vitest projects |
| `npm run perf:check` | Bundle-size check against `perf-baselines/` |

When you change Rust code that ships in the binary bundle, run `npm run
build:cli` before `cargo test` (the Tauri build asserts that the CLI binary
exists — CI does the same).

## Testing policy

We treat tests as documentation of behaviour, not just regression bait. The
test categories below mirror the directory layout under `src/lib/__tests__/`
and the Rust modules in `src-tauri/src/`.

| Category | Runner | When to write one |
|---|---|---|
| **Unit** | `npm run test` (Vitest, node) / `cargo test` | A pure module with no DOM, IPC, or filesystem dependencies |
| **Browser** | `npm run test:browser` (Vitest + Playwright) | A Svelte component that needs a real DOM, layout, or focus behaviour |
| **Integration** | `cargo test` | Code that crosses module boundaries inside `src-tauri/` (ring buffer + dispatcher, etc.) |
| **End-to-end** | manual / build:app + smoke script | Whole-app flows that require the bundled binary |

Guidelines:

- **Cover the new branch you write.** If you add an `if`, add a test for both
  arms. We aim for ~100% line coverage on pure modules; we accept lower for
  modules that wrap I/O.
- **Delete dead code.** If a test only passes because a feature flag is off,
  remove the flag or the test. The CI suite must reflect real behaviour.
- **Prefer narrow tests.** A failing unit test should localise the bug to one
  module. Browser tests are expensive — reach for them only when a real DOM
  matters.
- **Tests must be deterministic.** No sleeps, no real network, no random
  inputs without a seed.
- **Snapshot tests must be small.** A 500-line snapshot is a regression
  trap; split it.

Run the relevant subset locally before pushing:

```bash
npm run lint
npm run check
npm run test
npm run test:rust
```

The `.husky/pre-commit` hook runs the fast feedback (lint-staged + frontend
unit). The full Rust suite and bundle-size check run in CI on every PR.

## Branch naming

Use a short topical prefix and a hyphenated slug:

| Prefix | Use for |
|---|---|
| `feature/` | New user-visible behaviour |
| `fix/` | Bug fix |
| `docs/` | Documentation-only change |
| `refactor/` | Internal change with no behaviour change |
| `perf/` | Performance work |
| `chore/` | Tooling, dependency bumps, CI |

Examples:

```
feature/keybinding-customization-ui
fix/issue-17-cli-bridge-window-scoped-listen
docs/contributing-guide
```

Reference an issue when one exists: `fix/issue-123-short-slug`.

## Commit format

We follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body — optional, wrapped at ~72 cols>

<footer — optional, e.g. `Closes #123`>
```

Common `<type>` values: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`,
`chore`, `ci`, `build`.

Common scopes: `cli`, `terminal`, `frontend`, `backend`, `worktree`, `remote`,
`ci`.

Write the subject in the imperative ("add foo", not "added foo"). Explain the
**why** in the body when it isn't obvious from the diff.

## Pull request checklist

Before requesting review:

- [ ] Branch is rebased on top of latest `main`
- [ ] `npm run lint` passes
- [ ] `npm run check` passes
- [ ] `npm run test` passes
- [ ] `npm run test:rust` passes (or you note why it is skipped)
- [ ] PR description fills out **Why / How / Verification** sections from
      `.github/pull_request_template.md`
- [ ] Screenshots or short clips attached for any visible UI change
- [ ] `Closes #<n>` is linked when the PR resolves an issue

CI mirrors these checks — see `.github/workflows/ci.yml` for the exact jobs.
If CI fails on a check you already ran locally, leave a comment with the
discrepancy; we treat that as a bug in CI, not in your PR.

## Code review expectations

- Reviewers look first at the PR description. If the **Why / How /
  Verification** sections are empty, expect a review to bounce back asking
  for context.
- Reviews focus on correctness, fit with existing patterns, and reviewability
  of the diff. Style nits should be handled by the formatter, not the
  reviewer.
- Authors: respond to every comment, even with `done` or `agreed`. Don't
  resolve other people's threads.
- Reviewers: prefer `suggestion:` blocks for small fixes; describe the larger
  changes in prose so the author can decide.
- For non-trivial changes, request at least one review from a maintainer.

## Where things live

- Frontend Svelte code: `src/`
- Frontend tests: `src/lib/__tests__/`
- Tauri host (Rust): `src-tauri/src/`
- Standalone crates (CLI, proto): `crates/`
- Contributor docs (this and related): root + `docs/contributing/`
- Maintainer-only docs (release, deploys): `docs/maintainers/`
- Feature designs and plans: `docs/features/`, `docs/plans/`,
  `docs/superpowers/`
- CI workflows: `.github/workflows/`

If you're not sure where a new file goes, open a draft PR and ask — wrong
location is a five-minute fix and not a blocker.

## License

By contributing you agree that your contribution is licensed under the MIT
License, the same license as the rest of the repo. See [`LICENSE`](LICENSE).
