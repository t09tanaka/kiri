# Open Recent Menu Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add "Open Recent" submenu to File menu for quick access to recently opened projects (max 5 items + Clear Recent).

**Architecture:** Frontend-driven approach. Rust builds the native submenu from store data at startup. Frontend emits `update-recent-menu` events whenever projects change, Rust rebuilds the submenu. Menu clicks emit events back to frontend for handling.

**Tech Stack:** Tauri 2 native menu API, tauri-plugin-store (Rust + TS), Svelte stores, Tauri events

---

## Task 1: Add `clearRecentProjects` to projectStore

**Files:**
- Modify: `src/lib/stores/projectStore.ts`
- Create: `src/lib/stores/projectStore.test.ts`

**Step 1: Write the failing tests**

```typescript
// src/lib/stores/projectStore.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { get } from 'svelte/store';

// The mock store instance needs to be accessible
const mockStoreInstance = {
  get: vi.fn(),
  set: vi.fn(),
  save: vi.fn(),
  delete: vi.fn(),
  reload: vi.fn(),
};

vi.mock('@tauri-apps/plugin-store', () => ({
  Store: {
    load: vi.fn().mockResolvedValue(mockStoreInstance),
  },
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@/lib/services/windowService', () => ({
  windowService: {
    setGeometry: vi.fn().mockResolvedValue(undefined),
    setSizeAndCenter: vi.fn().mockResolvedValue(undefined),
  },
}));

vi.mock('@/lib/services/eventService', () => ({
  eventService: {
    emit: vi.fn().mockResolvedValue(undefined),
  },
}));

describe('projectStore', () => {
  beforeEach(() => {
    vi.resetModules();
    mockStoreInstance.get.mockReset();
    mockStoreInstance.set.mockReset();
    mockStoreInstance.save.mockReset();
    mockStoreInstance.reload.mockReset();
  });

  describe('clearRecentProjects', () => {
    it('should clear all recent projects', async () => {
      mockStoreInstance.get.mockResolvedValue([
        { path: '/a', name: 'a', lastOpened: 1 },
        { path: '/b', name: 'b', lastOpened: 2 },
      ]);

      const { projectStore, recentProjects } = await import('./projectStore');
      await projectStore.init();

      expect(get(recentProjects).length).toBe(2);

      await projectStore.clearRecentProjects();

      expect(get(recentProjects).length).toBe(0);
      expect(mockStoreInstance.set).toHaveBeenCalledWith('recentProjects', []);
      expect(mockStoreInstance.save).toHaveBeenCalled();
    });
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npm run test -- src/lib/stores/projectStore.test.ts`
Expected: FAIL with "clearRecentProjects is not a function" or similar

**Step 3: Implement `clearRecentProjects` and `MAX_RECENT_MENU_ITEMS`**

Add to `src/lib/stores/projectStore.ts`:

1. Add constant: `export const MAX_RECENT_MENU_ITEMS = 5;`
2. Add method to store:

```typescript
async clearRecentProjects() {
  update((state) => {
    saveRecentProjects([]);
    return {
      ...state,
      recentProjects: [],
    };
  });
},
```

**Step 4: Run test to verify it passes**

Run: `npm run test -- src/lib/stores/projectStore.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/stores/projectStore.ts src/lib/stores/projectStore.test.ts
git commit -m "feat(menu): add clearRecentProjects and MAX_RECENT_MENU_ITEMS to projectStore"
```

---

## Task 2: Add event emitting to projectStore

**Files:**
- Modify: `src/lib/stores/projectStore.ts`
- Modify: `src/lib/stores/projectStore.test.ts`

**Step 1: Write the failing test**

Add to `projectStore.test.ts`:

