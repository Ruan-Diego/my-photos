//! F48: file operations on the library disk (AD-008): folders, moving media with their
//! Takeout JSON, and the app's own trash in `.myphotos\trash` (AD-009, AD-011).
//!
//! Every write starts from an explicit action in the UI. The file moves first; the index
//! follows in the same call, and a failed index update moves the file back. A trashed
//! media keeps its row, marked `missing_file` so every screen hides it and the scan leaves
//! it alone; restoring moves it back and marks it `ok`.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::thumbs::{thumb_path, ThumbSize};

/// Where trashed items live, relative to the root.
pub const TRASH_DIR: &str = ".myphotos/trash";
/// AD-009: days an item stays in the trash.
pub const TRASH_DAYS: i64 = 30;

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    /// `""` for the root, else `a/b` with `/` like `relative_path`.
    pub path: String,
    pub name: String,
    pub depth: usize,
    /// Media right inside it.
    pub direct: u64,
    /// Media inside it or any folder below.
    pub total: u64,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FolderItem {
    pub id: i64,
    pub filename: String,
    pub video: bool,
    pub has_json: bool,
}

#[derive(Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    /// Items done.
    pub done: u64,
    /// `(name, reason)` of each item left as it was.
    pub skipped: Vec<(String, String)>,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrashItem {
    pub id: i64,
    pub kind: String,
    /// Media id for the thumbnail; `None` for a folder.
    pub media_id: Option<i64>,
    pub name: String,
    /// Folder it was in (`""` for the root).
    pub folder: String,
    /// Media inside a trashed folder.
    pub items: u64,
    pub trashed_at: String,
    pub purge_after: String,
}

fn sql(error: rusqlite::Error) -> String {
    error.to_string()
}

fn io_text(error: io::Error) -> String {
    match error.kind() {
        io::ErrorKind::NotFound => "o arquivo não foi encontrado no disco".to_string(),
        io::ErrorKind::PermissionDenied => "o Windows negou o acesso (arquivo aberto em outro programa?)".to_string(),
        _ => error.to_string(),
    }
}

/// `a/b` → `a`; `a` → `""`.
pub fn parent_of(path: &str) -> &str {
    path.rfind('/').map_or("", |at| &path[..at])
}

/// `a/b` → `b`.
pub fn name_of(path: &str) -> &str {
    path.rfind('/').map_or(path, |at| &path[at + 1..])
}

fn join(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

#[cfg(test)]
/// `path` is `folder` or inside it (NOCASE, like the index).
fn under(path: &str, folder: &str) -> bool {
    folder.is_empty()
        || path.eq_ignore_ascii_case(folder)
        || (path.len() > folder.len()
            && path.as_bytes()[folder.len()] == b'/'
            && path[..folder.len()].eq_ignore_ascii_case(folder))
}

fn on_disk(root: &Path, relative: &str) -> PathBuf {
    relative.split('/').filter(|part| !part.is_empty()).fold(root.to_path_buf(), |path, part| path.join(part))
}

/// A folder name Windows accepts and the app can show. Returns it trimmed.
pub fn valid_name(name: &str) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("o nome está vazio".to_string());
    }
    if name.chars().count() > 120 {
        return Err("o nome é longo demais".to_string());
    }
    if let Some(bad) = name.chars().find(|c| matches!(c, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control()) {
        return Err(format!("o nome não pode ter {bad:?}"));
    }
    if name.ends_with('.') || name == ".." {
        return Err("o nome não pode terminar com ponto".to_string());
    }
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
        || ((stem.starts_with("COM") || stem.starts_with("LPT")) && stem.len() == 4 && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        return Err(format!("{name} é um nome reservado do Windows"));
    }
    if name.eq_ignore_ascii_case(".myphotos") {
        return Err("esse nome é do próprio app".to_string());
    }
    Ok(name.to_string())
}

fn library_id(connection: &Connection) -> Result<i64, String> {
    connection.query_row("SELECT id FROM libraries ORDER BY id LIMIT 1", [], |row| row.get(0)).map_err(sql)
}

fn known_folder(connection: &Connection, folder: &str) -> Result<bool, String> {
    if folder.is_empty() {
        return Ok(true);
    }
    connection
        .query_row("SELECT 1 FROM folders WHERE relative_path = ?1", [folder], |_| Ok(()))
        .optional()
        .map(|found| found.is_some())
        .map_err(sql)
}

