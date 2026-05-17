# Platform support

kiri's primary target is macOS Apple Silicon. The Tauri host and the
`kiri-cli` binary both build and pass tests on other Unix targets, but
the desktop UI has only been smoke-tested on macOS. This document
tracks what is known to work, what is known to be broken, and what is
unverified per platform — so contributors and users can pick the right
expectation before filing a bug.

If you run kiri on a platform marked **unverified** below, please open
an issue (use the `bug` template; mark `area:platform`) describing
what worked and what did not. Concrete reports here turn into checked
boxes — or new caveats — instead of staying invisible.

## Tier 1 — supported

| Platform | Build artefact | UI smoke-tested | CLI tested |
|---|---|---|---|
| macOS Apple Silicon (M-series, macOS 13+) | `.dmg` from [Releases](https://github.com/t09tanaka/kiri/releases) | yes | yes |

Tier 1 means: every release is tested by hand against the golden path
(open a project, split a pane, run a command via the CLI, close).
Bugs filed against Tier 1 get triage priority.

## Tier 2 — builds from source, mostly works, unverified UX

| Platform | Build artefact | UI smoke-tested | CLI tested | Known gaps |
|---|---|---|---|---|
| macOS Intel (x86_64, macOS 13+) | none shipped; `cargo build --release` works | no | yes (CI runs on `ubuntu-latest`; binary verified by parity with aarch64) | Same Apple Silicon binary will not run; you must build from source. App icon and window-vibrancy effects are designed for Apple Silicon defaults and may render differently. |
| Linux x86_64 (Debian 12 / Ubuntu 22.04+) | none shipped; build from source | no | partial (CI exercises the unit-test surface, not the bundled app) | Tray icon and global shortcuts use platform-specific Tauri plugins; behaviour on X11 vs Wayland is unverified. The DEC 2026 (Synchronized Output) rendering depends on the WebKit version shipped with `libwebkit2gtk-4.1` — older distros may flicker. |

Tier 2 means: the source compiles, the tests pass in CI, but nobody on
the maintainer team uses this configuration daily. Issues are welcome;
expect slower fixes than Tier 1.

## Tier 3 — unverified

| Platform | Status |
|---|---|
| Linux ARM64 | Unverified. The `kiri-cli` crate is portable; the Tauri host should build but the bundle workflow does not currently target this triple. |
| Windows 11 (x86_64) | Unverified. Tauri targets Windows in general; kiri's terminal pipeline uses [`portable-pty`](https://docs.rs/portable-pty/latest/portable_pty/), which has a Windows backend, but the Unix domain socket transport in `crates/kiri-cli` requires a UDS shim (`interprocess` exposes a Windows named-pipe backend that should work; not tried). The kiri-side socket discovery in `~/.kiri/instances/` assumes a POSIX home directory layout. |
| WSL2 | Unverified. WSL has its own UDS namespace and `~/.kiri/instances/` would be inside the WSL filesystem, not the host's. Running the Windows-side `kiri` against a WSL kiri host is unlikely to work without changes. |

Tier 3 means: nothing has been tried. If you get any of these to
work, even partially, file the report — that's how a Tier 3 entry
becomes Tier 2.

## How to verify a platform

If you want to upgrade a Tier 3 platform to Tier 2, the smoke test is:

1. **Build.** `cargo build --release` (full app) and
   `cargo build --release -p kiri-cli` (CLI alone). Both must succeed
   without warnings.
2. **Launch.** Start the bundled app. The start screen must render
   without flicker; the keyboard shortcut hint at the bottom must be
   readable.
3. **Open a project.** `Cmd+O` / `Ctrl+O` to open a directory.
4. **Spawn a terminal pane and a CLI session.** Inside the pane, run
   `echo $KIRI_TERMINAL` (expect `1`) and `kiri env --pretty` (expect
   the configured socket alive).
5. **Split.** `kiri term split --dir v --name smoke --color iris`.
   A second pane must open and the parent pane must not close.
6. **Run a command via the CLI.** `kiri term run --pane <id> echo
   hello`. Must return `{"exit_code":0,...}` with `hello` in `output`.
7. **Close.** `kiri term close --pane <id>`. The pane must go away
   without leaking PTY processes (`ps aux | grep -v grep | grep
   ${pane-process}`).

Capture the output of `kiri env --pretty` and the OS / arch info, open
an issue, and we can move the tier up.

## CI vs runtime

CI builds Tauri on macOS only today; the `Backend Tests` job runs on
`ubuntu-latest` and exercises the Rust crates but never starts the
windowing system. That is why Linux is Tier 2 instead of Tier 1 even
though the workspace compiles cleanly there. Adding a Linux UI smoke
job is tracked in the build/CI labels.
