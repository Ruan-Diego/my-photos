pub mod bench;
mod commands;
mod db;
mod scan;
mod takeout;

use std::sync::Mutex;

use tauri::Manager;

use commands::health::DbState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(DbState(db::open_library().map(Mutex::new)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::health::app_health])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
