//! F22: the search screen (artboard `Search`) over the F14 index. Read only (AD-008).

use tauri::State;

use super::health::DbState;
use crate::search::{search_ids, search_options, SearchIds, SearchOptions, SearchQuery};

#[tauri::command]
pub fn search_media(state: State<'_, DbState>, query: SearchQuery) -> Result<SearchIds, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    search_ids(connection, &query).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn search_filters(state: State<'_, DbState>) -> Result<SearchOptions, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    search_options(connection).map_err(|error| error.to_string())
}
