# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.1] - 2026-06-20

### Added

#### CLI
- `kiri window open <dir>` opens (or focuses) a kiri window for a directory,
  and a global `--window` selector targets a specific window for any
  `term` subcommand. Closes #142.
- `kiri term status` reports what each pane is running, including the
  detected AI tool (`ai_kind`) surfaced in `kiri term ls`.
- `OpenWindow` / `AgentStatus` wire-protocol requests and a `PaneInfo.ai_kind`
  field, with the CLI server now serving `cli:pane-snapshot` straight from the
  xterm buffer.

#### Terminal
- Panes can be minimized into a footer dock with a floating peek view, so a
  pane can be tucked away without disposing its session. Closes #143.

### Fixed
- New Window and the settings window are decoupled from the main window, so
  closing the main window no longer tears down their CLI server or state.
  Closes #147.
- CLI server sweeps dead sockets on startup and cleans up the CLI server on
  window destroy and app exit; `cleanup_window_resources` now recovers a
  poisoned lock instead of panicking.
- The startup command selection is persisted immediately instead of only on
  modal close. Closes #146.
- The terminal viewport pins to the bottom on pane split, close, and divider
  resize (not just on output). Closes #141.
- Closing the last visible pane no longer leaves a blank layout, and closing
  a docked pane disposes its PTY.

### Removed
- Terminal AI shortcut bar that appeared while `claude` / `codex` ran in a
  pane (quick-reply / command / number-pick buttons and its settings modal),
  along with the foreground-process polling that drove it.
- `kiri term minimize` / `kiri term restore` CLI subcommands and the
  `kiri term split --no-minimized` flag, plus the `minimized` wire-protocol
  fields that only controlled the shortcut bar's collapsed state.

## [0.6.0] - 2026-05-18

### Added

#### File operations
- File tree right-click context menu (`Open`, `Rename`, `New File`,
  `New Folder`, `Copy Path`, `Reveal in Finder`, `Open in Terminal here`,
  `Delete`) with inline rename / new-file editing and keyboard shortcuts
  (`F2`, `⌘N`, `⌘⇧N`, `⌫`, `⌘C`, `⌘⇧R`). Closes #82 / #84 / #90.
- New Tauri commands (`rename`, `create_file`, `trash`, `open_terminal`)
  with matching `fileService` invokers and a single-step undo for the
  most recent destructive operation.
- ContextMenu flips to the opposite side of the cursor when it would
  overflow the viewport edge, instead of being clamped under the cursor.
  Closes #60.

#### Accessibility
- `<ProgressBar>` now exposes `role="progressbar"`, `aria-valuemin/max`,
  `aria-valuenow` for determinate mode, and `aria-busy="true"` (without
  `aria-valuenow`) for indeterminate mode. Closes #52.
- Modal focus is trapped inside `DiffViewModal`, `EditorModal`,
  `ContentSearchModal`, and `QuickOpen`. Tab cycles through the modal's
  focusable elements and is pulled back if focus escapes. Closes #55.
- `--text-muted` bumped from `#484f58` to `#828993` so muted labels
  clear WCAG AA 4.5:1 contrast against `--bg-secondary` / `--bg-tertiary`.
  Closes #56.

