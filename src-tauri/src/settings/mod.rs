//! F31: the settings screen (artboard `Settings`). Choices live in `app_settings`,
//! on the library disk with everything else (AD-011); the theme stays in the window.

use std::fs;
use std::io;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::thumbs::{cache_bytes, cache_limit, CACHE_LIMIT_SETTING};

/// "Gerar capa dos vídeos": on unless turned off.
pub const VIDEO_COVERS: &str = "video_covers";
/// "Procurar arquivos novos ao conectar o HD": off unless turned on.
pub const SCAN_ON_CONNECT: &str = "scan_on_connect";

/// F37: "Procurar atualizações ao abrir". Off unless turned on: with it off the app makes
/// no network connection at all (F36, sec04).
pub const UPDATE_CHECKS: &str = "update_checks";

/// Folders the scan never enters: the app's own, and hidden system folders such as these.
pub const IGNORED_FOLDERS: [&str; 3] = [".myphotos", "$RECYCLE.BIN", "System Volume Information"];

/// Choices offered for the thumbnail cache, in bytes.
pub const CACHE_LIMITS: [u64; 5] = [1 << 30, 2 << 30, 4 << 30, 5 << 30, 10 << 30];

pub fn flag(connection: &Connection, key: &str, default: bool) -> rusqlite::Result<bool> {
    let value: Option<Option<String>> = connection
        .query_row("SELECT value FROM app_settings WHERE key = ?1", [key], |row| row.get(0))
        .optional()?;
    Ok(match value.flatten().as_deref().map(str::trim) {
        Some("1") => true,
        Some("0") => false,
        _ => default,
    })
}

fn store(connection: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT INTO app_settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Only the two switches of the screen can be set this way.
pub fn set_flag(connection: &Connection, key: &str, on: bool) -> Result<(), String> {
    if ![VIDEO_COVERS, SCAN_ON_CONNECT, UPDATE_CHECKS].contains(&key) {
        return Err(format!("configuração desconhecida: {key}"));
    }
    store(connection, key, if on { "1" } else { "0" }).map_err(|error| error.to_string())
}

/// One of `CACHE_LIMITS`; the excess leaves on the next thumbnail run.
pub fn set_cache_limit(connection: &Connection, bytes: u64) -> Result<(), String> {
    if !CACHE_LIMITS.contains(&bytes) {
        return Err(format!("limite fora das opções: {bytes}"));
    }
    store(connection, CACHE_LIMIT_SETTING, &bytes.to_string()).map_err(|error| error.to_string())
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub root_path: Option<String>,
    pub volume_label: Option<String>,
    pub database_path: Option<String>,
    pub database_bytes: Option<u64>,
    pub thumbnail_bytes: u64,
    pub thumbnail_limit_bytes: u64,
    pub cache_limits: Vec<u64>,
    pub video_covers: bool,
    pub ffmpeg: bool,
    pub scan_on_connect: bool,
    pub ignored_folders: Vec<String>,
    pub update_checks: bool,
    /// F37: this build knows where to look for updates (a release build). Filled by the command.
    pub updater_configured: bool,
    /// The app's version. Filled by the command.
    pub version: String,
}

pub fn settings(connection: &Connection, ffmpeg: bool) -> rusqlite::Result<Settings> {
    let (root_path, volume_label) = connection
        .query_row("SELECT root_path, volume_label FROM libraries ORDER BY id LIMIT 1", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()?
        .unwrap_or((None, None));
    let database_path = connection.path().filter(|path| !path.is_empty()).map(str::to_string);
    let database_bytes = database_path.as_deref().and_then(database_size);
    Ok(Settings {
        root_path,
        volume_label,
        database_path,
        database_bytes,
        thumbnail_bytes: cache_bytes(connection)?,
        thumbnail_limit_bytes: cache_limit(connection)?,
        cache_limits: CACHE_LIMITS.to_vec(),
        video_covers: flag(connection, VIDEO_COVERS, true)?,
        ffmpeg,
        scan_on_connect: flag(connection, SCAN_ON_CONNECT, false)?,
        ignored_folders: IGNORED_FOLDERS.iter().map(|name| name.to_string()).collect(),
        update_checks: flag(connection, UPDATE_CHECKS, false)?,
        updater_configured: false,
        version: String::new(),
    })
}

/// The database plus its write-ahead log, which holds recent changes until a checkpoint.
fn database_size(path: &str) -> Option<u64> {
    let main = fs::metadata(path).ok()?.len();
    let wal = fs::metadata(format!("{path}-wal")).map(|meta| meta.len()).unwrap_or(0);
    Some(main + wal)
}

/// "Limpar miniaturas": forgets every thumbnail and deletes the `thumbs` folder. The
/// next "Procurar arquivos novos" makes them again. Returns the bytes the index counted.
pub fn clear_thumbnails(connection: &Connection, root: &Path) -> Result<u64, String> {
    let bytes = cache_bytes(connection).map_err(|error| error.to_string())?;
    connection.execute("DELETE FROM thumbnails", []).map_err(|error| error.to_string())?;
    match fs::remove_dir_all(root.join(".myphotos").join("thumbs")) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error.to_string()),
        _ => Ok(bytes),
    }
}

#[cfg(test)]
mod tests;
