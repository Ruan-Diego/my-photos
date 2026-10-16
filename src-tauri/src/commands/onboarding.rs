//! F16: what the welcome screen shows (artboard `Main`) and the indexing run.
//! F17: pause, the thumbnails that just arrived (artboard `Indexing`).

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use super::health::DbState;
use crate::index::{index_and_thumbnail_default_library, IndexSummary, Progress};
use crate::takeout::takeout_export;
use crate::thumbs::{thumb_path, ThumbSize};
use crate::volume::{locate_library, volume_info};

pub const PROGRESS_EVENT: &str = "indexing://progress";
pub const FINISHED_EVENT: &str = "indexing://finished";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFolder {
    /// `E:\`
    pub root_path: String,
    pub volume_label: Option<String>,
    /// Takeout exports at the root or one folder down (`E:\Ruan\takeout-…`):
    /// `takeout-<stamp>-N-NNN` parts of one export count once; a plain `Takeout` folder
    /// counts as one.
    pub takeout_exports: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexingFinished {
    pub summary: Option<IndexSummary>,
    /// Error code from F05/F10/F11 when the run failed.
    pub error: Option<String>,
}

/// One run at a time; `stop` pauses it (F17).
#[derive(Default)]
pub struct IndexingState {
    running: AtomicBool,
    stop: Arc<AtomicBool>,
}

pub(crate) fn count_exports<'a>(names: impl IntoIterator<Item = &'a str>) -> u32 {
    let mut exports: Vec<String> = Vec::new();
    for name in names {
        let export = if name.eq_ignore_ascii_case("takeout") {
            "takeout".to_string()
        } else if let Some(base) = takeout_export(name) {
            base.to_ascii_lowercase()
        } else {
            continue;
        };
        if !exports.contains(&export) {
            exports.push(export);
        }
    }
    exports.len() as u32
}

