mod commands;
mod status_stream;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_workspaces,
            commands::read_app_config,
            commands::save_app_config,
            commands::ensure_config,
            commands::read_workspace,
            commands::save_workspace,
            commands::delete_workspace,
            commands::set_default_workspace,
            commands::check_requirements,
            commands::monitor_launch_agent_status,
            commands::install_monitor_launch_agent,
            commands::restart_monitor_launch_agent,
            commands::discover_provider_orgs,
            commands::read_workspace_structure,
            commands::read_status,
            commands::start_sync,
            commands::extension_status,
            commands::open_url,
        ])
        .setup(|app| {
            // Resolve the host-facing IPC config once and share it with every
            // command handler via state, so handlers read the mirrored
            // status.json from the host's own home rather than reaching into the
            // app-group container (which triggers the "access data from other
            // apps" TCC prompt).
            let host_ipc = git_same_core::ipc::IpcConfig::host_status_path()?;
            app.manage(commands::HostIpc(host_ipc.clone()));

            // A leftover symlink at the host status.json path means an old
            // monitor build is still running (only pre-upgrade monitors symlink
            // it into the container; the current monitor writes a real mirror
            // file). Best-effort restart the installed monitor so the upgraded
            // build takes over and starts mirroring, instead of the app showing
            // stale status until the user restarts it by hand. symlink_metadata
            // does not follow the link, so this never reaches into the app-group
            // container (no "access data from other apps" TCC prompt). Run on a
            // background thread so the synchronous launchctl calls do not block
            // app startup.
            let host_status_is_symlink = host_ipc
                .status_file_path()
                .symlink_metadata()
                .map(|meta| meta.file_type().is_symlink())
                .unwrap_or(false);
            if host_status_is_symlink {
                std::thread::spawn(|| {
                    if let Err(error) = commands::restart_monitor_if_installed() {
                        eprintln!("failed to restart monitor after upgrade: {error}");
                    }
                });
            }

            if let Err(error) = status_stream::spawn_watcher(app.handle().clone(), host_ipc) {
                eprintln!("failed to start status watcher: {error}");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Git-Same");
}
