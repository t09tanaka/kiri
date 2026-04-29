pub mod commands;

use commands::{
    check_gh_cli, clear_performance_timings, close_terminal,
    get_foreground_process_name, get_terminal_cwd, get_terminal_process_info,
    copy_paths_to_directory, create_directory, move_path,
    create_terminal, create_window, delete_path, fetch_remote,
    focus_or_create_window, generate_remote_qr_code, get_all_git_diffs, get_behind_ahead_count,
    get_branch_ahead_count, get_commit_diff, get_commit_log, get_git_diff, get_git_file_status,
    get_git_status, get_home_directory, get_memory_metrics, get_performance_report, get_worktree_info,
    is_terminal_alive, list_pull_requests, pull_commits,
    push_commits, read_directory, read_file, read_file_as_base64, record_command_timing,
    regenerate_remote_token, register_window, resize_terminal, reveal_in_finder,
    get_pull_request_detail, search_content, search_files, setup_menu, start_watching, stop_all_watching,
    is_cloudflared_available, start_cloudflare_tunnel, start_remote_server,
    stop_cloudflare_tunnel, stop_remote_server, is_remote_server_running,
    stop_watching, unregister_window, write_terminal, RemoteServerState, RemoteServerStateType,
    TerminalState, TunnelState, TunnelStateType, WatcherState, WindowRegistry, WindowRegistryState,
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
        .manage(Arc::new(tokio::sync::Mutex::new(RemoteServerState::new())) as RemoteServerStateType)
        .manage(Arc::new(tokio::sync::Mutex::new(TunnelState::new())) as TunnelStateType)
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

            // Install the kiri-cli binary into ~/.kiri/bin so that PTYs
            // spawned with that dir on PATH can invoke it as `kiri`.
            // Best-effort: we log and continue if it fails so that a
            // missing/broken cli does not prevent the app from launching.
            if let Err(e) = commands::cli_install::ensure_installed(app.handle()) {
                log::warn!("failed to install kiri CLI: {e}");
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
            get_foreground_process_name,
            get_terminal_process_info,
            get_terminal_cwd,
            read_file,
            read_file_as_base64,
            get_git_status,
            get_git_file_status,
            get_git_diff,
            get_all_git_diffs,
            get_worktree_info,
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
            // Remote access
            start_remote_server,
            stop_remote_server,
            is_remote_server_running,
            generate_remote_qr_code,
            regenerate_remote_token,
            // Cloudflare Tunnel
            is_cloudflared_available,
            start_cloudflare_tunnel,
            stop_cloudflare_tunnel,
            // GitHub PR
            check_gh_cli,
            list_pull_requests,
            get_pull_request_detail,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
