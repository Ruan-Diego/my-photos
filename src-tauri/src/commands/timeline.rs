//! F18: what the timeline (artboard `Timeline`) lists, and the small thumbnails it shows,
//! served through the `thumb` protocol so the grid loads them as plain `<img>`.

use std::fs;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tauri::http::{header, Response, StatusCode};
use tauri::{Manager, State, UriSchemeContext, UriSchemeResponder, Wry};

use super::health::DbState;
use crate::thumbs::{now, thumbnail_for, ThumbSize};

/// The scheme of `http://thumb.localhost/1066` (Windows) or `thumb://localhost/1066`.
pub const THUMB_SCHEME: &str = "thumb";

/// Flag bits in [`Timeline::flags`].
pub const VIDEO: u8 = 1;
pub const FAVORITE: u8 = 2;

/// Every media the timeline shows, newest first, undated last. One array per field:
/// 300k items stay a few MB of JSON instead of 300k objects.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timeline {
    pub ids: Vec<i64>,
    /// Unix seconds (UTC); `None` when no date was found.
    pub times: Vec<Option<i64>>,
    /// Minutes east of UTC when the file said where it was taken; `None` uses the PC's.
    pub offsets: Vec<Option<i32>>,
    /// Width / height as shown (rotation applied), two decimals. 1 when unknown.
    pub ratios: Vec<f64>,
    /// Video length in ms, when known.
    pub durations: Vec<Option<i64>>,
    pub flags: Vec<u8>,
}

/// `-03:00` → -180. Anything else → `None`.
fn offset_minutes(zone: &str) -> Option<i32> {
    let (sign, rest) = match zone.as_bytes().first()? {
        b'+' => (1, &zone[1..]),
        b'-' => (-1, &zone[1..]),
        _ => return None,
    };
    let (hours, minutes) = rest.split_once(':')?;
    let hours: i32 = hours.parse().ok()?;
    let minutes: i32 = minutes.parse().ok()?;
    (hours <= 14 && minutes < 60).then_some(sign * (hours * 60 + minutes))
}

/// The small thumbnail has the rotation baked in; without one, EXIF orientations 5–8
/// swap the stored width and height.
fn ratio(thumb: Option<(i64, i64)>, stored: Option<(i64, i64)>, orientation: Option<i64>) -> f64 {
    let (width, height) = match (thumb, stored) {
        (Some(size), _) => size,
        (None, Some((width, height))) if matches!(orientation, Some(5..=8)) => (height, width),
        (None, Some(size)) => size,
        (None, None) => return 1.0,
    };
    if width <= 0 || height <= 0 {
        return 1.0;
    }
    ((width as f64 / height as f64) * 100.0).round() / 100.0
}

/// The same media the sidebar counts (F15): hidden and missing files left out.
pub fn timeline_items(connection: &Connection) -> rusqlite::Result<Timeline> {
    let mut statement = connection.prepare(
        "SELECT m.id, CAST(strftime('%s', substr(m.taken_at, 1, 19)) AS INTEGER), m.taken_at_timezone,
                t.width, t.height, m.width, m.height, m.orientation,
                m.media_type = 'video', m.duration_ms, m.is_favorite
         FROM media_items m
         LEFT JOIN thumbnails t ON t.media_id = m.id AND t.size = 'small'
         WHERE m.is_hidden = 0 AND m.indexing_status != 'missing_file'
         ORDER BY m.taken_at IS NULL, m.taken_at DESC, m.id DESC",
    )?;
    let mut rows = statement.query([])?;
    let mut timeline = Timeline::default();
    while let Some(row) = rows.next()? {
        let zone: Option<String> = row.get(2)?;
        let thumb = row.get::<_, Option<i64>>(3)?.zip(row.get::<_, Option<i64>>(4)?);
        let stored = row.get::<_, Option<i64>>(5)?.zip(row.get::<_, Option<i64>>(6)?);
        let video: bool = row.get(8)?;
        let favorite: bool = row.get(10)?;
        timeline.ids.push(row.get(0)?);
        timeline.times.push(row.get(1)?);
        timeline.offsets.push(zone.as_deref().and_then(offset_minutes));
        timeline.ratios.push(ratio(thumb, stored, row.get(7)?));
        timeline.durations.push(if video { row.get(9)? } else { None });
        timeline.flags.push(u8::from(video) * VIDEO | u8::from(favorite) * FAVORITE);
    }
    Ok(timeline)
}

