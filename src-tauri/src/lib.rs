mod commands;

use commands::{
    close_terminal, create_terminal, create_window, get_all_git_diffs, get_git_diff,
    get_git_file_status, get_git_status, get_home_directory, read_directory, read_file,
    resize_terminal, reveal_in_finder, search_content, search_files, setup_menu, write_file,
    write_terminal, TerminalState,
};
use std::sync::{Arc, Mutex};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(Arc::new(Mutex::new(commands::TerminalManager::new())) as TerminalState)
        .setup(|app| {
            // Setup menu bar
            setup_menu(app)?;

            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            read_directory,
            get_home_directory,
            create_terminal,
            write_terminal,
            resize_terminal,
            close_terminal,
            read_file,
            write_file,
            get_git_status,
            get_git_file_status,
            get_git_diff,
            get_all_git_diffs,
            search_files,
            search_content,
            create_window,
            reveal_in_finder,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