fn folder_names(path: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Folders at the root and one level down, skipping the app's and Windows' own.
fn exports_at(root: &Path) -> u32 {
    let mut names = Vec::new();
    for name in folder_names(root) {
        if name.starts_with('.') || name.starts_with('$') || name.eq_ignore_ascii_case("System Volume Information") {
            continue;
        }
        if takeout_export(&name).is_none() && !name.eq_ignore_ascii_case("takeout") {
            names.extend(folder_names(&root.join(&name)));
        }
        names.push(name);
    }
    count_exports(names.iter().map(String::as_str))
}

#[tauri::command]
pub fn library_folder() -> Result<LibraryFolder, String> {
    let root = locate_library().map_err(|error| error.message().to_string())?;
    let path = Path::new(&root);
    Ok(LibraryFolder {
        volume_label: volume_info(path).and_then(|volume| volume.label),
        takeout_exports: exports_at(path),
        root_path: root,
    })
}

/// Starts F10 on the library disk in the background, then the small thumbnails (F17).
/// Progress and the end arrive as events; a second call while one runs is refused.
#[tauri::command]
pub fn start_indexing(app: AppHandle, state: State<'_, IndexingState>) -> Result<(), String> {
    if state.running.swap(true, Ordering::SeqCst) {
        return Err("a indexação já está em andamento".into());
    }
    state.stop.store(false, Ordering::SeqCst);
    let stop = Arc::clone(&state.stop);
    thread::spawn(move || {
        let result = index_and_thumbnail_default_library(
            &mut |progress: Progress| {
                let _ = app.emit(PROGRESS_EVENT, progress);
            },
            &|| stop.load(Ordering::SeqCst),
        );
        let finished = match result {
            Ok(summary) => IndexingFinished {
                summary: Some(summary),
                error: None,
            },
            Err(code) => IndexingFinished {
                summary: None,
                error: Some(code.to_string()),
            },
        };
        if let Some(state) = tauri::Manager::try_state::<IndexingState>(&app) {
            state.running.store(false, Ordering::SeqCst);
        }
        let _ = app.emit(FINISHED_EVENT, finished);
    });
    Ok(())
}

/// F17 "Pausar": the run stops after the file or batch in progress and ends with
/// `cancelled`. What was written stays; the next run picks up from there.
#[tauri::command]
pub fn pause_indexing(state: State<'_, IndexingState>) {
    if state.running.load(Ordering::SeqCst) {
        state.stop.store(true, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentThumbnail {
    pub media_id: i64,
    pub relative_path: String,
    /// `data:image/jpeg;base64,…`
    pub src: String,
}

/// The newest small thumbnails rows, newest first: `(media_id, relative_path, root_path)`.
pub(crate) fn newest_thumbnails(connection: &Connection, limit: u32) -> rusqlite::Result<Vec<(i64, String, String)>> {
    let mut statement = connection.prepare(
        "SELECT t.media_id, m.relative_path, l.root_path FROM thumbnails t
         JOIN media_items m ON m.id = t.media_id JOIN libraries l ON l.id = m.library_id
         WHERE t.size = 'small' ORDER BY t.created_at DESC, t.media_id DESC LIMIT ?1",
    )?;
    let rows = statement.query_map([limit], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
    rows.collect()
}

/// F17 "Chegando agora": the last small thumbnails made, as data URLs.
#[tauri::command]
pub fn recent_thumbnails(state: State<'_, DbState>, limit: u32) -> Vec<RecentThumbnail> {
    let Ok(slot) = &state.0 else {
        return Vec::new();
    };
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(connection) = guard.as_ref() else {
        return Vec::new();
    };
    let rows = newest_thumbnails(connection, limit.min(48)).unwrap_or_default();
    drop(guard);
    rows.into_iter()
        .filter_map(|(media_id, relative_path, root)| {
            let bytes = fs::read(thumb_path(Path::new(&root), media_id, ThumbSize::Small)).ok()?;
            Some(RecentThumbnail {
                media_id,
                relative_path,
                src: format!("data:image/jpeg;base64,{}", base64(&bytes)),
            })
        })
        .collect()
}

pub(crate) fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for (index, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            if index <= chunk.len() {
                out.push(TABLE[(n >> shift & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::{params, Connection};

    use super::{base64, count_exports, exports_at, newest_thumbnails};
    use crate::db::tests::Scratch;
    use crate::db::MIGRATIONS;

    #[test]
    fn onb01_parts_of_one_export_count_once() {
        assert_eq!(
            count_exports([
                "takeout-20260921T183943Z-1-001",
                "takeout-20260921T183943Z-1-002",
                "Takeout-20250101T000000Z-1-001",
                "Takeout",
                "Fotos antigas",
                "takeout-sem-numero",
                "$RECYCLE.BIN",
            ]),
            3
        );
        assert_eq!(count_exports(["takeout", "TAKEOUT"]), 1);
        assert_eq!(count_exports([]), 0);
    }

    #[test]
    fn onb02_exports_one_folder_down_count_too() {
        let scratch = Scratch::new("onb02");
        let root = scratch.path.join("root");
        for folder in [
            "Ruan/takeout-20260921T183943Z-1-001",
            "Ruan/takeout-20260921T183943Z-1-002",
            "Ruan/Fotos soltas",
            "takeout-20250101T000000Z-1-001",
            ".myphotos/takeout-20990101T000000Z-1-001",
            "Outros/Fundo/takeout-20240101T000000Z-1-001",
        ] {
            fs::create_dir_all(root.join(folder)).unwrap();
        }
        // Two levels down is too deep; the app's folder is never looked at.
        assert_eq!(exports_at(&root), 2);
    }

    #[test]
    fn prog01_base64_matches_rfc_4648() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[test]
    fn prog02_newest_small_thumbnails_first() {
        let connection = Connection::open_in_memory().unwrap();
        for migration in MIGRATIONS {
            connection.execute_batch(migration.sql).unwrap();
        }
        connection
            .execute(
                "INSERT INTO libraries (name, root_path, created_at) VALUES ('E', 'E:\\', 't')",
                [],
            )
            .unwrap();
        for (id, name) in [(1, "a.jpg"), (2, "b.jpg"), (3, "c.jpg")] {
            connection
                .execute(
                    "INSERT INTO media_items (id, library_id, relative_path, filename, extension, media_type,
                       created_at, updated_at) VALUES (?1, 1, ?2, ?2, 'jpg', 'photo', 't', 't')",
                    params![id, name],
                )
                .unwrap();
        }
        for (id, size, at) in [(1, "small", "2026-10-15T10:00:00Z"), (2, "small", "2026-10-15T10:00:02Z"), (3, "medium", "2026-10-15T10:00:09Z")] {
            connection
                .execute(
                    "INSERT INTO thumbnails (media_id, size, width, height, bytes, source_fingerprint, created_at, last_accessed_at)
                     VALUES (?1, ?2, 1, 1, 1, 'f', ?3, ?3)",
                    params![id, size, at],
                )
                .unwrap();
        }
        let rows = newest_thumbnails(&connection, 18).unwrap();
        assert_eq!(
            rows,
            [(2, "b.jpg".to_string(), "E:\\".to_string()), (1, "a.jpg".to_string(), "E:\\".to_string())]
        );
    }
}
