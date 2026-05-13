# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0] - 2026-05-14

### Added

#### Terminal — pane labels (`kiri term split --name --color`)
- `kiri term split --name STR --color COLOR` tags a newly-split pane with a short label and one of six fixed Mist-palette colors (`sky | iris | jade | amber | coral | rose`)
  - Pane header renders a `8px` colored dot plus the name, between the split buttons and the worktree tag
  - Both flags are independent — `--name` only (text), `--color` only (dot), both, or neither all work
  - CLI rejects names that are empty, > 32 Unicode scalar values, or contain control characters; backend re-validates the same rules as defense-in-depth and replies with `ErrorCode::InvalidArgument` if a raw-protocol client bypasses the CLI
- `kiri term ls` returns optional `name` and `color` fields per `PaneInfo` (omitted entirely when unset — not `null`); pretty-table output gains `NAME` and `COLOR` columns
- Six `--pane-color-*` CSS tokens added to the root palette in `app.css`
- New `PaneColor` enum on the wire (`kiri-cli-proto`) and `ErrorCode::InvalidArgument` for label validation failures

#### Terminal — minimize / restore (`kiri term minimize/restore` and `split --minimized`)
- New `kiri term minimize` / `kiri term restore` subcommands collapse the per-pane shortcut bar to a thin strip and expand it back
- New `kiri term split --minimized` creates the new pane with its shortcut bar already collapsed — useful when an agent spawns a side pane and does not want to push the user's primary view down
- `kiri term ls` returns the new `minimized` boolean (always present) on each `PaneInfo`
- Shortcut bar gained a collapse button and a thin-bar layout for the collapsed state

#### Bundled skill
- `kiri-cli` SKILL.md bumped to `0.2.0` with documentation for the new subcommands and split flags; the in-app skill install dialog will offer a `0.1.0 → 0.2.0` upgrade

### Fixed

#### Terminal
- Guard `xterm.open()` against a detached container so async xterm lazy-loading no longer throws "Terminal requires a parent element" when a pane unmounts during init (also clears 4 noisy unhandled rejections from the browser test run)
- Right-aligned trailing header cluster (`pane-label` + `worktree-tag` + close button) now uses a single flex spacer instead of `margin-left: auto` on `.worktree-tag`, so single-pane worktree headers stay right-aligned even when the close button is hidden

### Removed

- `refactor(terminal): remove frequent-command suggestion feature` — drop the in-app frequent-command suggester that was unused after the CLI / skill flows landed

## [0.4.1] - 2026-04-29

### Fixed

#### CLI
- Start the per-window CLI server for every project entry point, not just URL `?project=`
  - Previously `register_window` was only called from the focus_or_create_window flow; opening a project via Cmd+O, the start-screen "Open Folder" button, or Recent Projects never created a socket file, so `kiri term *` always failed with `internal_error: no kiri windows are running`
  - The CLI register/unregister now mirrors `projectStore.currentPath` reactively, so every entry point (URL param, Cmd+O, start screen, Cmd+Shift+W close, project switch) converges on the same setup/teardown path
- Recover `kiri term run` output capture when the shell echoes the payload inline
  - First `run` succeeded but subsequent runs returned `output: ""` even with a correct exit code
  - `extract_output` now uses the regex match's start position to locate the real sentinel line (instead of scanning for the literal `__KIRI_DONE_` substring, which collides with the printf format string in the shell's echo)
  - Hardened leading-echo stripping with extra shapes for ANSI-redraw-mangled echoes (`g\x08git status; printf …`)

## [0.4.0] - 2026-04-29

### Added

#### CLI
- New `kiri` CLI binary for driving the app from inside its terminal panes
  - Wire protocol crate (`kiri-cli-proto`) with `Request`/`Response`/`PaneInfo`/`PaneRef`/`SplitDirection`/`ErrorCode`
  - clap-based surface with UDS transport and pretty rendering (`kiri term ls/run/send/read/follow/cancel/split/close`)
  - Per-window CLI server started/stopped on window register/unregister
  - Pure modules: `ring_buffer`, `run_logic`, `pane_map`, `frontend_bridge`
  - Dispatch + handlers + listener + Tauri commands
  - `TerminalOutputBus` for in-process subscribers
  - Frontend `cliBridge` + `focusedPaneStore` to power the in-PTY `kiri` command
  - Inject `kiri` into PTY env and install the binary on startup
  - Fall back to socket discovery when `KIRI_SOCKET` is stale

#### Skill installer
- New `skill_install` backend module + Tauri commands for installing Claude Code skills
- Install confirmation dialog with English copy + service wrapper
- Bundled `kiri-cli` SKILL.md for Claude Code agents

### Fixed

#### CLI
- Spawn the listener inside Tauri's tokio runtime (fixes startup hang)
- Map frontend error codes by name instead of collapsing them to `PaneNotFound`
- Address code review findings across cli-server (5 issues)

#### CI
- Grant `contents: write` to the release create job
- Resolve pre-existing clippy 1.95 lints and stale tab-system test

### Changed

- Bump `@tauri-apps/plugin-dialog` to 2.7.0 and add `@tauri-apps/plugin-fs`
- Move `Cargo.lock` to the workspace root and gitignore workspace `target/`

## [0.3.0] - 2026-04-27

### Added

#### Pull Requests
- New PR panel (Cmd+Shift+P) with list and detail views
  - `gh` CLI integration for fetching PR data
  - Status bar PR button with badge
  - PR info header in worktree windows and worktree list