#[tauri::command]
pub fn timeline(state: State<'_, DbState>) -> Result<Timeline, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    timeline_items(connection).map_err(|error| error.to_string())
}

/// The JPEG of one small thumbnail, when it is made and current.
pub(crate) fn small_thumbnail(connection: &Connection, media_id: i64) -> Option<Vec<u8>> {
    let root: String = connection
        .query_row(
            "SELECT l.root_path FROM media_items m JOIN libraries l ON l.id = m.library_id WHERE m.id = ?1",
            [media_id],
            |row| row.get(0),
        )
        .optional()
        .ok()??;
    let path = thumbnail_for(connection, Path::new(&root), media_id, ThumbSize::Small, &now()).ok()??;
    fs::read(path).ok()
}

/// `thumb` protocol: the path is the media id. 404 while there is no thumbnail (the
/// tile keeps its placeholder); off the UI thread, since the file is on the USB disk.
pub fn serve_thumbnail(context: UriSchemeContext<'_, Wry>, request: tauri::http::Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = context.app_handle().clone();
    let media_id = request.uri().path().trim_matches('/').parse::<i64>().ok();
    tauri::async_runtime::spawn_blocking(move || {
        let jpeg = media_id.and_then(|media_id| {
            let state = app.try_state::<DbState>()?;
            let slot = state.0.as_ref().ok()?;
            let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            small_thumbnail(guard.as_ref()?, media_id)
        });
        let response = match jpeg {
            Some(bytes) => Response::builder()
                .header(header::CONTENT_TYPE, "image/jpeg")
                .header(header::CACHE_CONTROL, "max-age=3600")
                .body(bytes),
            None => Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()),
        };
        responder.respond(response.expect("static response"));
    });
}

#[cfg(test)]
mod tests {
    use std::fs;

    use rusqlite::{params, Connection};

    use super::{offset_minutes, ratio, small_thumbnail, timeline_items, Timeline, FAVORITE, VIDEO};
    use crate::db::tests::Scratch;
    use crate::db::MIGRATIONS;
    use crate::thumbs::{thumb_path, ThumbSize};