/// The folders the index knows, root first, each after its parent, with media counts.
pub fn folder_tree(connection: &Connection) -> rusqlite::Result<Vec<Folder>> {
    let mut paths: Vec<String> = connection
        .prepare("SELECT relative_path FROM folders")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    paths.sort_by_key(|path| path.to_lowercase());
    let media: Vec<String> = connection
        .prepare("SELECT relative_path FROM media_items WHERE is_hidden = 0 AND indexing_status != 'missing_file'")?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    let mut folders = vec![Folder { path: String::new(), name: String::new(), depth: 0, direct: 0, total: media.len() as u64 }];
    folders.extend(paths.into_iter().map(|path| Folder {
        depth: path.split('/').count(),
        name: name_of(&path).to_string(),
        path,
        direct: 0,
        total: 0,
    }));
    // Uma passada pelas mídias e outra pelas pastas: conta direto e sobe para os pais.
    let mut direct: HashMap<String, u64> = HashMap::new();
    for file in &media {
        *direct.entry(parent_of(file).to_lowercase()).or_default() += 1;
    }
    let mut totals: HashMap<String, u64> = HashMap::new();
    for (folder, count) in &direct {
        let mut path = folder.as_str();
        while !path.is_empty() {
            *totals.entry(path.to_string()).or_default() += count;
            path = parent_of(path);
        }
    }
    folders[0].direct = direct.get("").copied().unwrap_or(0);
    for folder in folders.iter_mut().skip(1) {
        let key = folder.path.to_lowercase();
        folder.direct = direct.get(&key).copied().unwrap_or(0);
        folder.total = totals.get(&key).copied().unwrap_or(0);
    }
    Ok(folders)
}

/// Media right inside `folder`, by name.
pub fn folder_items(connection: &Connection, folder: &str) -> rusqlite::Result<Vec<FolderItem>> {
    let mut items: Vec<(String, FolderItem)> = connection
        .prepare(
            "SELECT id, relative_path, filename, media_type, has_takeout_json FROM media_items
             WHERE is_hidden = 0 AND indexing_status != 'missing_file'",
        )?
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(1)?,
                FolderItem {
                    id: row.get(0)?,
                    filename: row.get(2)?,
                    video: row.get::<_, String>(3)? == "video",
                    has_json: row.get::<_, i64>(4)? == 1,
                },
            ))
        })?
        .collect::<Result<_, _>>()?;
    items.retain(|(path, _)| parent_of(path).eq_ignore_ascii_case(folder));
    items.sort_by(|a, b| a.1.filename.to_lowercase().cmp(&b.1.filename.to_lowercase()));
    Ok(items.into_iter().map(|(_, item)| item).collect())
}

/// "Nova pasta" inside `parent`. Returns its path.
pub fn create_folder(connection: &Connection, root: &Path, parent: &str, name: &str, now: &str) -> Result<String, String> {
    let name = valid_name(name)?;
    if !known_folder(connection, parent)? {
        return Err("a pasta de cima não existe mais".to_string());
    }
    let path = join(parent, &name);
    if known_folder(connection, &path)? || on_disk(root, &path).exists() {
        return Err(format!("já existe {name} aqui"));
    }
    fs::create_dir(on_disk(root, &path)).map_err(io_text)?;
    let library = library_id(connection)?;
    connection
        .execute(
            "INSERT INTO folders (library_id, relative_path, created_at) VALUES (?1, ?2, ?3)",
            params![library, path, now],
        )
        .map_err(sql)?;
    Ok(path)
}

/// Rewrites every indexed path at or under `from` to sit under `to`.
fn repath(connection: &Connection, from: &str, to: &str) -> rusqlite::Result<()> {
    let length = from.chars().count() as i64;
    let prefix = |column: &str| {
        format!(
            "{column} = CASE WHEN {column} = ?1 COLLATE NOCASE THEN ?2
                             WHEN substr({column}, 1, ?3 + 1) = ?1 || '/' COLLATE NOCASE THEN ?2 || substr({column}, ?3 + 1)
                             ELSE {column} END"
        )
    };
    let like = "(x = ?1 COLLATE NOCASE OR substr(x, 1, ?3 + 1) = ?1 || '/' COLLATE NOCASE)";
    connection.execute(
        &format!("UPDATE folders SET {} WHERE {}", prefix("relative_path"), like.replace('x', "relative_path")),
        params![from, to, length],
    )?;
    connection.execute(
        &format!(
            "UPDATE media_items SET {}, {} WHERE {} OR {}",
            prefix("relative_path"),
            prefix("json_relative_path"),
            like.replace('x', "relative_path"),
            like.replace('x', "json_relative_path")
        ),
        params![from, to, length],
    )?;
    connection.execute(
        &format!("UPDATE albums SET {} WHERE {}", prefix("folder_relative_path"), like.replace('x', "folder_relative_path")),
        params![from, to, length],
    )?;
    Ok(())
}

