use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tauri::{
    menu::{IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
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
            &[&app_menu, &file_menu, &edit_menu, &view_menu, &window_menu],
        )?)
    }

    #[cfg(not(target_os = "macos"))]
    Ok(Menu::with_items(
        handle,
        &[&file_menu, &edit_menu, &view_menu, &window_menu],
    )?)
}

pub fn setup_menu(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle();

    // Load initial recent projects from store
    let initial_projects = load_recent_projects_from_store(app);

    // Store paths for menu event handling
    let recent_paths: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(
        initial_projects.iter().map(|p| p.path.clone()).collect(),
    ));

    // Build initial menu with Open Recent submenu
    let menu = rebuild_menu(handle, &initial_projects)?;
    app.set_menu(menu)?;

    // Handle menu events
    let recent_paths_for_events = Arc::clone(&recent_paths);
    app.on_menu_event(move |app_handle, event| {
        let id = event.id().as_ref();
        match id {
            "new_window" => {
                // Emit to frontend so main window can assign proper windowIndex
                let _ = app_handle.emit("menu-new-window", ());
            }
            "open" => {
                // Emit event to frontend to handle open dialog
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

    // Listen for update-recent-menu events from frontend
    let recent_paths_for_update = Arc::clone(&recent_paths);
    let app_handle = app.handle().clone();
    app.listen("update-recent-menu", move |event| {
        if let Ok(projects) = serde_json::from_str::<Vec<RecentProject>>(event.payload()) {
            // Update stored paths
            {
                let mut paths = recent_paths_for_update.lock().unwrap();
                *paths = projects.iter().map(|p| p.path.clone()).collect();
            }
            // Rebuild entire menu
            if let Ok(new_menu) = rebuild_menu(&app_handle, &projects) {
                let _ = app_handle.set_menu(new_menu);
            }
        }
    });

    Ok(())
}