```typescript
import { eventService } from '@/lib/services/eventService';

describe('recent menu event emitting', () => {
  it('should emit update-recent-menu after openProject', async () => {
    mockStoreInstance.get.mockResolvedValue([]);

    const { projectStore } = await import('./projectStore');
    await projectStore.init();

    await projectStore.openProject('/test/project');

    expect(eventService.emit).toHaveBeenCalledWith(
      'update-recent-menu',
      expect.arrayContaining([
        expect.objectContaining({ path: '/test/project' }),
      ])
    );
  });

  it('should emit update-recent-menu after clearRecentProjects', async () => {
    mockStoreInstance.get.mockResolvedValue([
      { path: '/a', name: 'a', lastOpened: 1 },
    ]);

    const { projectStore } = await import('./projectStore');
    await projectStore.init();

    await projectStore.clearRecentProjects();

    expect(eventService.emit).toHaveBeenCalledWith('update-recent-menu', []);
  });

  it('should emit at most MAX_RECENT_MENU_ITEMS items', async () => {
    const projects = Array.from({ length: 10 }, (_, i) => ({
      path: `/project-${i}`,
      name: `project-${i}`,
      lastOpened: i,
    }));
    mockStoreInstance.get.mockResolvedValue(projects);

    const { projectStore, MAX_RECENT_MENU_ITEMS } = await import('./projectStore');
    await projectStore.init();

    await projectStore.openProject('/new-project');

    const emitCall = vi.mocked(eventService.emit).mock.calls.find(
      (call) => call[0] === 'update-recent-menu'
    );
    expect(emitCall).toBeDefined();
    expect((emitCall![1] as unknown[]).length).toBeLessThanOrEqual(MAX_RECENT_MENU_ITEMS);
  });
});
```

**Step 2: Run test to verify it fails**

Run: `npm run test -- src/lib/stores/projectStore.test.ts`
Expected: FAIL (eventService.emit not called)

**Step 3: Add event emitting to projectStore**

In `src/lib/stores/projectStore.ts`:

1. Import eventService: `import { eventService } from '@/lib/services/eventService';`
2. Add helper function:

```typescript
async function emitRecentMenuUpdate(projects: RecentProject[]) {
  const menuItems = projects.slice(0, MAX_RECENT_MENU_ITEMS);
  await eventService.emit('update-recent-menu', menuItems);
}
```

3. Call `emitRecentMenuUpdate(updatedProjects)` at end of:
   - `openProject()` (after `saveRecentProjects`)
   - `clearRecentProjects()`
   - `removeProject()`
   - `init()` (to populate menu at startup)

**Step 4: Run test to verify it passes**

Run: `npm run test -- src/lib/stores/projectStore.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add src/lib/stores/projectStore.ts src/lib/stores/projectStore.test.ts
git commit -m "feat(menu): emit update-recent-menu events from projectStore"
```

---

## Task 3: Rust - Add Open Recent submenu to menu.rs

**Files:**
- Modify: `src-tauri/src/commands/menu.rs`

**Step 1: Add serde structs for recent project data**

At the top of `menu.rs`, add:

```rust
use serde::Deserialize;
use std::sync::Mutex;
use tauri::Listener;

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct RecentProject {
    path: String,
    name: String,
    last_opened: f64,
    git_branch: Option<String>,
}
```

**Step 2: Read initial recent projects from store at startup**

Add helper function to read from store:

```rust
fn load_recent_projects_from_store(app: &App) -> Vec<RecentProject> {
    use tauri_plugin_store::StoreExt;

    match app.store("kiri-settings.json") {
        Ok(store) => {
            match store.get("recentProjects") {
                Some(value) => {
                    serde_json::from_value::<Vec<RecentProject>>(value)
                        .unwrap_or_default()
                }
                None => Vec::new(),
            }
        }
        Err(_) => Vec::new(),
    }
}
```

**Step 3: Build the Open Recent submenu**

Add helper function:

