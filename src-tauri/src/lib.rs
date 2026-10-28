pub mod albums;
pub mod backup;
pub mod favorites;
pub mod fileops;
pub mod bench;
pub mod duplicates;
mod commands;
mod db;
pub mod scan;
mod time;
mod photo_meta;
#[cfg(test)]
mod perf;
pub mod places;
pub mod problems;
mod resolve;
pub mod search;
pub mod settings;
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
            let mut library = db::open_library();
            if let Ok(connection) = &mut library {
                // Nothing indexes yet: a run still `running` was cut off by a close.
                let now = time::iso_utc(std::time::SystemTime::now()).unwrap_or_default();
                let _ = scan::close_orphan_runs(connection, &now);
                // F24: places for photos indexed before place names existed.
                let _ = places::fill_place_labels(connection);
                // F26: Google Photos favorites of items indexed before favorites existed.
                let _ = favorites::import_takeout_favorites(connection);
                // F48: what has been in the app's trash for 30 days goes for good (AD-009).
                let root: Option<String> =
                    connection.query_row("SELECT root_path FROM libraries ORDER BY id LIMIT 1", [], |row| row.get(0)).ok();
                if let Some(root) = root {
                    let _ = fileops::purge_expired(connection, std::path::Path::new(&root), &now);
                }
                // F35: a copy of the database once a day, the newest seven kept. On its own
                // connection and thread: copying the database must not hold up the window.
                if let (Some(folder), Some(path)) = (backup::folder(connection), connection.path().map(str::to_owned)) {
                    let now = now.clone();
                    std::thread::spawn(move || {
                        if let Ok(copy) = rusqlite::Connection::open(&path) {
                            let _ = backup::auto_backup(&copy, &folder, std::time::SystemTime::now(), &now);
                        }
                    });
                }
            }
            app.manage(DbState(library.map(|c| Mutex::new(Some(c)))));
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
            commands::search::search_filters,
            commands::people::people_list,
            commands::locations::locations_overview,
            commands::albums::albums_list,
            commands::favorites::favorite_set,
            commands::problems::problems_overview,
            commands::problems::database_check,
            commands::problems::problems_export,
            commands::duplicates::duplicates_list,
            commands::settings::settings_overview,
            commands::settings::settings_set_flag,
            commands::settings::settings_set_cache_limit,
            commands::settings::thumbnails_clear,
            commands::fileops::folder_tree,
            commands::fileops::folder_items,
            commands::fileops::folder_create,
            commands::fileops::folder_rename,
            commands::fileops::folder_trash,
            commands::fileops::media_move,
            commands::fileops::media_trash,
            commands::fileops::trash_list,
            commands::fileops::trash_restore,
            commands::fileops::trash_restore_all,
            commands::backup::backup_overview,
            commands::backup::backup_now,
            commands::backup::appdata_export,
            commands::backup::appdata_import
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
