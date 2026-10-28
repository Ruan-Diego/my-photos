//! F35: copies of the app's database and an export of what only the app knows (favorites
//! and notes), both in `.myphotos/backups` next to `library.db`. The photos themselves are
//! never copied: the backup protects the index, not the disk.

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

pub const FOLDER: &str = "backups";
/// Copies of the database kept; older ones go.
pub const KEEP: usize = 7;
/// The automatic copy runs at startup when the newest one is older than this.
pub const EVERY: Duration = Duration::from_secs(24 * 60 * 60);
/// An export bigger than this is not an export of this app.
const MAX_IMPORT_BYTES: u64 = 64 * 1024 * 1024;
const APP: &str = "my-photos";
const VERSION: u32 = 1;

/// `.myphotos/backups` for an on-disk database; `None` for one in memory.
pub fn folder(connection: &Connection) -> Option<PathBuf> {
    let database = connection.path().filter(|path| !path.is_empty())?;
    Some(Path::new(database).parent()?.join(FOLDER))
}

/// `20261028140245` from `2026-10-28T14:02:45.000Z`.
fn stamp(now: &str) -> String {
    now.chars().filter(char::is_ascii_digit).take(14).collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub name: String,
    pub bytes: u64,
    /// `database` (copy of `library.db`) or `export` (favorites and notes).
    pub kind: &'static str,
}

fn kind_of(name: &str) -> Option<&'static str> {
    if name.starts_with("library-") && name.ends_with(".db") {
        Some("database")
    } else if name.starts_with("dados-") && name.ends_with(".json") {
        Some("export")
    } else {
        None
    }
}

/// The backups in `folder`, newest first (names carry the time).
pub fn list(folder: &Path) -> Vec<BackupFile> {
    let mut files: Vec<BackupFile> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let kind = kind_of(&name)?;
                    let bytes = entry.metadata().ok().filter(|meta| meta.is_file())?.len();
                    Some(BackupFile { name, bytes, kind })
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort_by(|a, b| b.name.cmp(&a.name));
    files
}

/// A consistent copy of the open database (`VACUUM INTO`), then only the newest `KEEP`.
pub fn backup_now(connection: &Connection, folder: &Path, now: &str) -> Result<PathBuf, String> {
    std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let file = folder.join(format!("library-{}.db", stamp(now)));
    if file.exists() {
        return Ok(file);
    }
    connection
        .execute("VACUUM INTO ?1", [file.to_string_lossy()])
        .map_err(|error| error.to_string())?;
    for old in list(folder).into_iter().filter(|backup| backup.kind == "database").skip(KEEP) {
        let _ = std::fs::remove_file(folder.join(old.name));
    }
    Ok(file)
}

/// Startup: a copy when there is none from the last `EVERY`. `None` when not needed.
pub fn auto_backup(connection: &Connection, folder: &Path, now: SystemTime, now_iso: &str) -> Result<Option<PathBuf>, String> {
    let newest = list(folder)
        .into_iter()
        .find(|backup| backup.kind == "database")
        .and_then(|backup| std::fs::metadata(folder.join(backup.name)).ok()?.modified().ok());
    if newest.is_some_and(|time| now.duration_since(time).unwrap_or_default() < EVERY) {
        return Ok(None);
    }
    backup_now(connection, folder, now_iso).map(Some)
}