```rust
const MAX_RECENT_MENU_ITEMS: usize = 5;

fn build_recent_submenu(
    handle: &tauri::AppHandle,
    projects: &[RecentProject],
) -> Result<Submenu<tauri::Wry>, Box<dyn std::error::Error>> {
    let mut items: Vec<Box<dyn IsMenuItem<tauri::Wry>>> = Vec::new();

    if projects.is_empty() {
        let empty_item = MenuItem::with_id(
            handle,
            "recent_empty",
            "(No Recent Projects)",
            false, // disabled
            None::<&str>,
        )?;
        items.push(Box::new(empty_item));
    } else {
        let display_count = projects.len().min(MAX_RECENT_MENU_ITEMS);
        for (i, project) in projects.iter().take(display_count).enumerate() {
            let id = format!("recent_{}", i);
            let item = MenuItem::with_id(
                handle,
                &id,
                &project.name,
                true,
                None::<&str>,
            )?;
            items.push(Box::new(item));
        }

        items.push(Box::new(PredefinedMenuItem::separator(handle)?));

        let clear_item = MenuItem::with_id(
            handle,
            "clear_recent",
            "Clear Recent",
            true,
            None::<&str>,
        )?;
        items.push(Box::new(clear_item));
    }

    let item_refs: Vec<&dyn IsMenuItem<tauri::Wry>> = items.iter().map(|i| i.as_ref()).collect();

    Ok(Submenu::with_id_and_items(
        handle,
        "open_recent",
        "Open Recent",
        true,
        &item_refs,
    )?)
}
```

**Step 4: Integrate into setup_menu**

Add `use tauri::menu::IsMenuItem;` to imports.

In `setup_menu`, after creating `open` item and before `close_window`:

```rust
// Load initial recent projects from store
let initial_projects = load_recent_projects_from_store(app);

// Build Open Recent submenu
let open_recent = build_recent_submenu(handle, &initial_projects)?;

// Store recent projects paths for menu event handling
let recent_paths = Arc::new(Mutex::new(
    initial_projects.iter().map(|p| p.path.clone()).collect::<Vec<_>>()
));
```

Update File menu items array to include `&open_recent`:

```rust
let file_menu = Submenu::with_items(
    handle,
    "File",
    true,
    &[
        &new_window,
        &PredefinedMenuItem::separator(handle)?,
        &open,
        &open_recent,
        &PredefinedMenuItem::separator(handle)?,
        &close_window,
    ],
)?;
```

**Step 5: Handle recent menu click events**

Update `on_menu_event` to handle `recent_N` and `clear_recent`:

```rust
let recent_paths_for_events = Arc::clone(&recent_paths);

app.on_menu_event(move |app_handle, event| {
    let id = event.id().as_ref();
    match id {
        "new_window" => {
            let _ = app_handle.emit("menu-new-window", ());
        }
        "open" => {
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.emit("menu-open", ());
            } else if let Some(windows) = app_handle.webview_windows().values().next() {
                let _ = windows.emit("menu-open", ());
            }
        }
        "clear_recent" => {
            let _ = app_handle.emit("menu-clear-recent", ());
        }
        _ if id.starts_with("recent_") => {
            if let Ok(index) = id.strip_prefix("recent_").unwrap().parse::<usize>() {
                let paths = recent_paths_for_events.lock().unwrap();
                if let Some(path) = paths.get(index) {
                    let _ = app_handle.emit("menu-open-recent", path.clone());
                }
            }
        }
        _ => {}
    }
});
```

**Step 6: Listen for `update-recent-menu` event from frontend**

After `on_menu_event`, add event listener for dynamic updates:

```rust
let recent_paths_for_update = Arc::clone(&recent_paths);
let app_handle = app.handle().clone();

app.listen("update-recent-menu", move |event| {
    if let Ok(projects) = serde_json::from_str::<Vec<RecentProject>>(event.payload()) {
        // Update stored paths
        {
            let mut paths = recent_paths_for_update.lock().unwrap();
            *paths = projects.iter().map(|p| p.path.clone()).collect();
        }

        // Rebuild submenu
        if let Ok(new_submenu) = build_recent_submenu(&app_handle, &projects) {
            // Get the current menu and find the File submenu to update
            if let Some(menu) = app_handle.menu() {
                // Remove old Open Recent and insert new one
                if let Some(old_recent) = menu.get("open_recent") {
                    let _ = menu.remove(&old_recent);
                }
                // Re-insert at position (after Open...)
                let _ = menu.get("file_menu"); // This approach is complex
            }
        }
    }
});
```

**Note:** Tauri 2's menu API makes replacing a submenu within a parent submenu complex. A simpler approach is to rebuild the entire File menu. Let me revise:

