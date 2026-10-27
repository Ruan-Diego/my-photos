//! F28: the problems screen (artboard `Problems`).

use std::path::Path;

use tauri::State;

use super::health::DbState;
use crate::problems::{integrity, overview, problems, report_csv, Overview};
use crate::thumbs::locate_ffmpeg;
use crate::time;

fn with_connection<T>(state: &DbState, work: impl FnOnce(&rusqlite::Connection) -> Result<T, String>) -> Result<T, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    work(connection)
}

#[tauri::command]
pub fn problems_overview(state: State<'_, DbState>) -> Result<Overview, String> {
    let ffmpeg = locate_ffmpeg().is_some();
    with_connection(&state, |connection| overview(connection, ffmpeg).map_err(|error| error.to_string()))
}

/// "Verificar integridade do banco": `None` when sound, else what SQLite complained about.
#[tauri::command]
pub fn database_check(state: State<'_, DbState>) -> Result<Option<String>, String> {
    with_connection(&state, |connection| integrity(connection).map_err(|error| error.to_string()))
}

/// "Exportar relatório": a CSV next to the database (`E:\.myphotos\relatorios\`, AD-011).
/// Returns where it was saved.
#[tauri::command]
pub fn problems_export(state: State<'_, DbState>) -> Result<String, String> {
    with_connection(&state, |connection| {
        let rows = problems(connection).map_err(|error| error.to_string())?;
        let database = connection.path().filter(|path| !path.is_empty()).ok_or("banco sem arquivo")?;
        let folder = Path::new(database).parent().ok_or("banco sem pasta")?.join("relatorios");
        std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
        let stamp: String = time::iso_utc(std::time::SystemTime::now())
            .unwrap_or_default()
            .chars()
            .filter(char::is_ascii_digit)
            .take(14)
            .collect();
        let file = folder.join(format!("problemas-{stamp}.csv"));
        std::fs::write(&file, report_csv(&rows)).map_err(|error| error.to_string())?;
        Ok(file.to_string_lossy().into_owned())
    })
}