/// "Renomear pasta". Returns the new path.
pub fn rename_folder(connection: &mut Connection, root: &Path, folder: &str, name: &str) -> Result<String, String> {
    if folder.is_empty() {
        return Err("a raiz do disco não pode ser renomeada".to_string());
    }
    let name = valid_name(name)?;
    if !known_folder(connection, folder)? {
        return Err("a pasta não existe mais".to_string());
    }
    let target = join(parent_of(folder), &name);
    if target == folder {
        return Ok(target);
    }
    // Só mudar maiúsculas e minúsculas é a mesma pasta no NTFS.
    if !target.eq_ignore_ascii_case(folder) && (known_folder(connection, &target)? || on_disk(root, &target).exists()) {
        return Err(format!("já existe {name} aqui"));
    }
    fs::rename(on_disk(root, folder), on_disk(root, &target)).map_err(io_text)?;
    let updated = connection.transaction().and_then(|transaction| {
        repath(&transaction, folder, &target)?;
        transaction.commit()
    });
    if let Err(error) = updated {
        let _ = fs::rename(on_disk(root, &target), on_disk(root, folder));
        return Err(sql(error));
    }
    Ok(target)
}

struct Media {
    id: i64,
    path: String,
    filename: String,
    json: Option<String>,
}

fn media(connection: &Connection, id: i64) -> Result<Option<Media>, String> {
    connection
        .query_row(
            "SELECT id, relative_path, filename, json_relative_path FROM media_items
             WHERE id = ?1 AND indexing_status != 'missing_file'",
            [id],
            |row| Ok(Media { id: row.get(0)?, path: row.get(1)?, filename: row.get(2)?, json: row.get(3)? }),
        )
        .optional()
        .map_err(sql)
}

/// Moves a media and its JSON to `folder` (relative). The JSON stays put when its name is
/// taken there. Returns the new `(media, json)` paths.
fn move_files(root: &Path, item: &Media, folder: &str) -> Result<(String, Option<String>), String> {
    let target = join(folder, &item.filename);
    if on_disk(root, &target).exists() {
        return Err(format!("já existe {} no destino", item.filename));
    }
    if let Some(parent) = on_disk(root, &target).parent() {
        fs::create_dir_all(parent).map_err(io_text)?;
    }
    fs::rename(on_disk(root, &item.path), on_disk(root, &target)).map_err(io_text)?;
    let json = item.json.as_ref().map(|json| {
        let moved = join(folder, name_of(json));
        let source = on_disk(root, json);
        if source.exists() && !on_disk(root, &moved).exists() && fs::rename(&source, on_disk(root, &moved)).is_ok() {
            moved
        } else {
            json.clone()
        }
    });
    Ok((target, json))
}

/// Puts files moved by `move_files` back.
fn undo_move(root: &Path, item: &Media, moved: &(String, Option<String>)) {
    let _ = fs::rename(on_disk(root, &moved.0), on_disk(root, &item.path));
    if let (Some(json), Some(original)) = (&moved.1, &item.json) {
        if json != original {
            let _ = fs::rename(on_disk(root, json), on_disk(root, original));
        }
    }
}

/// "Mover para…": each media with its Takeout JSON (AD-008).
pub fn move_media(connection: &Connection, root: &Path, ids: &[i64], folder: &str, now: &str) -> Result<Outcome, String> {
    if !known_folder(connection, folder)? {
        return Err("a pasta de destino não existe mais".to_string());
    }
    let mut outcome = Outcome::default();
    for &id in ids {
        let Some(item) = media(connection, id)? else {
            outcome.skipped.push((format!("#{id}"), "não está mais no índice".to_string()));
            continue;
        };
        if parent_of(&item.path).eq_ignore_ascii_case(folder) {
            continue;
        }
        let moved = match move_files(root, &item, folder) {
            Ok(moved) => moved,
            Err(reason) => {
                outcome.skipped.push((item.filename.clone(), reason));
                continue;
            }
        };
        let updated = connection.execute(
            "UPDATE media_items SET relative_path = ?2, json_relative_path = ?3, updated_at = ?4 WHERE id = ?1",
            params![item.id, moved.0, moved.1, now],
        );
        if let Err(error) = updated {
            undo_move(root, &item, &moved);
            outcome.skipped.push((item.filename.clone(), sql(error)));
            continue;
        }
        outcome.done += 1;
    }
    Ok(outcome)
}

