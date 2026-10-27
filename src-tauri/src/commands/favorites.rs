//! F26: mark or unmark a favorite. Writes only the app's database, never the file (AD-008).

use tauri::State;

use super::health::DbState;
use crate::favorites::set_favorite;
use crate::time;

#[tauri::command]
pub fn favorite_set(state: State<'_, DbState>, id: i64, favorite: bool) -> Result<bool, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    let now = time::iso_utc(std::time::SystemTime::now()).unwrap_or_default();
    match set_favorite(connection, id, favorite, &now) {
        Ok(true) => Ok(favorite),
        Ok(false) => Err("item não encontrado".into()),
        Err(error) => Err(error.to_string()),
    }
}
