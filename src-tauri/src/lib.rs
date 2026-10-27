pub mod bench;
mod commands;
mod db;
pub mod scan;
mod time;
mod photo_meta;
mod resolve;
pub mod search;
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
            app.manage(commands::onboarding::IndexingState::default());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::health::app_health,
            commands::library::library_summary,
            commands::onboarding::library_folder,
            commands::onboarding::start_indexing,
            commands::onboarding::pause_indexing,
            commands::onboarding::recent_thumbnails,
            commands::timeline::timeline,
            commands::viewer::media_details,
            commands::details::media_panel_details,
            commands::search::search_media,
            commands::search::search_filters
        ])
        .register_asynchronous_uri_scheme_protocol(
            commands::timeline::THUMB_SCHEME,
            commands::timeline::serve_thumbnail,
        )
        .register_asynchronous_uri_scheme_protocol(commands::viewer::MEDIA_SCHEME, commands::viewer::serve_original)
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