/// `.myphotos/trash/<stamp>-<n>`: one folder per entry, so equal names never collide.
fn trash_slot(connection: &Connection, now: &str) -> Result<String, String> {
    let next: i64 = connection
        .query_row("SELECT coalesce(max(id), 0) + 1 FROM trash_entries", [], |row| row.get(0))
        .map_err(sql)?;
    let stamp: String = now.chars().filter(char::is_ascii_digit).take(14).collect();
    Ok(format!("{TRASH_DIR}/{stamp}-{next}"))
}

/// "Mover para a lixeira" for media.
pub fn trash_media(connection: &Connection, root: &Path, ids: &[i64], now: &str) -> Result<Outcome, String> {
    let library = library_id(connection)?;
    let mut outcome = Outcome::default();
    for &id in ids {
        let Some(item) = media(connection, id)? else {
            outcome.skipped.push((format!("#{id}"), "não está mais no índice".to_string()));
            continue;
        };
        let slot = trash_slot(connection, now)?;
        let moved = match move_files(root, &item, &slot) {
            Ok(moved) => moved,
            Err(reason) => {
                outcome.skipped.push((item.filename.clone(), reason));
                continue;
            }
        };
        let json_moved = moved.1.as_ref().filter(|json| item.json.as_ref() != Some(*json));
        let recorded = connection
            .execute(
                "INSERT INTO trash_entries (library_id, kind, media_id, original_relative_path, trash_relative_path,
                   json_original_relative_path, json_trash_relative_path, trashed_at)
                 VALUES (?1, 'file', ?2, ?3, ?4, ?5, ?6, ?7)",
                params![library, item.id, item.path, moved.0, json_moved.and(item.json.as_ref()), json_moved, now],
            )
            .and_then(|_| {
                connection.execute(
                    "UPDATE media_items SET indexing_status = 'missing_file', updated_at = ?2 WHERE id = ?1",
                    params![item.id, now],
                )
            });
        if let Err(error) = recorded {
            undo_move(root, &item, &moved);
            outcome.skipped.push((item.filename.clone(), sql(error)));
            continue;
        }
        outcome.done += 1;
    }
    Ok(outcome)
}

/// "Mover pasta para a lixeira": the whole folder, with everything in it.
pub fn trash_folder(connection: &mut Connection, root: &Path, folder: &str, now: &str) -> Result<(), String> {
    if folder.is_empty() {
        return Err("a raiz do disco não vai para a lixeira".to_string());
    }
    if !known_folder(connection, folder)? {
        return Err("a pasta não existe mais".to_string());
    }
    let library = library_id(connection)?;
    let target = join(&trash_slot(connection, now)?, name_of(folder));
    if let Some(parent) = on_disk(root, &target).parent() {
        fs::create_dir_all(parent).map_err(io_text)?;
    }
    fs::rename(on_disk(root, folder), on_disk(root, &target)).map_err(io_text)?;
    let length = folder.chars().count() as i64;
    let recorded = connection.transaction().and_then(|transaction| {
        transaction.execute(
            "INSERT INTO trash_entries (library_id, kind, original_relative_path, trash_relative_path, trashed_at)
             VALUES (?1, 'folder', ?2, ?3, ?4)",
            params![library, folder, target, now],
        )?;
        transaction.execute(
            "UPDATE media_items SET indexing_status = 'missing_file', updated_at = ?3
             WHERE indexing_status != 'missing_file' AND substr(relative_path, 1, ?2 + 1) = ?1 || '/' COLLATE NOCASE",
            params![folder, length, now],
        )?;
        transaction.execute(
            "DELETE FROM folders WHERE relative_path = ?1 COLLATE NOCASE OR substr(relative_path, 1, ?2 + 1) = ?1 || '/' COLLATE NOCASE",
            params![folder, length],
        )?;
        transaction.commit()
    });
    if let Err(error) = recorded {
        let _ = fs::rename(on_disk(root, &target), on_disk(root, folder));
        return Err(sql(error));
    }
    Ok(())
}

