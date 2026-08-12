mod commands;
mod database;
mod domain;
mod error;
mod search;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();

    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());

    builder
        .manage(AppState::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::create_project,
            commands::open_project,
            commands::close_project,
            commands::get_project_snapshot,
            commands::update_project_settings,
            commands::query_entry_summaries,
            commands::load_entry,
            commands::create_entry,
            commands::save_entry,
            commands::delete_entry,
            commands::restore_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
