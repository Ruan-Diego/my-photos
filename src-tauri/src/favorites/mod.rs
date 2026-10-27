//! F26: favorites live only in the app's database (`media_items.is_favorite`); the
//! original file never changes (AD-008). Google Photos favorites come in once from the
//! Takeout JSON (`"favorited": true`), so unmarking one in the app sticks.

#[cfg(test)]
mod tests;

use rusqlite::{params, Connection, OptionalExtension};

/// `app_settings` key: the highest media id whose Takeout JSON was already read for
/// `favorited`. Ids only grow (AUTOINCREMENT), so newer items are the ones above it.
const IMPORTED_UPTO: &str = "takeout_favorites_imported_upto";

/// Marks or unmarks one item. `false` when there is no such item.
pub fn set_favorite(connection: &Connection, media_id: i64, favorite: bool, now: &str) -> rusqlite::Result<bool> {
    let changed = connection.execute(
        "UPDATE media_items SET is_favorite = ?2, updated_at = ?3 WHERE id = ?1",
        params![media_id, favorite, now],
    )?;
    Ok(changed > 0)
}

/// Copies Google Photos favorites from the Takeout JSON of items not read before.
/// Returns how many became favorites.
pub fn import_takeout_favorites(connection: &mut Connection) -> rusqlite::Result<usize> {
    let transaction = connection.transaction()?;
    let upto: i64 = transaction
        .query_row("SELECT value FROM app_settings WHERE key = ?1", [IMPORTED_UPTO], |row| row.get::<_, String>(0))
        .optional()?
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let newest: i64 = transaction.query_row("SELECT coalesce(max(id), 0) FROM media_items", [], |row| row.get(0))?;
    if newest <= upto {
        return Ok(0);
    }
    let marked = transaction.execute(
        "UPDATE media_items SET is_favorite = 1
         WHERE id > ?1 AND is_favorite = 0 AND id IN (
           SELECT media_id FROM media_metadata_raw
           WHERE json_valid(takeout_json) AND json_extract(takeout_json, '$.favorited') = 1)",
        [upto],
    )?;
    transaction.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT (key) DO UPDATE SET value = excluded.value",
        params![IMPORTED_UPTO, newest.to_string()],
    )?;
    transaction.commit()?;
    Ok(marked)
}
