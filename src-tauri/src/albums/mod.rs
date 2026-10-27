//! F25: albums rebuilt from the Takeout (artboard `Albums`). Every folder right under
//! `Google Fotos` / `Google Photos` that holds media is an album ("Viagem Europa 2022",
//! "Fotos de 2022"), named by its `metadata.json` when there is one. A folder split
//! across export parts (`takeout-…-1-001`, `-2-001`) is one album. Rebuilt on each
//! indexing run; the originals are never touched (AD-008).

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::takeout::{logical_dir, DiscoveryReport};

fn is_photos_root(part: &str) -> bool {
    part == "Google Fotos" || part == "Google Photos"
}

/// The album folder of a media path, logical (export part dropped): the folder right
/// under `Google Fotos`, when the file sits directly in it.
pub(crate) fn album_folder(relative_path: &str) -> Option<String> {
    let (directory, _) = relative_path.rsplit_once('/')?;
    let (parent, _) = directory.rsplit_once('/')?;
    let last = parent.rsplit('/').next()?;
    is_photos_root(last).then(|| logical_dir(directory))
}

fn folder_name(folder: &str) -> &str {
    folder.rsplit('/').next().unwrap_or(folder)
}

struct Found {
    name: String,
    description: Option<String>,
    media: Vec<String>,
}

/// Replaces the library's Takeout albums with what this run found. Returns how many.
pub fn rebuild_takeout_albums(
    connection: &mut Connection,
    library_id: i64,
    report: &DiscoveryReport,
    now: &str,
) -> rusqlite::Result<usize> {
    let mut found: BTreeMap<String, Found> = BTreeMap::new();
    for hit in &report.media {
        if let Some(folder) = album_folder(&hit.relative_path) {
            found
                .entry(folder.clone())
                .or_insert_with(|| Found { name: folder_name(&folder).to_string(), description: None, media: Vec::new() })
                .media
                .push(hit.relative_path.clone());
        }
    }
    for album in &report.albums {
        // `…/Google Fotos/Viagem/metadata.json` names the folder `…/Google Fotos/Viagem`.
        let Some(folder) = album_folder(&album.relative_path) else { continue };
        if let Some(entry) = found.get_mut(&folder) {
            if let Some(title) = album.metadata.title.as_deref().map(str::trim).filter(|title| !title.is_empty()) {
                entry.name = title.to_string();
            }
            entry.description = album.metadata.description.clone();
        }
    }

    let transaction = connection.transaction()?;
    let existing: Vec<(i64, String)> = transaction
        .prepare("SELECT id, folder_relative_path FROM albums WHERE library_id = ?1 AND source = 'takeout_folder'")?
        .query_map([library_id], |row| Ok((row.get(0)?, row.get::<_, Option<String>>(1)?.unwrap_or_default())))?
        .collect::<rusqlite::Result<_>>()?;
    for (id, folder) in &existing {
        if !found.contains_key(folder) {
            transaction.execute("DELETE FROM albums WHERE id = ?1", [id])?;
        }
    }
    {
        let mut media_id = transaction.prepare(
            "SELECT id FROM media_items WHERE library_id = ?1 AND relative_path = ?2 COLLATE NOCASE",
        )?;
        for (folder, album) in &found {
            let id = match existing.iter().find(|(_, existing)| existing == folder) {
                Some((id, _)) => {
                    transaction.execute(
                        "UPDATE albums SET name = ?2, description = ?3 WHERE id = ?1",
                        params![id, album.name, album.description],
                    )?;
                    *id
                }
                None => {
                    transaction.execute(
                        "INSERT INTO albums (library_id, name, source, folder_relative_path, description, created_at)
                         VALUES (?1, ?2, 'takeout_folder', ?3, ?4, ?5)",
                        params![library_id, album.name, folder, album.description, now],
                    )?;
                    transaction.last_insert_rowid()
                }
            };
            transaction.execute("DELETE FROM media_albums WHERE album_id = ?1", [id])?;
            for path in &album.media {
                let media: Option<i64> = media_id.query_row(params![library_id, path], |row| row.get(0)).optional()?;
                if let Some(media) = media {
                    transaction.execute(
                        "INSERT OR IGNORE INTO media_albums (media_id, album_id) VALUES (?1, ?2)",
                        [media, id],
                    )?;
                }
            }
        }
    }
    transaction.commit()?;
    Ok(found.len())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumCard {
    pub id: i64,
    pub name: String,
    /// Items the timeline shows in it.
    pub count: u64,
    /// Oldest and newest `taken_at` (UTC), when any item has a date.
    pub first: Option<String>,
    pub last: Option<String>,
    /// Up to three newest photos (videos when there are not enough): the stack.
    pub covers: Vec<i64>,
}

/// Albums with at least one visible item, newest first.
pub fn album_cards(connection: &Connection) -> rusqlite::Result<Vec<AlbumCard>> {
    let mut cards = connection
        .prepare(
            "SELECT a.id, a.name, count(*), min(m.taken_at), max(m.taken_at)
             FROM albums a
             JOIN media_albums ma ON ma.album_id = a.id
             JOIN media_items m ON m.id = ma.media_id
             WHERE m.is_hidden = 0 AND m.indexing_status != 'missing_file'
             GROUP BY a.id
             ORDER BY max(m.taken_at) IS NULL, max(m.taken_at) DESC, a.name",
        )?
        .query_map([], |row| {
            Ok(AlbumCard {
                id: row.get(0)?,
                name: row.get(1)?,
                count: row.get::<_, i64>(2)?.max(0) as u64,
                first: row.get(3)?,
                last: row.get(4)?,
                covers: Vec::new(),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut covers = connection.prepare(
        "SELECT m.id FROM media_albums ma JOIN media_items m ON m.id = ma.media_id
         WHERE ma.album_id = ?1 AND m.is_hidden = 0 AND m.indexing_status != 'missing_file'
         ORDER BY m.media_type = 'photo' DESC, m.taken_at IS NULL, m.taken_at DESC, m.id DESC
         LIMIT 3",
    )?;
    for card in &mut cards {
        card.covers = covers.query_map([card.id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    }
    Ok(cards)
}