/// One media in an export: where it was and what its bytes were, so it is found again
/// after a reindex or a move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaRef {
    pub path: String,
    pub sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoteRef {
    #[serde(flatten)]
    pub media: MediaRef,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppData {
    pub app: String,
    pub version: u32,
    pub exported_at: String,
    pub favorites: Vec<MediaRef>,
    /// Google Photos favorites unmarked in the app: a new index would mark them again.
    pub unfavorites: Vec<MediaRef>,
    pub notes: Vec<NoteRef>,
}

fn refs(connection: &Connection, sql: &str) -> rusqlite::Result<Vec<MediaRef>> {
    connection
        .prepare(sql)?
        .query_map([], |row| Ok(MediaRef { path: row.get(0)?, sha256: row.get(1)? }))?
        .collect()
}

pub fn export(connection: &Connection, now: &str) -> rusqlite::Result<AppData> {
    let favorites = refs(
        connection,
        "SELECT relative_path, file_hash_sha256 FROM media_items
         WHERE is_favorite = 1 AND indexing_status != 'missing_file' ORDER BY relative_path",
    )?;
    let unfavorites = refs(
        connection,
        "SELECT m.relative_path, m.file_hash_sha256 FROM media_items m
         JOIN media_metadata_raw r ON r.media_id = m.id
         WHERE m.is_favorite = 0 AND m.indexing_status != 'missing_file'
           AND json_valid(r.takeout_json) AND json_extract(r.takeout_json, '$.favorited') = 1
         ORDER BY m.relative_path",
    )?;
    let notes = connection
        .prepare(
            "SELECT m.relative_path, m.file_hash_sha256, n.note FROM user_notes n
             JOIN media_items m ON m.id = n.media_id
             WHERE n.note IS NOT NULL AND n.note != '' AND m.indexing_status != 'missing_file'
             ORDER BY m.relative_path",
        )?
        .query_map([], |row| {
            Ok(NoteRef { media: MediaRef { path: row.get(0)?, sha256: row.get(1)? }, note: row.get(2)? })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(AppData { app: APP.into(), version: VERSION, exported_at: now.into(), favorites, unfavorites, notes })
}

/// Writes `dados-<time>.json` in `folder`.
pub fn write_export(connection: &Connection, folder: &Path, now: &str) -> Result<PathBuf, String> {
    let data = export(connection, now).map_err(|error| error.to_string())?;
    std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let file = folder.join(format!("dados-{}.json", stamp(now)));
    let text = serde_json::to_string_pretty(&data).map_err(|error| error.to_string())?;
    std::fs::write(&file, text).map_err(|error| error.to_string())?;
    Ok(file)
}

/// Reads an export by name from `folder` only: a bare `dados-*.json` name, never a path.
pub fn read_export(folder: &Path, name: &str) -> Result<AppData, String> {
    if kind_of(name) != Some("export") || name.contains(['/', '\\']) || name.contains("..") {
        return Err("arquivo de exportação inválido".into());
    }
    let file = folder.join(name);
    let bytes = std::fs::metadata(&file).map_err(|error| error.to_string())?.len();
    if bytes > MAX_IMPORT_BYTES {
        return Err("arquivo grande demais para uma exportação do app".into());
    }
    let data: AppData = serde_json::from_slice(&std::fs::read(&file).map_err(|error| error.to_string())?)
        .map_err(|_| "o arquivo não é uma exportação do My Photos".to_string())?;
    if data.app != APP || data.version > VERSION {
        return Err("o arquivo não é uma exportação desta versão do My Photos".into());
    }
    Ok(data)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub favorites: u64,
    pub unfavorites: u64,
    pub notes: u64,
    /// Entries whose media is not in this library (by path or by content).
    pub missing: u64,
}

/// Media ids for a reference: the same path if it is still there, otherwise every copy
/// with the same content (a move or a reindex keeps the bytes).
struct Finder {
    by_path: HashMap<String, i64>,
    by_sha: HashMap<String, Vec<i64>>,
}

impl Finder {
    fn new(connection: &Connection) -> rusqlite::Result<Self> {
        let mut by_path = HashMap::new();
        let mut by_sha: HashMap<String, Vec<i64>> = HashMap::new();
        let mut statement = connection
            .prepare("SELECT id, relative_path, file_hash_sha256 FROM media_items WHERE indexing_status != 'missing_file'")?;
        let mut rows = statement.query([])?;
        while let Some(row) = rows.next()? {
            let id: i64 = row.get(0)?;
            by_path.insert(row.get::<_, String>(1)?.to_lowercase(), id);
            if let Some(sha) = row.get::<_, Option<String>>(2)? {
                by_sha.entry(sha.to_lowercase()).or_default().push(id);
            }
        }
        Ok(Self { by_path, by_sha })
    }

    fn find(&self, media: &MediaRef) -> Vec<i64> {
        if let Some(&id) = self.by_path.get(&media.path.to_lowercase()) {
            return vec![id];
        }
        media
            .sha256
            .as_ref()
            .and_then(|sha| self.by_sha.get(&sha.to_lowercase()))
            .cloned()
            .unwrap_or_default()
    }
}

/// Applies an export: marks favorites, unmarks the unmarked, and writes notes (an
/// imported note replaces the one there). One transaction: all or nothing.
pub fn import(connection: &mut Connection, data: &AppData, now: &str) -> rusqlite::Result<ImportOutcome> {
    let finder = Finder::new(connection)?;
    let transaction = connection.transaction()?;
    let mut outcome = ImportOutcome::default();
    {
        let mut favorite = transaction.prepare("UPDATE media_items SET is_favorite = ?2, updated_at = ?3 WHERE id = ?1")?;
        for (list, value, count) in [(&data.favorites, true, &mut outcome.favorites), (&data.unfavorites, false, &mut outcome.unfavorites)] {
            for media in list {
                let ids = finder.find(media);
                if ids.is_empty() {
                    outcome.missing += 1;
                }
                for id in ids {
                    *count += favorite.execute(params![id, value, now])? as u64;
                }
            }
        }
        let mut note = transaction.prepare(
            "INSERT INTO user_notes (media_id, note, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (media_id) DO UPDATE SET note = excluded.note, updated_at = excluded.updated_at",
        )?;
        for entry in &data.notes {
            let ids = finder.find(&entry.media);
            if ids.is_empty() {
                outcome.missing += 1;
            }
            for id in ids {
                outcome.notes += note.execute(params![id, entry.note, now])? as u64;
            }
        }
    }
    transaction.commit()?;
    Ok(outcome)
}
