mod commands;
mod status_stream;

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
            if let Err(error) = status_stream::spawn_watcher(app.handle().clone()) {
                eprintln!("failed to start status watcher: {error}");
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running Git-Same");
}
