mod commands;
mod status_stream;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::list_workspaces,
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