#### Terminal
- Show worktree tag in panel header when in a linked worktree
  - Detects worktree info via new `get_worktree_info` Tauri command
  - Polls worktree info on cwd changes
  - Long worktree names ellipsis-truncate

### Changed

#### Terminal
- **Breaking**: Drop tab system — single terminal per window
- Align controls bar height with sidebar header
- Load shortcut/settings state on every pane mount

### Removed

#### Worktree
- **Breaking**: Remove the entire git worktree integration
  - Frontend `WorktreePanel`
  - Backend `git_worktree` / `port_isolation` / `compose_isolation` commands
  - `WindowRegistry` worktree tracking
  - PR panel "Open locally" flow that created worktrees

#### Terminal
- Drop memory indicator UI

## [0.2.2] - 2026-03-28

### Added

#### Terminal
- Add shortcut suggestions feature that learns from user input patterns
  - `InputRecord` type and persistence for tracking command frequency
  - `inputStatsService` with recording logic and 1000-entry eviction limit
  - Suggestion filtering and dismiss logic
  - `detectShortcutType` for auto-detecting command vs reply shortcuts
  - `ShortcutSuggestions` component with badge and popover UI
  - Integrated into `TerminalShortcutBar` and `Terminal`

### Fixed

#### Terminal
- Close suggestion popover on outside click and clear input buffer on AI exit

## [0.2.1] - 2026-03-27

### Changed

#### Worktree
- Replace glob-based copy patterns with .gitignore-based file copying
  - Automatically detects all .gitignore rules (root + nested)
  - Each rule can be toggled ON/OFF in Settings
  - Copies all gitignored files (node_modules, dist, .env, etc.) instead of running npm install
- Remove auto-detected package manager initialization commands
  - Only user-defined commands are shown and executed
- Instant panel startup with lazy background data loading
  - Footer stats show spinner→checkmark as each item loads

### Fixed

#### Terminal
- Observe terminalPadding to refit on shortcut bar visibility change

### Added

#### Shortcuts
- Add number row (1, 2, 3) for quick selection in shortcut bar

## [0.1.0] - 2026-03-22

First feature-complete milestone release.

### Added

#### File Tree
- File tree navigation with collapsible project header
- Git status display with color propagation to parent directories
- Gitignored files shown as semi-transparent
- File icons with color schemes for 60+ extensions
- Special icons for config files, Docker files, .env files, markdown, and test files
- Test file grouping under parent with tree connectors
- Create directory from context menu
- Delete file/folder functionality
- Drag-and-drop file move (internal) and file copy (from Finder)
- Drag preview with optimistic UI

#### Terminal
- Terminal with PTY support and multiple tabs
- Pane splitting with draggable dividers
- Foreground process name as tab title
- Memory usage display in pane header
- macOS keyboard navigation shortcuts (Cmd+Delete, Shift+Enter)
- Command suggestions with inline ghost text
- Desktop notifications for terminal apps
- Close confirmation when commands are running
- OSC 8 hyperlink support with file:// handling
- Resize buffering for smoother Ink app rendering
- Claude Code / Ink app compatibility
- CWD tracking and pane structure persistence across restarts

#### Editor
- Code editor with CodeMirror 6 and syntax highlighting (40+ languages)
- Quick peek editor for terminal file path links
- In-file search (Cmd+F)
- Git diff display in editor gutter
- External file change detection
- Copy button in EditorModal and PeekEditor
- Markdown and YAML syntax highlighting

#### Git
- Git changes view with diff display in separate window
- Commit history with graph visualization
- Branch divergence display
- Infinite scroll for commit history
- Diff statistics in status bar
- Per-file line stats in DiffView file list
- Image file preview in Changes window
- Fetch, pull, and behind/ahead count
- Unread commit indicator dots
- Co-author display in commit detail

#### Search
- Quick Open file search (Cmd+P)
- Project-wide content search (Cmd+Shift+F) with syntax highlighting

#### Git Worktree
- Create/remove worktrees with branch validation
- Port isolation with automatic detection (.env, docker-compose, package.json)
- Per-project 100-port block allocation (range 20000-39999)
- Docker Compose host-only port transformation
- File copy patterns (.env, docker-compose) including subdirectories
- Package manager auto-detection (npm, yarn, pnpm, bun)
- Husky auto-detection for init commands
- Custom initialization commands
- Conflict warning banner for compose files
- Worktree disabled when opened from subdirectory

#### Window Management
- Multiple window support (Cmd+Shift+N)
- Session persistence with multi-window restore
- Window geometry persistence
- Project name in window title
- DiffView opens at same size/position as main window
- Start screen with recent projects and startup command config
- Open Recent submenu with dynamic project list

#### UI
- Mode switching between Terminal and Editor
- Custom confirm dialog with keyboard support
- Font zoom shortcuts with persistence
- Sidebar toggle button in status bar
- Keyboard shortcuts for Diff View (Cmd+D) and Worktrees (Cmd+G)
- Mountain logo loading screen and favicon
- Reusable UI component library
- Service layer for Tauri API abstraction

### Infrastructure
- GitHub Actions CI (lint, test, build)
- GitHub Actions release workflow (macOS, Windows, Linux)
- Performance measurement system (dev-only)
- Husky pre-commit hooks (lint-staged + tests)