The simpler approach is to **clear and rebuild the entire menu** when recent projects change. However, this can be even simpler: since `Submenu` in Tauri 2 has no `set_items` method on a nested submenu, we should use `app.set_menu()` to replace the full menu.

**Revised approach for dynamic updates:**

Add a `rebuild_menu` function that takes `AppHandle` and projects:

```rust
fn rebuild_menu(
    handle: &tauri::AppHandle,
    projects: &[RecentProject],
) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let new_window = MenuItem::with_id(handle, "new_window", "New Window", true, Some("CmdOrCtrl+Shift+N"))?;
    let open = MenuItem::with_id(handle, "open", "Open...", true, Some("CmdOrCtrl+O"))?;
    let close_window = PredefinedMenuItem::close_window(handle, Some("Close Window"))?;
    let open_recent = build_recent_submenu(handle, projects)?;

    let file_menu = Submenu::with_items(handle, "File", true, &[
        &new_window,
        &PredefinedMenuItem::separator(handle)?,
        &open,
        &open_recent,
        &PredefinedMenuItem::separator(handle)?,
        &close_window,
    ])?;

    // ... (rebuild all menus)
    // Return the complete Menu
}
```

Then in the event listener:

```rust
if let Ok(new_menu) = rebuild_menu(&app_handle, &projects) {
    let _ = app_handle.set_menu(new_menu);
}
```

**Step 7: Run compilation check**

Run: `cd src-tauri && cargo check`
Expected: PASS (no compilation errors)

**Step 8: Commit**

```bash
git add src-tauri/src/commands/menu.rs
git commit -m "feat(menu): add Open Recent submenu with dynamic updates"
```

---

## Task 4: Frontend - Handle menu events in App.svelte

**Files:**
- Modify: `src/App.svelte`

**Step 1: Add event listeners for menu-open-recent and menu-clear-recent**

In `App.svelte`, in the `onMount` block (near line 760 where other menu listeners are), add:

```typescript
// Listen for menu-open-recent event
const unlistenOpenRecent = await listen<string>('menu-open-recent', async (event) => {
  const path = event.payload;
  if (path) {
    await projectStore.openProject(path);
    // Open a default terminal tab when opening from recent (if no tabs exist)
    const { tabs } = tabStore.getStateForPersistence();
    if (tabs.length === 0) {
      tabStore.addTerminalTab();
    }
  }
});

// Listen for menu-clear-recent event
const unlistenClearRecent = await listen('menu-clear-recent', async () => {
  await projectStore.clearRecentProjects();
});
```

**Step 2: Add cleanup in the return function**

In the cleanup return function (around line 782), add:

```typescript
unlistenOpenRecent();
unlistenClearRecent();
```

**Step 3: Run type check**

Run: `npm run check`
Expected: PASS

**Step 4: Commit**

```bash
git add src/App.svelte
git commit -m "feat(menu): handle Open Recent menu events in App.svelte"
```

---

## Task 5: Integration verification

**Step 1: Run all frontend tests**

Run: `npm run test`
Expected: All PASS

**Step 2: Run all Rust tests**

Run: `npm run test:rust`
Expected: All PASS

**Step 3: Run linting**

Run: `npm run lint`
Expected: No errors

**Step 4: Run type check**

Run: `npm run check`
Expected: No errors

**Step 5: Build verification**

Run: `cd src-tauri && cargo build`
Expected: PASS

**Step 6: Commit any fixes**

If any fixes were needed, commit them.

---

## Task 6: Manual E2E verification

**Step 1: Start the app**

Run: `npm run tauri dev`

**Step 2: Verify Open Recent menu**

1. Open File menu -> verify "Open Recent" submenu exists
2. If no projects have been opened: verify "(No Recent Projects)" disabled item
3. Open a project with Cmd+O
4. Check File > Open Recent -> project should appear
5. Open more projects -> verify list updates (max 5)
6. Click a recent project -> verify it opens
7. Click "Clear Recent" -> verify list is emptied

**Step 3: Take screenshot for verification**

Use `@hypothesi/tauri-mcp-server` to take screenshot of the menu.
