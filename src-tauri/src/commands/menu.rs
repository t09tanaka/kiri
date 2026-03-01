use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    App, Emitter, Listener, Manager,
};

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
struct RecentProject {
    path: String,
    name: String,
    #[allow(dead_code)]
    last_opened: f64,
    #[allow(dead_code)]
    git_branch: Option<String>,
}

const MAX_RECENT_MENU_ITEMS: usize = 5;

/// Emit an event to the focused window only, falling back to main window.
/// Prevents duplicate event handling in multi-window scenarios.
fn emit_to_focused_window<S: serde::Serialize + Clone>(
    app_handle: &tauri::AppHandle,
    event: &str,
    payload: S,
) -> Result<(), tauri::Error> {
    let windows = app_handle.webview_windows();
    let target = windows
        .values()
        .find(|w| w.is_focused().unwrap_or(false))
        .or_else(|| windows.get("main"));
    if let Some(window) = target {
        window.emit(event, payload)
    } else {
        app_handle.emit(event, payload)
    }
}

struct ToolsState {
    remote_access_on: bool,
    startup_command: String,
}

fn load_recent_projects_from_store(app: &App) -> Vec<RecentProject> {
    use tauri_plugin_store::StoreExt;
    match app.store("kiri-settings.json") {
        Ok(store) => match store.get("recentProjects") {
            Some(value) => serde_json::from_value::<Vec<RecentProject>>(value).unwrap_or_default(),
            None => Vec::new(),
        },
        Err(_) => Vec::new(),
    }
}

fn load_startup_command_from_store(app: &App) -> String {
    use tauri_plugin_store::StoreExt;
    match app.store("kiri-settings.json") {
        Ok(store) => {
            if let Some(value) = store.get("globalSettings") {
                value
                    .get("startupCommand")
                    .and_then(|v| v.as_str())
                    .unwrap_or("none")
                    .to_string()
            } else {
                "none".to_string()
            }
        }
        Err(_) => "none".to_string(),
    }
}

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
            false,
            None::<&str>,
        )?;
        items.push(Box::new(empty_item));
    } else {
        let display_count = projects.len().min(MAX_RECENT_MENU_ITEMS);
        for (i, project) in projects.iter().take(display_count).enumerate() {
            let id = format!("recent_{}", i);
            let item = MenuItem::with_id(handle, &id, &project.name, true, None::<&str>)?;
            items.push(Box::new(item));
        }
        items.push(Box::new(PredefinedMenuItem::separator(handle)?));
        let clear_item =
            MenuItem::with_id(handle, "clear_recent", "Clear Recent", true, None::<&str>)?;
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

fn rebuild_menu(
    handle: &tauri::AppHandle,
    projects: &[RecentProject],
    tools: &ToolsState,
) -> Result<Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let new_window = MenuItem::with_id(
        handle,
        "new_window",
        "New Window",
        true,
        Some("CmdOrCtrl+Shift+N"),
    )?;
    let open = MenuItem::with_id(handle, "open", "Open...", true, Some("CmdOrCtrl+O"))?;
    let close_window = PredefinedMenuItem::close_window(handle, Some("Close Window"))?;
    let open_recent = build_recent_submenu(handle, projects)?;

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

    // Edit menu
    let undo = PredefinedMenuItem::undo(handle, Some("Undo"))?;
    let redo = PredefinedMenuItem::redo(handle, Some("Redo"))?;
    let cut = PredefinedMenuItem::cut(handle, Some("Cut"))?;
    let copy = PredefinedMenuItem::copy(handle, Some("Copy"))?;
    let paste = PredefinedMenuItem::paste(handle, Some("Paste"))?;
    let select_all = PredefinedMenuItem::select_all(handle, Some("Select All"))?;

    let edit_menu = Submenu::with_items(
        handle,
        "Edit",
        true,
        &[
            &undo,
            &redo,
            &PredefinedMenuItem::separator(handle)?,
            &cut,
            &copy,
            &paste,
            &PredefinedMenuItem::separator(handle)?,
            &select_all,
        ],
    )?;

    // View menu
    let toggle_fullscreen = PredefinedMenuItem::fullscreen(handle, Some("Toggle Full Screen"))?;

    let view_menu = Submenu::with_items(handle, "View", true, &[&toggle_fullscreen])?;

    // Tools menu
    let remote_enabled = CheckMenuItem::with_id(
        handle,
        "toggle_remote_access",
        "Enabled",
        true,
        tools.remote_access_on,
        None::<&str>,
    )?;
    let show_qr = MenuItem::with_id(
        handle,
        "show_qr_code",
        "Show QR Code",
        tools.remote_access_on, // only enabled when remote is on
        None::<&str>,
    )?;

    let remote_submenu = Submenu::with_id_and_items(
        handle,
        "remote_access",
        "Remote Access",
        true,
        &[&remote_enabled, &show_qr],
    )?;

    let cmd_none = CheckMenuItem::with_id(
        handle,
        "startup_cmd_none",
        "None",
        true,
        tools.startup_command == "none",
        None::<&str>,
    )?;
    let cmd_claude = CheckMenuItem::with_id(
        handle,
        "startup_cmd_claude",
        "Claude",
        true,
        tools.startup_command == "claude",
        None::<&str>,
    )?;
    let cmd_codex = CheckMenuItem::with_id(
        handle,
        "startup_cmd_codex",
        "Codex",
        true,
        tools.startup_command == "codex",
        None::<&str>,
    )?;

    let startup_submenu = Submenu::with_id_and_items(
        handle,
        "startup_command",
        "Startup Command",
        true,
        &[&cmd_none, &cmd_claude, &cmd_codex],
    )?;

    let tools_menu = Submenu::with_items(
        handle,
        "Tools",
        true,
        &[
            &remote_submenu,
            &startup_submenu,
        ],
    )?;

    // Window menu
    let minimize = PredefinedMenuItem::minimize(handle, Some("Minimize"))?;

    let window_menu = Submenu::with_items(handle, "Window", true, &[&minimize])?;

    // macOS app menu
    #[cfg(target_os = "macos")]
    {
        let about = PredefinedMenuItem::about(handle, Some("About kiri"), None)?;
        let quit = PredefinedMenuItem::quit(handle, Some("Quit kiri"))?;
        let hide = PredefinedMenuItem::hide(handle, Some("Hide kiri"))?;
        let hide_others = PredefinedMenuItem::hide_others(handle, Some("Hide Others"))?;
        let show_all = PredefinedMenuItem::show_all(handle, Some("Show All"))?;
        let app_menu = Submenu::with_items(
            handle,
            "kiri",
            true,
            &[
                &about,
                &PredefinedMenuItem::separator(handle)?,
                &hide,
                &hide_others,
                &show_all,
                &PredefinedMenuItem::separator(handle)?,
                &quit,
            ],
        )?;
        Ok(Menu::with_items(
            handle,
            &[
                &app_menu, &file_menu, &edit_menu, &view_menu, &tools_menu, &window_menu,
            ],
        )?)
    }

    #[cfg(not(target_os = "macos"))]
    Ok(Menu::with_items(
        handle,
        &[
            &file_menu, &edit_menu, &view_menu, &tools_menu, &window_menu,
        ],
    )?)
}

