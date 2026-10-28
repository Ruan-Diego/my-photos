//! F31: the settings screen (artboard `Settings`).

use std::path::Path;

use tauri::State;

use super::health::DbState;
use super::onboarding::IndexingState;
use crate::settings::{clear_thumbnails, set_cache_limit, set_flag, settings, Settings};
use crate::thumbs::locate_ffmpeg;

fn with_connection<T>(state: &DbState, work: impl FnOnce(&rusqlite::Connection) -> Result<T, String>) -> Result<T, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    work(connection)
}

#[tauri::command]
pub fn settings_overview(app: tauri::AppHandle, state: State<'_, DbState>) -> Result<Settings, String> {
    let ffmpeg = locate_ffmpeg().is_some();
    let mut settings = with_connection(&state, |connection| settings(connection, ffmpeg).map_err(|error| error.to_string()))?;
    settings.updater_configured = super::update::configured(app.config());
    settings.version = app.package_info().version.to_string();
    Ok(settings)
}

/// "Gerar capa dos vídeos" and "Procurar arquivos novos ao conectar o HD".
#[tauri::command]
pub fn settings_set_flag(state: State<'_, DbState>, key: String, on: bool) -> Result<(), String> {
    with_connection(&state, |connection| set_flag(connection, &key, on))
}

#[tauri::command]
pub fn settings_set_cache_limit(state: State<'_, DbState>, bytes: u64) -> Result<(), String> {
    with_connection(&state, |connection| set_cache_limit(connection, bytes))
}

/// "Limpar miniaturas", never while an indexing run is writing them. Returns the bytes freed.
#[tauri::command]
pub fn thumbnails_clear(state: State<'_, DbState>, indexing: State<'_, IndexingState>) -> Result<u64, String> {
    if indexing.is_running() {
        return Err("a indexação está rodando; espere terminar ou pause".to_string());
    }
    with_connection(&state, |connection| {
        let root: String = connection
            .query_row("SELECT root_path FROM libraries ORDER BY id LIMIT 1", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        clear_thumbnails(connection, Path::new(&root))
    })
}
