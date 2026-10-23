//! F19: the viewer (artboard `Viewer`) shows the small thumbnail at once and then the
//! original, served through the `media` protocol. Read only (AD-008).

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tauri::http::{header, Response, StatusCode};
use tauri::{Manager, State, UriSchemeContext, UriSchemeResponder, Wry};

use super::health::DbState;
use crate::scan::absolute_path;

/// The scheme of `http://media.localhost/1066` (Windows) or `media://localhost/1066`.
pub const MEDIA_SCHEME: &str = "media";

/// Photos the WebView draws itself. HEIC and RAW stay on the thumbnail.
fn content_type(extension: &str) -> Option<&'static str> {
    match extension.to_ascii_lowercase().as_str() {
        "jpg" | "jpeg" => Some("image/jpeg"),
        "png" => Some("image/png"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub id: i64,
    pub filename: String,
    pub relative_path: String,
    /// `jpg`, `heic`, `mp4`…
    pub extension: String,
    pub video: bool,
    /// The original can be shown in the app (`media` protocol); else only the thumbnail.
    pub viewable: bool,
    pub width: Option<i64>,
    pub height: Option<i64>,
}

struct Original {
    path: PathBuf,
    content_type: &'static str,
}

pub fn media_info(connection: &Connection, media_id: i64) -> rusqlite::Result<Option<MediaInfo>> {
    connection
        .query_row(
            "SELECT id, filename, relative_path, extension, media_type = 'video', width, height
             FROM media_items WHERE id = ?1 AND indexing_status != 'missing_file'",
            [media_id],
            |row| {
                let extension: String = row.get(3)?;
                let video: bool = row.get(4)?;
                Ok(MediaInfo {
                    id: row.get(0)?,
                    filename: row.get(1)?,
                    relative_path: row.get(2)?,
                    viewable: !video && content_type(&extension).is_some(),
                    extension,
                    video,
                    width: row.get(5)?,
                    height: row.get(6)?,
                })
            },
        )
        .optional()
}

/// Where the original of a viewable photo is, and how to label it.
fn original(connection: &Connection, media_id: i64) -> Option<Original> {
    let (root, relative, extension, video): (String, String, String, bool) = connection
        .query_row(
            "SELECT l.root_path, m.relative_path, m.extension, m.media_type = 'video'
             FROM media_items m JOIN libraries l ON l.id = m.library_id
             WHERE m.id = ?1 AND m.indexing_status != 'missing_file'",
            [media_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .optional()
        .ok()??;
    if video {
        return None;
    }
    Some(Original {
        path: absolute_path(Path::new(&root), &relative),
        content_type: content_type(&extension)?,
    })
}

#[tauri::command]
pub fn media_details(state: State<'_, DbState>, id: i64) -> Result<Option<MediaInfo>, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    media_info(connection, id).map_err(|error| error.to_string())
}

/// `media` protocol: the path is the media id; the body is the original file. The
/// database lock is released before the read, which can take a while on a USB disk.
pub fn serve_original(context: UriSchemeContext<'_, Wry>, request: tauri::http::Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = context.app_handle().clone();
    let media_id = request.uri().path().trim_matches('/').parse::<i64>().ok();
    tauri::async_runtime::spawn_blocking(move || {
        let found = media_id.and_then(|media_id| {
            let state = app.try_state::<DbState>()?;
            let slot = state.0.as_ref().ok()?;
            let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            original(guard.as_ref()?, media_id)
        });
        let response = match found.and_then(|original| Some((fs::read(&original.path).ok()?, original.content_type))) {
            Some((bytes, kind)) => Response::builder()
                .header(header::CONTENT_TYPE, kind)
                .header(header::CACHE_CONTROL, "max-age=600")
                .body(bytes),
            None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
        };
        responder.respond(response.expect("static response"));
    });
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::{media_info, original, MediaInfo};
    use crate::db::MIGRATIONS;

    fn database() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        for migration in MIGRATIONS {
            connection.execute_batch(migration.sql).unwrap();
        }
        connection
            .execute("INSERT INTO libraries (name, root_path, created_at) VALUES ('E', 'E:\\', 't')", [])
            .unwrap();
        for (id, path, extension, kind, status) in [
            (1, "Takeout/Google Fotos/Viagem/IMG_1.JPG", "jpg", "photo", "ok"),
            (2, "Ruan/IMG_2.heic", "heic", "photo", "ok"),
            (3, "Ruan/VID_3.mp4", "mp4", "video", "ok"),
            (4, "Ruan/gone.jpg", "jpg", "photo", "missing_file"),
        ] {
            let filename = path.rsplit('/').next().unwrap();
            connection
                .execute(
                    "INSERT INTO media_items (id, library_id, relative_path, filename, extension, media_type,
                       width, height, indexing_status, created_at, updated_at)
                     VALUES (?1, 1, ?2, ?3, ?4, ?5, 4032, 3024, ?6, 't', 't')",
                    params![id, path, filename, extension, kind, status],
                )
                .unwrap();
        }
        connection
    }

    #[test]
    fn view01_only_photos_the_webview_draws_are_viewable() {
        let connection = database();
        assert_eq!(
            media_info(&connection, 1).unwrap(),
            Some(MediaInfo {
                id: 1,
                filename: "IMG_1.JPG".into(),
                relative_path: "Takeout/Google Fotos/Viagem/IMG_1.JPG".into(),
                extension: "jpg".into(),
                video: false,
                viewable: true,
                width: Some(4032),
                height: Some(3024),
            })
        );
        assert!(!media_info(&connection, 2).unwrap().unwrap().viewable, "HEIC");
        assert!(!media_info(&connection, 3).unwrap().unwrap().viewable, "video: F20");
        assert_eq!(media_info(&connection, 4).unwrap(), None, "missing file");
    }

    #[test]
    fn view02_the_original_is_read_from_the_library_root() {
        let connection = database();
        let found = original(&connection, 1).unwrap();
        let expected: std::path::PathBuf = ["E:\\", "Takeout", "Google Fotos", "Viagem", "IMG_1.JPG"].iter().collect();
        assert_eq!(found.path, expected);
        assert_eq!(found.content_type, "image/jpeg");
        assert!(original(&connection, 2).is_none());
        assert!(original(&connection, 3).is_none());
        assert!(original(&connection, 4).is_none());
    }
}
