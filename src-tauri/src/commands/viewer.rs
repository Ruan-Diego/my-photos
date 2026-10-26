//! F19: the viewer (artboard `Viewer`) shows the small thumbnail at once and then the
//! original, served through the `media` protocol. F20: videos play from the same
//! protocol in byte ranges, so seeking never reads the whole file. Read only (AD-008).

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom};
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

/// Containers the WebView plays (WebView2 = Chromium). MKV, AVI and 3GP do not.
fn video_type(extension: &str) -> Option<&'static str> {
    match extension.to_ascii_lowercase().as_str() {
        "mp4" | "m4v" => Some("video/mp4"),
        "mov" => Some("video/quicktime"),
        "webm" => Some("video/webm"),
        _ => None,
    }
}

/// Most a single range answer carries: the player asks again for the rest.
const MAX_RANGE: u64 = 4 * 1024 * 1024;

/// `bytes=100-199`, `bytes=100-`, `bytes=-500` → first and last byte, capped at
/// [`MAX_RANGE`]. `None` when it cannot be satisfied.
pub(crate) fn parse_range(header: &str, length: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?.split(',').next()?.trim();
    let (first, last) = spec.split_once('-')?;
    if length == 0 {
        return None;
    }
    let (start, end) = if first.is_empty() {
        let suffix: u64 = last.parse().ok()?;
        (length.saturating_sub(suffix), length - 1)
    } else {
        let start: u64 = first.parse().ok()?;
        let end = if last.is_empty() { length - 1 } else { last.parse::<u64>().ok()?.min(length - 1) };
        (start, end)
    };
    if start > end || start >= length {
        return None;
    }
    Some((start, end.min(start + MAX_RANGE - 1)))
}

fn read_range(path: &Path, start: u64, end: u64) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::with_capacity((end - start + 1) as usize);
    file.take(end - start + 1).read_to_end(&mut bytes)?;
    Ok(bytes)
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
    /// A video the app can play (F20).
    pub playable: bool,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub size_bytes: Option<i64>,
}

struct Original {
    path: PathBuf,
    content_type: &'static str,
    video: bool,
}

pub fn media_info(connection: &Connection, media_id: i64) -> rusqlite::Result<Option<MediaInfo>> {
    connection
        .query_row(
            "SELECT id, filename, relative_path, extension, media_type = 'video', width, height, size_bytes
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
                    playable: video && video_type(&extension).is_some(),
                    extension,
                    video,
                    width: row.get(5)?,
                    height: row.get(6)?,
                    size_bytes: row.get(7)?,
                })
            },
        )
        .optional()
}

/// Where the original of a viewable photo or playable video is, and how to label it.
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
    let content_type = if video { video_type(&extension)? } else { content_type(&extension)? };
    Some(Original {
        path: absolute_path(Path::new(&root), &relative),
        content_type,
        video,
    })
}

/// The answer for one request: the whole photo, or the asked range of a video
/// (a video asked without a range gets its first range, as `bytes=0-`).
fn answer(original: &Original, range: Option<&str>) -> Option<Response<Vec<u8>>> {
    if !original.video {
        let bytes = fs::read(&original.path).ok()?;
        return Response::builder()
            .header(header::CONTENT_TYPE, original.content_type)
            .header(header::CACHE_CONTROL, "max-age=600")
            .body(bytes)
            .ok();
    }
    let length = fs::metadata(&original.path).ok()?.len();
    let Some((start, end)) = parse_range(range.unwrap_or("bytes=0-"), length) else {
        return Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(header::CONTENT_RANGE, format!("bytes */{length}"))
            .body(Vec::new())
            .ok();
    };
    let bytes = read_range(&original.path, start, end).ok()?;
    Response::builder()
        .status(StatusCode::PARTIAL_CONTENT)
        .header(header::CONTENT_TYPE, original.content_type)
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{length}"))
        .header(header::CONTENT_LENGTH, bytes.len())
        .body(bytes)
        .ok()
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
    let range = request
        .headers()
        .get(header::RANGE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    tauri::async_runtime::spawn_blocking(move || {
        let found = media_id.and_then(|media_id| {
            let state = app.try_state::<DbState>()?;
            let slot = state.0.as_ref().ok()?;
            let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            original(guard.as_ref()?, media_id)
        });
        let response = found
            .and_then(|original| answer(&original, range.as_deref()))
            .unwrap_or_else(|| {
                Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Vec::new())
                    .expect("static response")
            });
        responder.respond(response);
    });
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use std::fs;

    use super::{answer, media_info, original, parse_range, MediaInfo, MAX_RANGE};
    use crate::db::tests::Scratch;
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
                playable: false,
                width: Some(4032),
                height: Some(3024),
                size_bytes: None,
            })
        );
        assert!(!media_info(&connection, 2).unwrap().unwrap().viewable, "HEIC");
        let video = media_info(&connection, 3).unwrap().unwrap();
        assert!(!video.viewable && video.playable, "MP4 plays (F20)");
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
        assert_eq!(original(&connection, 3).unwrap().content_type, "video/mp4");
        assert!(original(&connection, 4).is_none());
    }

    #[test]
    fn video01_ranges_are_parsed_and_capped() {
        assert_eq!(parse_range("bytes=0-", 1000), Some((0, 999)));
        assert_eq!(parse_range("bytes=100-199", 1000), Some((100, 199)));
        assert_eq!(parse_range("bytes=900-5000", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=-100", 1000), Some((900, 999)));
        assert_eq!(parse_range("bytes=1000-", 1000), None);
        assert_eq!(parse_range("bytes=5-2", 1000), None);
        assert_eq!(parse_range("items=0-", 1000), None);
        assert_eq!(parse_range("bytes=0-", 0), None);
        let big = 100 * 1024 * 1024;
        assert_eq!(parse_range("bytes=0-", big), Some((0, MAX_RANGE - 1)), "one chunk at a time");
    }

    #[test]
    fn video02_a_range_answer_carries_only_that_slice() {
        let scratch = Scratch::new("f20-range");
        let path = scratch.path.join("VID.mp4");
        fs::write(&path, (0..=255u8).collect::<Vec<_>>()).unwrap();
        let video = super::Original { path: path.clone(), content_type: "video/mp4", video: true };

        let response = answer(&video, Some("bytes=10-19")).unwrap();
        assert_eq!(response.status(), 206);
        assert_eq!(response.headers()["content-range"], "bytes 10-19/256");
        assert_eq!(response.headers()["accept-ranges"], "bytes");
        assert_eq!(response.body(), &(10..=19u8).collect::<Vec<_>>());

        let first = answer(&video, None).unwrap();
        assert_eq!(first.status(), 206);
        assert_eq!(first.body().len(), 256);

        let past = answer(&video, Some("bytes=300-")).unwrap();
        assert_eq!(past.status(), 416);
        assert_eq!(past.headers()["content-range"], "bytes */256");

        let photo = super::Original { path, content_type: "image/jpeg", video: false };
        let whole = answer(&photo, Some("bytes=10-19")).unwrap();
        assert_eq!(whole.status(), 200);
        assert_eq!(whole.body().len(), 256);
    }
}
