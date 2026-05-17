# Tauri Capabilities Policy

This document describes the capability layout in this directory
(`src-tauri/capabilities/`) and the rationale for each grant. It exists so
that adding a new plugin or permission requires an intentional, reviewable
change.

> **Why here?** `docs/` is gitignored (see `.gitignore`); the policy lives
> next to the JSON it documents so the two cannot drift unnoticed.

## Layout

The Tauri runtime merges every JSON file under `src-tauri/capabilities/`. We
split that surface by **feature** rather than putting everything in one file,
so a security-sensitive reviewer can read a single file and understand the
permissions that feature contributes:

| File | Identifier | Windows | Purpose |
| --- | --- | --- | --- |
| `default.json` | `default` | `*` | Base permissions every window needs (core, dialog, store, opener). |
| `notifications.json` | `notifications` | `*` | OS notifications via `tauri-plugin-notification`. Split out so the scope can be tightened later without touching the base file. |
| `mcp-bridge.json` | `mcp-bridge` | `main` | Debug-only MCP bridge plugin. Scoped to the `main` window because the bridge exposes IPC primitives that should not be reachable from auxiliary webviews. |

## Per-permission rationale

### `default.json`

- `core:default` — bundles the **read-only** defaults for `core:app`,
  `core:event`, `core:image`, `core:menu`, `core:path`, `core:resources`,
  `core:tray`, `core:webview`, and `core:window`. No destructive operations
  are included; see the generated `desktop-schema.json` for the exact list.
- `core:window:allow-close`, `core:window:allow-destroy`,
  `core:window:allow-set-title` — destructive window operations that
  `core:window:default` deliberately omits. The app needs them for the
  multi-window UX (closing/destroying secondary windows, updating titles
  when projects are opened).
- `dialog:default` — file/folder pickers and message dialogs. The included
  permissions are `allow-message`, `allow-save`, `allow-open`. We do
  **not** add the deprecated `allow-ask` alias.
- `store:default` — read/write to the persistent key/value store used by
  `persistenceService.ts`.
- `opener:default` — open URLs / paths in the OS default handler. Used by
  `openerService.ts` for "Open in Browser" actions and link clicks in the
  editor.

### `notifications.json`

- `notification:default` — show OS notifications via the notification
  plugin. Scoped to all windows today; if multi-window UX ever stops using
  notifications outside the main window, narrow `windows` here.

### `mcp-bridge.json`

- `mcp-bridge:default` — the MCP bridge plugin is only `init()`-ed in
  `cfg!(debug_assertions)` builds (see `src-tauri/src/lib.rs`). The
  capability is still declared at build time so the schema validates, but
  the plugin will not respond in release. Scoping the window list to
  `main` ensures that, if a future feature opens an auxiliary webview, it
  cannot reach the MCP bridge.

## Adding a new permission

1. Identify the **smallest** permission that satisfies the use case
   (avoid `:default` when a single `allow-*` would do).
2. Pick the file that matches the feature, or create a new
   `<feature>.json` if it doesn't fit anywhere.
3. Set `windows` as narrowly as possible. Use `["main"]` for
   server/sensitive features; only fall back to `["*"]` when the
   capability is needed by webviews created via `create_window`.
4. Update this document with the rationale.
5. Run `npm run check` and `cargo check --offline` to make sure the
   schema accepts the change.

## What we deliberately do **not** grant

- Filesystem plugins (`tauri-plugin-fs`) — all FS access goes through
  audited `#[tauri::command]` handlers (`read_directory`, `read_file`,
  `delete_path`, ...) so we do not enable raw scoped FS permissions from
  the frontend.
- HTTP plugin (`tauri-plugin-http`) — not loaded.
- Shell plugin (`tauri-plugin-shell`) — not loaded. The terminal feature
  spawns PTYs through `terminal_commands.rs`, not the shell plugin.
- `dialog:allow-ask` — deprecated alias for `allow-message`, which is
  already granted via `dialog:default`.
- `dialog:allow-open` (explicit) — redundant; `dialog:default` includes
  it.
