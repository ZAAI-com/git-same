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
            commands::monitor_status,
            commands::start_monitor,
            commands::stop_monitor,
            commands::restart_monitor,
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
        .manage(commands::MonitorStatusCache::default())
        .setup(|app| {
            if let Err(error) = status_stream::spawn_watcher(app.handle().clone()) {
                eprintln!("failed to start status watcher: {error}");
            }
            // Recover monitoring in the background; the window stays responsive.
            commands::ensure_monitor_on_startup(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // A Stop or Start issued from a terminal while no monitor runs
            // touches no watched file: re-inspect when the user comes back.
            if let tauri::WindowEvent::Focused(true) = event {
                use tauri::Manager;
                commands::refresh_monitor_status(window.app_handle().clone());
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Git-Same");
}