    fn database(root: &str) -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        for migration in MIGRATIONS {
            connection.execute_batch(migration.sql).unwrap();
        }
        connection
            .execute("INSERT INTO libraries (name, root_path, created_at) VALUES ('E', ?1, 't')", [root])
            .unwrap();
        connection
    }

    #[allow(clippy::too_many_arguments)]
    fn add(
        connection: &Connection,
        id: i64,
        taken_at: Option<&str>,
        zone: Option<&str>,
        kind: &str,
        size: Option<(i64, i64)>,
        orientation: Option<i64>,
        duration_ms: Option<i64>,
        favorite: bool,
        hidden: bool,
        status: &str,
    ) {
        connection
            .execute(
                "INSERT INTO media_items (id, library_id, relative_path, filename, extension, media_type,
                   width, height, orientation, duration_ms, taken_at, taken_at_timezone, is_favorite,
                   is_hidden, indexing_status, created_at, updated_at)
                 VALUES (?1, 1, ?2, ?2, 'jpg', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 't', 't')",
                params![
                    id,
                    format!("{id}.jpg"),
                    kind,
                    size.map(|size| size.0),
                    size.map(|size| size.1),
                    orientation,
                    duration_ms,
                    taken_at,
                    zone,
                    favorite,
                    hidden,
                    status
                ],
            )
            .unwrap();
    }

    #[test]
    fn tl01_newest_first_undated_last_hidden_and_missing_left_out() {
        let connection = database("E:\\");
        add(&connection, 1, Some("2022-08-10T14:37:00.000Z"), Some("+01:00"), "photo", Some((4000, 3000)), None, None, true, false, "ok");
        add(&connection, 2, Some("2022-08-11T09:00:00Z"), None, "video", Some((1920, 1080)), None, Some(138_000), false, false, "ok");
        add(&connection, 3, None, None, "photo", None, None, None, false, false, "ok");
        add(&connection, 4, Some("2023-01-01T00:00:00Z"), None, "photo", None, None, None, false, true, "ok");
        add(&connection, 5, Some("2023-01-02T00:00:00Z"), None, "photo", None, None, None, false, false, "missing_file");
        add(&connection, 6, Some("2021-07-23T18:00:00Z"), None, "photo", Some((4000, 3000)), Some(6), None, false, false, "error");

        assert_eq!(
            timeline_items(&connection).unwrap(),
            Timeline {
                ids: vec![2, 1, 6, 3],
                times: vec![Some(1_660_208_400), Some(1_660_142_220), Some(1_627_063_200), None],
                offsets: vec![None, Some(60), None, None],
                ratios: vec![1.78, 1.33, 0.75, 1.0],
                durations: vec![Some(138_000), None, None, None],
                flags: vec![VIDEO, FAVORITE, 0, 0],
            }
        );
    }

    #[test]
    fn tl02_the_thumbnail_size_wins_over_the_stored_one() {
        let connection = database("E:\\");
        add(&connection, 1, Some("2022-08-10T14:37:00Z"), None, "photo", Some((4000, 3000)), Some(6), None, false, false, "ok");
        connection
            .execute(
                "INSERT INTO thumbnails (media_id, size, width, height, bytes, source_fingerprint, created_at, last_accessed_at)
                 VALUES (1, 'small', 300, 200, 1, 'f', 't', 't')",
                [],
            )
            .unwrap();
        assert_eq!(timeline_items(&connection).unwrap().ratios, [1.5]);
        assert_eq!(ratio(None, Some((0, 10)), None), 1.0);
    }

    #[test]
    fn tl03_offsets_only_from_well_formed_zones() {
        assert_eq!(offset_minutes("-03:00"), Some(-180));
        assert_eq!(offset_minutes("+05:30"), Some(330));
        assert_eq!(offset_minutes("Z"), None);
        assert_eq!(offset_minutes("-3"), None);
        assert_eq!(offset_minutes("+99:00"), None);
    }

    #[test]
    fn tl04_serves_a_current_small_thumbnail_and_nothing_else() {
        let scratch = Scratch::new("f18-thumb");
        let root = scratch.path.to_string_lossy().to_string();
        let connection = database(&root);
        add(&connection, 7, Some("2022-08-10T14:37:00Z"), None, "photo", None, None, None, false, false, "ok");
        assert_eq!(small_thumbnail(&connection, 7), None, "not made yet");

        let fingerprint: String = connection
            .query_row("SELECT '|' || '|'", [], |row| row.get(0))
            .unwrap();
        connection
            .execute(
                "INSERT INTO thumbnails (media_id, size, width, height, bytes, source_fingerprint, created_at, last_accessed_at)
                 VALUES (7, 'small', 3, 2, 4, ?1, 't', 't')",
                [fingerprint],
            )
            .unwrap();
        let path = thumb_path(&scratch.path, 7, ThumbSize::Small);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"jpeg").unwrap();

        assert_eq!(small_thumbnail(&connection, 7).as_deref(), Some(&b"jpeg"[..]));
        assert_eq!(small_thumbnail(&connection, 8), None, "unknown media");
        let touched: String = connection
            .query_row("SELECT last_accessed_at FROM thumbnails WHERE media_id = 7", [], |row| row.get(0))
            .unwrap();
        assert_ne!(touched, "t", "the LRU sees the timeline use it");
    }
}
