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
            if let Err(error) = status_stream::spawn_watcher(app.handle().clone(), host_ipc) {
                eprintln!("failed to start status watcher: {error}");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Git-Same");
}