pub fn setup_menu(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle();

    // Load initial state
    let initial_projects = load_recent_projects_from_store(app);
    let initial_startup_cmd = load_startup_command_from_store(app);

    // Shared state
    let recent_paths: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(
        initial_projects.iter().map(|p| p.path.clone()).collect(),
    ));
    let recent_projects_state: Arc<Mutex<Vec<RecentProject>>> =
        Arc::new(Mutex::new(initial_projects.clone()));
    let tools_state: Arc<Mutex<ToolsState>> = Arc::new(Mutex::new(ToolsState {
        remote_access_on: false,
        startup_command: initial_startup_cmd,
    }));

    // Build initial menu
    {
        let tools = tools_state.lock().unwrap();
        let menu = rebuild_menu(handle, &initial_projects, &tools)?;
        app.set_menu(menu)?;
    }

    // Handle menu events
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
            "toggle_remote_access" => {
                // Emit to focused window only to avoid duplicate toggles in multi-window
                let _ = emit_to_focused_window(app_handle, "menu-toggle-remote", ());
            }
            "show_qr_code" => {
                let _ = emit_to_focused_window(app_handle, "menu-show-qr-code", ());
            }
            "startup_cmd_none" | "startup_cmd_claude" | "startup_cmd_codex" => {
                let cmd = id.strip_prefix("startup_cmd_").unwrap().to_string();
                // Startup command is idempotent, safe to broadcast
                let _ = app_handle.emit("menu-set-startup-command", cmd);
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

    // Listen for update-recent-menu events from frontend
    {
        let recent_paths = Arc::clone(&recent_paths);
        let recent_projects = Arc::clone(&recent_projects_state);
        let tools = Arc::clone(&tools_state);
        let handle = app.handle().clone();
        app.listen("update-recent-menu", move |event| {
            if let Ok(projects) = serde_json::from_str::<Vec<RecentProject>>(event.payload()) {
                {
                    let mut paths = recent_paths.lock().unwrap();
                    *paths = projects.iter().map(|p| p.path.clone()).collect();
                }
                {
                    let mut stored = recent_projects.lock().unwrap();
                    *stored = projects.clone();
                }
                let tools = tools.lock().unwrap();
                if let Ok(new_menu) = rebuild_menu(&handle, &projects, &tools) {
                    let _ = handle.set_menu(new_menu);
                }
            }
        });
    }

    // Listen for tools state updates from frontend
    {
        let recent_projects = Arc::clone(&recent_projects_state);
        let tools = Arc::clone(&tools_state);
        let handle = app.handle().clone();
        app.listen("update-tools-menu", move |event| {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct ToolsUpdate {
                remote_access_on: Option<bool>,
                startup_command: Option<String>,
            }
            if let Ok(update) = serde_json::from_str::<ToolsUpdate>(event.payload()) {
                {
                    let mut t = tools.lock().unwrap();
                    if let Some(on) = update.remote_access_on {
                        t.remote_access_on = on;
                    }
                    if let Some(cmd) = update.startup_command {
                        t.startup_command = cmd;
                    }
                }
                let projects = recent_projects.lock().unwrap();
                let tools = tools.lock().unwrap();
                if let Ok(new_menu) = rebuild_menu(&handle, &projects, &tools) {
                    let _ = handle.set_menu(new_menu);
                }
            }
        });
    }

    Ok(())
}