#### UI polish
- Shared `<EmptyState>` component (mist micro-animation + tone variants)
  used by `StartScreen` ("No recent projects") and `QuickOpen` ("Type to
  search files..."). Closes #57.
- Global `<kbd>` styling applied via `app.css`; per-modal duplicate kbd
  styles in `DiffViewModal`, `EditorModal`, `ContentSearchModal`,
  `QuickOpen`, `CommitHistoryModal`, and `PeekEditor` removed. Closes #59.
- Foundation tokens added: `--control-height-{xs,sm,md,lg}` scale,
  `--terminal-padding-x/y`, plus a defaults `<kbd>` rule.

#### CLI
- `kiri env` prints the active project, window socket path, and the
  kiri-cli binary location for debugging external-terminal integrations.

#### Lint
- New `windowService`-specific ESLint rule enforces multi-window data
  passing patterns (URL params / events) over implicit store sharing.
  Closes #50.

### Changed

#### UI/UX polish (#51 #53 #54 #58)
- `Toast` and `Badge` transition timing replaced hardcoded
  `0.3s cubic-bezier` / `0.2s ease-out` values with `--transition-fast` /
  `--transition-normal` so timings stay tunable in one place.
- `Badge` `transition: all` replaced with an explicit property list
  (`transform`, `background`, `color`, `border-color`, `box-shadow`).
- StatusBar buttons (sidebar-toggle, git-branch, git-changes,
  shortcut-hint) now line up exactly via `--control-height-xs`; ad-hoc
  vertical padding removed.
- Terminal padding (`12px 16px`) now consumes `--terminal-padding-y/x`
  tokens; PTY runtime constants moved into `terminalConstants.ts`.
- Sidebar header shimmer opacity bumped from `0.03` to `0.08` so the
  animation is perceptible.

#### Refactors
- Migrated 7 stores to Svelte 5 `$state` classes via a backward-compatible
  facade so consumers stay unchanged. Closes #42.
- Split `settingsStore` into `uiPreferences` and `persistedSettings`.
  Closes #43.
- Split `DiffView` into sidebar / section / image-panel components, and
  extracted a `diffCache` helper. Closes #46 / #47.
- Extracted FileTree sort/drag-ghost/keyboard helpers and centralized the
  drag store + window listeners. Closes #48.
- Extracted Terminal theme / sync output / layout / setup helpers from
  `Terminal.svelte`. Closes #45.
- `getState()` helper replaces ad-hoc `get()` calls in components and
  settings persistence now lives in the store via auto-subscription.

#### Performance
- App: lazy-load heavy modal components from `App.svelte`.
- Editor: collapse CodeMirror language loaders into a shared lazy table.
- Filesystem: `read_directory` runs off the runtime thread with an entry
  cap; content search enforces server-side default + ceiling.
- Terminal: defer `WebLinks` + `Canvas` xterm addons. Closes #36.
- Rust: cut `String`/`Vec` churn on `git_history` and process snapshots;
  disable unused `git2` default features.
- CLI: coalesce burst pane-map invokes into one microtask.
- DX: `perf:measure` now emits a startup phase breakdown; new
  `perf:bundle-report` script.

#### Security & hardening
- Cloudflare tunnel token now validated as defense-in-depth before
  invocation.
- Drag-and-drop copy skips symlinks and bounds recursion depth.
- Tauri capabilities split per-feature; redundant grants dropped.
- IPC error messages redact filesystem paths.
- Rust startup exits gracefully on Tauri runtime failure; std `Mutex`
  poisoning is recovered instead of panicking; spawned PTY now uses an
  RAII cleanup guard on setup failure.

### Fixed

- `PtyCleanupGuard::as_mut` clippy `should_implement_trait` warning
  silenced.
- CI now builds `kiri-cli` before `cargo test`/`check` so the build.rs
  resource assertion does not race.

### Removed

#### Remote Access
- Removed the Remote Access feature in its entirety: built-in Axum server,
  Cloudflare Tunnel integration, QR code surface, `Cmd+Shift+R` shortcut,
  Tools → Remote Access menu, and the persisted `remoteAccess` settings.
- Reasoning: the feature broadened the attack surface (Cloudflare tunnel
  `sh -c` invocation, overly-permissive Tauri capabilities, untested auth /
  encryption seams — see #21, #23, #25) and was orthogonal to the agent
  terminal value prop. Removing it shrinks the bundle (axum + qrcode deps
  dropped) and simplifies the security review.

#### Terminal worktree tag
- Removed the linked-worktree label chip from the pane header along with
  its supporting `get_worktree_info` Tauri command, `WorktreeInfo`
  Rust/TS types, the per-pane cwd-poll loop that drove it, and the
  related design notes.

## [0.5.1] - 2026-05-17

### Fixed

#### Multi-window
- `cli:pane-*` events (`split` / `close` / `minimize` / `set-label`) emitted by the Rust CLI server now reach only the targeted window
  - Previously the JS-side listener used the global `listen()` from `@tauri-apps/api/event`, so events that Rust targeted with `emit_to(label, …)` still leaked to every open kiri window. Splitting a pane in window A would split panes in window B (and any other open windows).
  - Switched `cliBridge.ts` to `eventService.listenCurrentWindow()` (`getCurrentWindow().listen()`), so each window only receives its own events.
  - Closes #17

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
