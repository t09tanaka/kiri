use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    App, Emitter, Manager,
};

pub fn setup_menu(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle();

    // App menu (macOS)
    #[cfg(target_os = "macos")]
    let app_menu = {
        let about = PredefinedMenuItem::about(handle, Some("About kiri"), None)?;
        let quit = PredefinedMenuItem::quit(handle, Some("Quit kiri"))?;
        let hide = PredefinedMenuItem::hide(handle, Some("Hide kiri"))?;
        let hide_others = PredefinedMenuItem::hide_others(handle, Some("Hide Others"))?;
        let show_all = PredefinedMenuItem::show_all(handle, Some("Show All"))?;

        Submenu::with_items(
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
        )?
    };

    // File menu items
    let new_window = MenuItem::with_id(handle, "new_window", "New Window", true, Some("CmdOrCtrl+Shift+N"))?;
    let open = MenuItem::with_id(handle, "open", "Open...", true, Some("CmdOrCtrl+O"))?;
    let close_window = PredefinedMenuItem::close_window(handle, Some("Close Window"))?;

    let file_menu = Submenu::with_items(
        handle,
        "File",
        true,
        &[
            &new_window,
            &PredefinedMenuItem::separator(handle)?,
            &open,
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

    let view_menu = Submenu::with_items(
        handle,
        "View",
        true,
        &[
            &toggle_fullscreen,
        ],
    )?;

    // Window menu
    let minimize = PredefinedMenuItem::minimize(handle, Some("Minimize"))?;

    let window_menu = Submenu::with_items(
        handle,
        "Window",
        true,
        &[
            &minimize,
        ],
    )?;

    // Build menu
    #[cfg(target_os = "macos")]
    let menu = Menu::with_items(
        handle,
        &[
            &app_menu,
            &file_menu,
            &edit_menu,
            &view_menu,
            &window_menu,
        ],
    )?;

    #[cfg(not(target_os = "macos"))]
    let menu = Menu::with_items(
        handle,
        &[
            &file_menu,
            &edit_menu,
            &view_menu,
            &window_menu,
        ],
    )?;

    app.set_menu(menu)?;

    // Handle menu events
    app.on_menu_event(move |app_handle, event| {
        match event.id().as_ref() {
            "new_window" => {
                if let Err(e) = super::create_window(app_handle.clone(), None, None, None, None, None) {
                    eprintln!("Failed to create window: {}", e);
                }
            }
            "open" => {
                // Emit event to frontend to handle open dialog
                if let Some(window) = app_handle.get_webview_window("main") {
                    let _ = window.emit("menu-open", ());
                } else if let Some(windows) = app_handle.webview_windows().values().next() {
                    let _ = windows.emit("menu-open", ());
                }
            }
            _ => {}
        }
    });

    Ok(())
}
