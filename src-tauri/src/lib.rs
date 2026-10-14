pub mod bench;
mod commands;
mod db;
pub mod scan;
mod time;
mod photo_meta;
mod resolve;
pub mod volume;
pub mod index;
mod takeout;
pub mod thumbs;

use std::sync::Mutex;

use tauri::{Manager, RunEvent};

use commands::health::DbState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            app.manage(DbState(db::open_library().map(|c| Mutex::new(Some(c)))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![commands::health::app_health])
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    app.run(|handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = handle.try_state::<DbState>() {
                state.shutdown();
            }
        }
    });
}
