# Multiple Windows Design

## Overview

Allow opening multiple instances of the application as independent windows.

## Requirements

1. Open new window from menu or keyboard shortcut (Cmd+Shift+N)
2. Each window is independent (own tabs, own file tree root)
3. Windows share the same backend process

## Architecture

### Tauri Multi-Window

Tauri supports multiple windows natively. Each window:
- Has its own WebView instance
- Shares the same Rust backend
- Can communicate through events

### Implementation Approach

1. **Window Creation**: Use `tauri::WebviewWindowBuilder` to create new windows
2. **State Isolation**: Each window has its own frontend state (Svelte stores)
3. **Event Communication**: Optional cross-window communication via Tauri events

## Backend (Rust)

```rust
#[tauri::command]
fn create_window(app: AppHandle) -> Result<(), String> {
    let label = format!("window-{}", uuid());
    WebviewWindowBuilder::new(&app, label, WebviewUrl::default())
        .title("Kiri")
        .inner_size(1200.0, 800.0)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}
```

## Frontend

### Keyboard Shortcut

Add Cmd+Shift+N handling in App.svelte to invoke `create_window` command.

### Menu Integration (Future)

Later, add native menu bar with "New Window" option.

## File Structure

```
src-tauri/
└── src/
    └── commands/
        └── window.rs

src/
└── App.svelte  (updated with shortcut)
```

## Implementation Steps

1. Create window command in Rust
2. Add Cmd+Shift+N shortcut in frontend
3. Test multi-window behavior

## Acceptance Criteria

- [ ] Cmd+Shift+N opens new window
- [ ] New window is independent
- [ ] Each window can have different tabs
- [ ] Closing one window doesn't affect others
