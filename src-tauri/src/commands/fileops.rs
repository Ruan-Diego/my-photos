//! F48: folders, moving and the app's trash (artboards `Folders` and `Trash`).

use std::path::PathBuf;

use rusqlite::Connection;
use tauri::State;

use super::health::DbState;
use super::onboarding::IndexingState;
use crate::fileops::{self, Folder, FolderItem, Outcome, TrashItem};
use crate::time;

fn now() -> String {
    time::iso_utc(std::time::SystemTime::now()).unwrap_or_default()
}

fn root(connection: &Connection) -> Result<PathBuf, String> {
    connection
        .query_row("SELECT root_path FROM libraries ORDER BY id LIMIT 1", [], |row| row.get::<_, String>(0))
        .map(PathBuf::from)
        .map_err(|error| error.to_string())
}

fn read<T>(state: &DbState, work: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    work(connection).map_err(|error| error.to_string())
}

/// Writes wait for no indexing run: the scan would see files mid-move.
fn write<T>(
    state: &DbState,
    indexing: &IndexingState,
    work: impl FnOnce(&mut Connection, PathBuf, String) -> Result<T, String>,
) -> Result<T, String> {
    if indexing.is_running() {
        return Err("a indexação está rodando; espere terminar ou pause".to_string());
    }
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let mut guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_mut().ok_or("biblioteca fechada")?;
    let root = root(connection)?;
    work(connection, root, now())
}

#[tauri::command]
pub fn folder_tree(state: State<'_, DbState>) -> Result<Vec<Folder>, String> {
    read(&state, fileops::folder_tree)
}

#[tauri::command]
pub fn folder_items(state: State<'_, DbState>, folder: String) -> Result<Vec<FolderItem>, String> {
    read(&state, |connection| fileops::folder_items(connection, &folder))
}

#[tauri::command]
pub fn folder_create(state: State<'_, DbState>, indexing: State<'_, IndexingState>, parent: String, name: String) -> Result<String, String> {
    write(&state, &indexing, |connection, root, now| fileops::create_folder(connection, &root, &parent, &name, &now))
}

#[tauri::command]
pub fn folder_rename(state: State<'_, DbState>, indexing: State<'_, IndexingState>, folder: String, name: String) -> Result<String, String> {
    write(&state, &indexing, |connection, root, _| fileops::rename_folder(connection, &root, &folder, &name))
}

#[tauri::command]
pub fn folder_trash(state: State<'_, DbState>, indexing: State<'_, IndexingState>, folder: String) -> Result<(), String> {
    write(&state, &indexing, |connection, root, now| fileops::trash_folder(connection, &root, &folder, &now))
}

#[tauri::command]
pub fn media_move(state: State<'_, DbState>, indexing: State<'_, IndexingState>, ids: Vec<i64>, folder: String) -> Result<Outcome, String> {
    write(&state, &indexing, |connection, root, now| fileops::move_media(connection, &root, &ids, &folder, &now))
}

#[tauri::command]
pub fn media_trash(state: State<'_, DbState>, indexing: State<'_, IndexingState>, ids: Vec<i64>) -> Result<Outcome, String> {
    write(&state, &indexing, |connection, root, now| fileops::trash_media(connection, &root, &ids, &now))
}

#[tauri::command]
pub fn trash_list(state: State<'_, DbState>) -> Result<Vec<TrashItem>, String> {
    read(&state, fileops::trash_list)
}

#[tauri::command]
pub fn trash_restore(state: State<'_, DbState>, indexing: State<'_, IndexingState>, id: i64) -> Result<(), String> {
    write(&state, &indexing, |connection, root, now| fileops::restore(connection, &root, id, &now))
}

#[tauri::command]
pub fn trash_restore_all(state: State<'_, DbState>, indexing: State<'_, IndexingState>) -> Result<Outcome, String> {
    write(&state, &indexing, |connection, root, now| fileops::restore_all(connection, &root, &now))
}
