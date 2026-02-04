mod commands;

use commands::{
    clear_performance_timings, close_terminal, copy_files_to_worktree, copy_paths_to_directory,
    create_directory, create_terminal, create_window, create_worktree, delete_path,
    detect_package_manager, focus_or_create_window, get_all_git_diffs, get_git_diff,
    get_git_file_status, get_git_status, get_home_directory, get_memory_metrics,
    get_performance_report, get_window_geometry, get_worktree_context, is_terminal_alive,
    list_branches, list_worktrees, read_directory, read_file, record_command_timing,
    register_window, resize_terminal, remove_worktree, reveal_in_finder, run_init_command,
    search_content, search_files, set_window_geometry, setup_menu, start_watching,
    stop_all_watching, stop_watching, unregister_window, write_terminal, TerminalState,
    WatcherState, WindowRegistry, WindowRegistryState,
};
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(Arc::new(Mutex::new(commands::TerminalManager::new())) as TerminalState)
        .manage(Arc::new(Mutex::new(commands::WatcherManager::new())) as WatcherState)
        .manage(Arc::new(Mutex::new(WindowRegistry::new())) as WindowRegistryState)
        .setup(|app| {
            // Setup menu bar
            setup_menu(app)?;

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
                app.handle()
                    .plugin(tauri_plugin_mcp_bridge::init())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_directory,
            create_directory,
            get_home_directory,
            create_terminal,
            write_terminal,
            resize_terminal,
            close_terminal,
            is_terminal_alive,
            read_file,
            get_git_status,
            get_git_file_status,
            get_git_diff,
            get_all_git_diffs,
            search_files,
            search_content,
            create_window,
            focus_or_create_window,
            register_window,
            unregister_window,
            get_window_geometry,
            set_window_geometry,
            reveal_in_finder,
            delete_path,
            list_worktrees,
            create_worktree,
            remove_worktree,
            get_worktree_context,
            list_branches,
            copy_files_to_worktree,
            detect_package_manager,
            run_init_command,
            start_watching,
            stop_watching,
            stop_all_watching,
            // Performance commands (debug builds only)
            get_memory_metrics,
            get_performance_report,
            record_command_timing,
            clear_performance_timings,
            // Drag and drop
            copy_paths_to_directory,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
