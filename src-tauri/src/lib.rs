mod commands;
mod database;
mod domain;
mod error;
mod export;
mod font_manager;
mod ordering;
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
            commands::list_sense_images,
            commands::attach_sense_image,
            commands::load_sense_image,
            commands::remove_sense_image,
            commands::delete_entry,
            commands::restore_entry,
            commands::save_export_settings,
            commands::save_entry_sort_settings,
            commands::save_manual_sort_layout,
            commands::preview_export,
            commands::export_project,
            commands::detect_xelatex,
            commands::list_font_packs,
            commands::install_font_pack,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