fn folder_media_count(connection: &Connection, folder: &str, trashed_at: &str) -> rusqlite::Result<u64> {
    let length = folder.chars().count() as i64;
    connection.query_row(
        "SELECT count(*) FROM media_items WHERE indexing_status = 'missing_file' AND updated_at = ?3
           AND substr(relative_path, 1, ?2 + 1) = ?1 || '/' COLLATE NOCASE",
        params![folder, length, trashed_at],
        |row| row.get::<_, i64>(0).map(|count| count.max(0) as u64),
    )
}

/// The trash, newest first.
pub fn trash_list(connection: &Connection) -> rusqlite::Result<Vec<TrashItem>> {
    let rows: Vec<(i64, String, Option<i64>, String, String, String)> = connection
        .prepare(
            "SELECT id, kind, media_id, original_relative_path, trashed_at, purge_after FROM trash_entries
             ORDER BY trashed_at DESC, id DESC",
        )?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)))?
        .collect::<Result<_, _>>()?;
    rows.into_iter()
        .map(|(id, kind, media_id, original, trashed_at, purge_after)| {
            let items = if kind == "folder" { folder_media_count(connection, &original, &trashed_at)? } else { 0 };
            Ok(TrashItem {
                id,
                kind,
                media_id,
                name: name_of(&original).to_string(),
                folder: parent_of(&original).to_string(),
                items,
                trashed_at,
                purge_after,
            })
        })
        .collect()
}

struct Entry {
    kind: String,
    media_id: Option<i64>,
    original: String,
    trash: String,
    json_original: Option<String>,
    json_trash: Option<String>,
    trashed_at: String,
}

fn entry(connection: &Connection, id: i64) -> Result<Entry, String> {
    connection
        .query_row(
            "SELECT kind, media_id, original_relative_path, trash_relative_path, json_original_relative_path,
                    json_trash_relative_path, trashed_at
             FROM trash_entries WHERE id = ?1",
            [id],
            |row| {
                Ok(Entry {
                    kind: row.get(0)?,
                    media_id: row.get(1)?,
                    original: row.get(2)?,
                    trash: row.get(3)?,
                    json_original: row.get(4)?,
                    json_trash: row.get(5)?,
                    trashed_at: row.get(6)?,
                })
            },
        )
        .optional()
        .map_err(sql)?
        .ok_or_else(|| "o item não está mais na lixeira".to_string())
}

/// The entry's own folder in the trash (`.myphotos/trash/<slot>`). `None` for anything
/// outside the trash, so a bad row can never make the app delete elsewhere.
fn slot_of(trash: &str) -> Option<String> {
    let parts: Vec<&str> = trash.splitn(4, '/').collect();
    let inside = parts.len() == 4 && format!("{}/{}", parts[0], parts[1]).eq_ignore_ascii_case(TRASH_DIR);
    let safe = |part: &str| !part.is_empty() && part != "." && part != "..";
    (inside && safe(parts[2])).then(|| parts[..3].join("/"))
}

/// Records the folders of `path` (not itself) that the index lost, e.g. after a rename.
fn ensure_folders(connection: &Connection, root: &Path, path: &str, now: &str) -> Result<(), String> {
    let library = library_id(connection)?;
    let mut folder = String::new();
    let parts: Vec<&str> = path.split('/').collect();
    for part in &parts[..parts.len().saturating_sub(1)] {
        folder = join(&folder, part);
        fs::create_dir_all(on_disk(root, &folder)).map_err(io_text)?;
        connection
            .execute(
                "INSERT OR IGNORE INTO folders (library_id, relative_path, created_at) VALUES (?1, ?2, ?3)",
                params![library, folder, now],
            )
            .map_err(sql)?;
    }
    Ok(())
}

