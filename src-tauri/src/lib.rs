pub mod commands;

use commands::{
    cleanup_window_resources, clear_performance_timings, cli_resolve_pending, cli_update_pane_map,
    close_terminal,
    get_foreground_process_name, get_terminal_cwd, get_terminal_process_info,
    copy_paths_to_directory, create_directory, create_file, move_path, move_to_trash,
    open_terminal_here, rename_path, restore_from_trash, trash_restore_supported,
    create_terminal, create_window, delete_path, fetch_remote,
    focus_or_create_window, get_all_git_diffs, get_behind_ahead_count,
    get_branch_ahead_count, get_commit_diff, get_commit_log, get_git_diff, get_git_file_status,
    get_git_status, get_home_directory, get_memory_metrics, get_performance_report,
    install_kiri_skill, is_terminal_alive, kiri_skill_status, pull_commits,
    push_commits, read_directory, read_file, read_file_as_base64, record_command_timing,
    register_window, resize_terminal, reveal_in_finder,
    search_content, search_files, setup_menu, start_watching, stop_all_watching,
    stop_watching, unregister_window, write_terminal, CliServerRegistry, CliServerRegistryState,
    TerminalOutputBus, TerminalOutputBusState, TerminalState,
    WatcherState, WindowRegistry, WindowRegistryState,
};
use std::sync::{Arc, Mutex};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .manage(Arc::new(Mutex::new(commands::TerminalManager::new())) as TerminalState)
        .manage(Arc::new(TerminalOutputBus::new()) as TerminalOutputBusState)
        .manage(Arc::new(CliServerRegistry::new()) as CliServerRegistryState)
        .manage(Arc::new(Mutex::new(commands::WatcherManager::new())) as WatcherState)
        .manage(Arc::new(Mutex::new(WindowRegistry::new())) as WindowRegistryState)
        .setup(|app| {
            // Sweep socket files left behind by a previous session that
            // crashed or was force-quit before its exit cleanup ran. Only
            // dead sockets are removed, so a concurrently running kiri
            // instance is unaffected. Done before any window registers.
            tauri::async_runtime::block_on(commands::cli_server::sweep_dead_sockets());

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

            // Install the kiri-cli binary into ~/.kiri/bin so that PTYs
            // spawned with that dir on PATH can invoke it as `kiri`.
            // Best-effort: we log and continue if it fails so that a
            // missing/broken cli does not prevent the app from launching.
            if let Err(e) = commands::cli_install::ensure_installed(app.handle()) {
                log::warn!("failed to install kiri CLI: {e}");
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            // Authoritative cleanup keyed on the real window lifecycle. The
            // frontend's onCloseRequested also unregisters, but it can be
            // skipped or interrupted (crash, forced close, app quit). Doing
            // it here too guarantees a destroyed window never leaves its CLI
            // server (and socket) running as a ghost the `kiri` CLI would
            // still list. Idempotent, so the double-call is harmless.
            if let tauri::WindowEvent::Destroyed = event {
                let app = window.app_handle();
                let label = window.label().to_string();
                let registry = app.state::<WindowRegistryState>();
                let cli_registry = app.state::<CliServerRegistryState>();
                cleanup_window_resources(&registry, &cli_registry, &label);
            }
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
            get_foreground_process_name,
            get_terminal_process_info,
            get_terminal_cwd,
            read_file,
            read_file_as_base64,
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
            reveal_in_finder,
            delete_path,
            start_watching,
            stop_watching,
            stop_all_watching,
            // Performance commands (debug builds only)
            get_memory_metrics,
            get_performance_report,
            record_command_timing,
            clear_performance_timings,
            // Core file operations (#82, #84, #90)
            rename_path,
            create_file,
            move_to_trash,
            restore_from_trash,
            trash_restore_supported,
            open_terminal_here,
            // Drag and drop
            copy_paths_to_directory,
            move_path,
            // Git history
            get_commit_log,
            get_commit_diff,
            push_commits,
            fetch_remote,
            get_behind_ahead_count,
            get_branch_ahead_count,
            pull_commits,
            // CLI server (per-window socket)
            cli_resolve_pending,
            cli_update_pane_map,
            // Skill install (manual; frontend gates with confirmation dialog)
            kiri_skill_status,
            install_kiri_skill,
        ])
        .build(tauri::generate_context!())
        .unwrap_or_else(|err| {
            // Print to stderr (so it lands in stderr-capture logs) AND log
            // via the configured logger before exiting non-zero. Panicking
            // here would lose the structured logger context.
            eprintln!("fatal: kiri tauri runtime failed: {err}");
            log::error!("fatal: kiri tauri runtime failed: {err}");
            std::process::exit(1);
        })
        .run(|app_handle, event| {
            // On app exit the Tokio runtime is torn down, so the per-window
            // listener tasks may never run their own socket cleanup. Stop and
            // remove every CLI server here so quitting leaves no stale sockets
            // behind for the next launch to mistake for live windows.
            if let tauri::RunEvent::Exit = event {
                let cli_registry = app_handle.state::<CliServerRegistryState>();
                cli_registry.stop_all();
            }
        });
}
