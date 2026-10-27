//! F25: the albums screen (artboard `Albums`). Read only (AD-008).

use tauri::State;

use super::health::DbState;
use crate::albums::{album_cards, AlbumCard};

#[tauri::command]
pub fn albums_list(state: State<'_, DbState>) -> Result<Vec<AlbumCard>, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    album_cards(connection).map_err(|error| error.to_string())
}
