//! F29: the duplicates screen (artboard `Duplicates`). Read only: nothing is deleted.

use tauri::State;

use super::health::DbState;
use crate::duplicates::{duplicates, Duplicates};

#[tauri::command]
pub fn duplicates_list(state: State<'_, DbState>) -> Result<Duplicates, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    duplicates(connection).map_err(|error| error.to_string())
}