/// "Restaurar": back to where it was, with its JSON.
pub fn restore(connection: &mut Connection, root: &Path, id: i64, now: &str) -> Result<(), String> {
    let entry = entry(connection, id)?;
    if on_disk(root, &entry.original).exists() {
        return Err(format!("já existe {} na pasta de origem", name_of(&entry.original)));
    }
    ensure_folders(connection, root, &entry.original, now)?;
    fs::rename(on_disk(root, &entry.trash), on_disk(root, &entry.original)).map_err(io_text)?;
    if let (Some(from), Some(to)) = (&entry.json_trash, &entry.json_original) {
        if !on_disk(root, to).exists() {
            let _ = fs::rename(on_disk(root, from), on_disk(root, to));
        }
    }
    let length = entry.original.chars().count() as i64;
    let library = library_id(connection)?;
    let updated = connection.transaction().and_then(|transaction| {
        if entry.kind == "folder" {
            transaction.execute(
                "UPDATE media_items SET indexing_status = 'ok', updated_at = ?4
                 WHERE indexing_status = 'missing_file' AND updated_at = ?3
                   AND substr(relative_path, 1, ?2 + 1) = ?1 || '/' COLLATE NOCASE",
                params![entry.original, length, entry.trashed_at, now],
            )?;
            // A pasta e as de dentro voltam para a árvore.
            let mut inside = vec![entry.original.clone()];
            collect_dirs(&on_disk(root, &entry.original), &entry.original, &mut inside);
            for folder in inside {
                transaction.execute(
                    "INSERT OR IGNORE INTO folders (library_id, relative_path, created_at) VALUES (?1, ?2, ?3)",
                    params![library, folder, now],
                )?;
            }
        } else if let Some(media_id) = entry.media_id {
            transaction.execute(
                "UPDATE media_items SET indexing_status = 'ok', relative_path = ?2, updated_at = ?3 WHERE id = ?1",
                params![media_id, entry.original, now],
            )?;
        }
        transaction.execute("DELETE FROM trash_entries WHERE id = ?1", [id])?;
        transaction.commit()
    });
    if let Err(error) = updated {
        let _ = fs::rename(on_disk(root, &entry.original), on_disk(root, &entry.trash));
        return Err(sql(error));
    }
    if let Some(slot) = slot_of(&entry.trash) {
        let _ = fs::remove_dir_all(on_disk(root, &slot));
    }
    Ok(())
}

fn collect_dirs(dir: &Path, relative: &str, found: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                let path = join(relative, name);
                collect_dirs(&entry.path(), &path, found);
                found.push(path);
            }
        }
    }
}

/// "Restaurar tudo".
pub fn restore_all(connection: &mut Connection, root: &Path, now: &str) -> Result<Outcome, String> {
    let ids: Vec<(i64, String)> = connection
        .prepare("SELECT id, original_relative_path FROM trash_entries ORDER BY id")
        .and_then(|mut statement| statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?.collect())
        .map_err(sql)?;
    let mut outcome = Outcome::default();
    for (id, original) in ids {
        match restore(connection, root, id, now) {
            Ok(()) => outcome.done += 1,
            Err(reason) => outcome.skipped.push((name_of(&original).to_string(), reason)),
        }
    }
    Ok(outcome)
}

/// AD-009: entries past 30 days are deleted for good, files, thumbnails and rows.
/// Returns how many entries went.
pub fn purge_expired(connection: &mut Connection, root: &Path, now: &str) -> rusqlite::Result<u64> {
    let due: Vec<i64> = connection
        .prepare("SELECT id FROM trash_entries WHERE purge_after <= ?1")?
        .query_map([now], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    let mut purged = 0;
    for id in due {
        let Ok(entry) = entry(connection, id) else { continue };
        let Some(slot) = slot_of(&entry.trash) else { continue };
        let slot = on_disk(root, &slot);
        if slot.exists() && fs::remove_dir_all(&slot).is_err() {
            continue;
        }
        let length = entry.original.chars().count() as i64;
        let gone: Vec<i64> = if entry.kind == "folder" {
            connection
                .prepare(
                    "SELECT id FROM media_items WHERE indexing_status = 'missing_file' AND updated_at = ?3
                       AND substr(relative_path, 1, ?2 + 1) = ?1 || '/' COLLATE NOCASE",
                )?
                .query_map(params![entry.original, length, entry.trashed_at], |row| row.get(0))?
                .collect::<Result<_, _>>()?
        } else {
            entry.media_id.into_iter().collect()
        };
        let transaction = connection.transaction()?;
        for media_id in &gone {
            transaction.execute("DELETE FROM media_items WHERE id = ?1", [media_id])?;
        }
        transaction.execute("DELETE FROM trash_entries WHERE id = ?1", [id])?;
        transaction.commit()?;
        for media_id in gone {
            for size in [ThumbSize::Small, ThumbSize::Medium, ThumbSize::Large] {
                let _ = fs::remove_file(thumb_path(root, media_id, size));
            }
        }
        purged += 1;
    }
    Ok(purged)
}

#[cfg(test)]
mod tests;
