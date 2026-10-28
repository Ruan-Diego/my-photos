//! F35: database copies and the favorites/notes export (Configurações › Backup).

use std::path::PathBuf;

use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

use super::health::DbState;
use super::onboarding::IndexingState;
use crate::backup::{self, BackupFile, ImportOutcome};
use crate::time;

fn now() -> String {
    time::iso_utc(std::time::SystemTime::now()).unwrap_or_default()
}

fn with<T>(state: &DbState, work: impl FnOnce(&mut Connection, PathBuf) -> Result<T, String>) -> Result<T, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let mut guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_mut().ok_or("biblioteca fechada")?;
    let folder = backup::folder(connection).ok_or("banco sem arquivo")?;
    work(connection, folder)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupOverview {
    /// Where the copies are, to show and to open.
    pub folder: String,
    pub files: Vec<BackupFile>,
    pub keep: usize,
}

#[tauri::command]
pub fn backup_overview(state: State<'_, DbState>) -> Result<BackupOverview, String> {
    with(&state, |_, folder| {
        Ok(BackupOverview { files: backup::list(&folder), folder: folder.to_string_lossy().into_owned(), keep: backup::KEEP })
    })
}

#[tauri::command]
pub fn backup_now(state: State<'_, DbState>) -> Result<String, String> {
    with(&state, |connection, folder| {
        backup::backup_now(connection, &folder, &now()).map(|file| file.to_string_lossy().into_owned())
    })
}

#[tauri::command]
pub fn appdata_export(state: State<'_, DbState>) -> Result<String, String> {
    with(&state, |connection, folder| {
        backup::write_export(connection, &folder, &now()).map(|file| file.to_string_lossy().into_owned())
    })
}

/// `name`: a `dados-*.json` in the backups folder, never a path.
#[tauri::command]
pub fn appdata_import(state: State<'_, DbState>, indexing: State<'_, IndexingState>, name: String) -> Result<ImportOutcome, String> {
    if indexing.is_running() {
        return Err("a indexação está rodando; espere terminar ou pause".to_string());
    }
    with(&state, |connection, folder| {
        let data = backup::read_export(&folder, &name)?;
        backup::import(connection, &data, &now()).map_err(|error| error.to_string())
    })
}
