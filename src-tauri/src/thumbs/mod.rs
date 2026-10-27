//! F12: JPEG thumbnails of the photos in three sizes, kept under `.myphotos\thumbs\` on
//! the library disk (AD-011). A queue serves what the UI asks for first and fills in the
//! rest newest first; the cache is held under a size limit by evicting the least
//! recently used. Reads the originals only (AD-008). F13 adds video covers (`video`).

#[cfg(test)]
mod tests;
#[cfg(test)]
mod video_tests;

mod queue;
mod render;
mod service;
pub mod video;

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;

use crate::db;
use crate::scan::absolute_path;
use crate::time::iso_utc;

pub(crate) use queue::{Job, Priority, ThumbQueue};
pub(crate) use render::render;
pub use service::ThumbService;
pub use video::{locate_ffmpeg, Ffmpeg, VideoProbe, VideoTool};

/// `app_settings` key for the cache limit in bytes (RF-058).
pub const CACHE_LIMIT_SETTING: &str = "thumbnail_cache_limit_bytes";

/// 4 GiB: every small thumbnail of a 100k-photo library plus room for the larger ones.
pub const DEFAULT_CACHE_LIMIT_BYTES: u64 = 4 * 1024 * 1024 * 1024;

const JPEG_QUALITY: u8 = 82;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ThumbSize {
    /// Timeline grid (RF-056).
    Small,
    /// Viewer while the original loads.
    Medium,
    /// Viewer full screen.
    Large,
}

impl ThumbSize {
    pub const ALL: [Self; 3] = [Self::Small, Self::Medium, Self::Large];

    /// Longest edge in pixels. Never upscaled past the original.
    pub fn max_edge(self) -> u32 {
        match self {
            Self::Small => 300,
            Self::Medium => 1280,
            Self::Large => 2560,
        }
    }

    pub fn as_column(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }

    pub fn from_column(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|size| size.as_column() == value)
    }
}

/// Why no thumbnail could be made; the values of `thumbnail_failures.reason`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbError {
    /// HEIC, RAW and other formats the decoder does not read.
    Unsupported,
    Corrupt,
    Unreadable,
}

impl ThumbError {
    pub fn as_column(self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported_format",
            Self::Corrupt => "corrupt_media",
            Self::Unreadable => "unreadable",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbSummary {
    pub queued: u64,
    pub generated: u64,
    pub unsupported: u64,
    pub corrupt: u64,
    pub unreadable: u64,
    /// The thumbnail file or its row could not be written; tried again next time.
    pub write_failed: u64,
    /// F13: of `generated`, video covers.
    pub videos: u64,
    /// F13: ffmpeg could not be run for a video; tried again next time.
    pub tool_failed: u64,
    /// F13: videos left out because no ffmpeg was found.
    pub videos_skipped: u64,
    /// F13: the folder ffmpeg was found in.
    pub ffmpeg_dir: Option<String>,
    pub evicted: u64,
    pub evicted_bytes: u64,
    pub cache_bytes: u64,
    pub cache_limit_bytes: u64,
    /// The background fill stopped because the cache reached its limit.
    pub stopped_cache_full: bool,
}

impl ThumbSummary {
    pub fn done(&self) -> u64 {
        self.generated
            + self.unsupported
            + self.corrupt
            + self.unreadable
            + self.write_failed
            + self.tool_failed
    }
}

/// `{root}\.myphotos\thumbs\small\2a\1066.jpg`: 256 folders per size so none grows huge.
pub fn thumb_path(root: &Path, media_id: i64, size: ThumbSize) -> PathBuf {
    root.join(".myphotos")
        .join("thumbs")
        .join(size.as_column())
        .join(format!("{:02x}", media_id.rem_euclid(256)))
        .join(format!("{media_id}.jpg"))
}

/// Which version of the original a thumbnail was made from. Orientation is in it
/// because the rotation is baked into the pixels.
pub(crate) fn fingerprint(
    size_bytes: Option<i64>,
    modified_at: Option<&str>,
    orientation: Option<i64>,
) -> String {
    format!(
        "{}|{}|{}",
        size_bytes.map(|value| value.to_string()).unwrap_or_default(),
        modified_at.unwrap_or_default(),
        orientation.map(|value| value.to_string()).unwrap_or_default()
    )
}

/// The cache limit from `app_settings`, or the default when unset or not a positive number.
pub fn cache_limit(connection: &Connection) -> rusqlite::Result<u64> {
    let value: Option<Option<String>> = connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [CACHE_LIMIT_SETTING],
            |row| row.get(0),
        )
        .optional()?;
    Ok(value
        .flatten()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .filter(|&limit| limit > 0)
        .unwrap_or(DEFAULT_CACHE_LIMIT_BYTES))
}

/// ffmpeg for video covers, unless "Gerar capa dos vídeos" is off in Settings (F31).
pub fn video_tool(connection: &Connection) -> Option<Ffmpeg> {
    match crate::settings::flag(connection, crate::settings::VIDEO_COVERS, true) {
        Ok(false) => None,
        _ => locate_ffmpeg(),
    }
}

pub fn cache_bytes(connection: &Connection) -> rusqlite::Result<u64> {
    connection.query_row("SELECT COALESCE(SUM(bytes), 0) FROM thumbnails", [], |row| {
        row.get::<_, i64>(0).map(|bytes| bytes.max(0) as u64)
    })
}

const MEDIA_COLUMNS: &str =
    "m.id, m.relative_path, m.orientation, m.taken_at, m.size_bytes, m.fs_modified_at, m.media_type";

fn job_from_row(row: &rusqlite::Row, size: ThumbSize, priority: Priority) -> rusqlite::Result<Job> {
    let orientation: Option<i64> = row.get(2)?;
    let size_bytes: Option<i64> = row.get(4)?;
    let modified_at: Option<String> = row.get(5)?;
    let media_type: String = row.get(6)?;
    Ok(Job {
        media_id: row.get(0)?,
        size,
        relative_path: row.get(1)?,
        orientation: orientation.and_then(|value| u16::try_from(value).ok()),
        fingerprint: fingerprint(size_bytes, modified_at.as_deref(), orientation),
        taken_at: row.get(3)?,
        video: media_type == "video",
        priority,
    })
}

/// Photos (and videos, when `videos`) of the library at `root` with no up-to-date `size`
/// thumbnail and no recorded failure for their current version, newest first.
pub(crate) fn pending(
    connection: &Connection,
    root: &Path,
    size: ThumbSize,
    videos: bool,
) -> rusqlite::Result<Vec<Job>> {
    let made = fingerprints(
        connection,
        "SELECT media_id, source_fingerprint FROM thumbnails WHERE size = ?1",
        Some(size.as_column()),
    )?;
    let failed = fingerprints(
        connection,
        "SELECT media_id, source_fingerprint FROM thumbnail_failures",
        None,
    )?;
    let mut statement = connection.prepare(&format!(
        "SELECT {MEDIA_COLUMNS} FROM media_items m
         WHERE (m.media_type = 'photo' OR (?2 AND m.media_type = 'video'))
           AND m.indexing_status != 'missing_file'
           AND m.library_id IN (SELECT id FROM libraries WHERE root_path = ?1 COLLATE NOCASE)
         ORDER BY m.taken_at DESC, m.id DESC"
    ))?;
    let jobs = statement
        .query_map(params![root.to_string_lossy(), videos], |row| {
            job_from_row(row, size, Priority::Background)
        })?
        .collect::<rusqlite::Result<Vec<Job>>>()?;
    Ok(jobs
        .into_iter()
        .filter(|job| {
            made.get(&job.media_id) != Some(&job.fingerprint)
                && failed.get(&job.media_id) != Some(&job.fingerprint)
        })
        .collect())
}

/// How many `pending` would return, without building the jobs (F32: the sidebar asks for
/// it on every summary; 0.5 s on 300k items the long way). The SQL fingerprint matches
/// `fingerprint`.
pub(crate) fn pending_count(connection: &Connection, root: &Path, size: ThumbSize, videos: bool) -> rusqlite::Result<u64> {
    let count: i64 = connection.query_row(
        "SELECT count(*) FROM (
           SELECT m.id, coalesce(CAST(m.size_bytes AS TEXT), '') || '|' || coalesce(m.fs_modified_at, '') || '|'
                    || coalesce(CAST(m.orientation AS TEXT), '') AS print
           FROM media_items m
           WHERE (m.media_type = 'photo' OR (?2 AND m.media_type = 'video'))
             AND m.indexing_status != 'missing_file'
             AND m.library_id IN (SELECT id FROM libraries WHERE root_path = ?1 COLLATE NOCASE)) p
         WHERE NOT EXISTS (SELECT 1 FROM thumbnails t WHERE t.media_id = p.id AND t.size = ?3 AND t.source_fingerprint = p.print)
           AND NOT EXISTS (SELECT 1 FROM thumbnail_failures f WHERE f.media_id = p.id AND f.source_fingerprint = p.print)",
        params![root.to_string_lossy(), videos, size.as_column()],
        |row| row.get(0),
    )?;
    Ok(count.max(0) as u64)
}

fn fingerprints(
    connection: &Connection,
    sql: &str,
    size: Option<&str>,
) -> rusqlite::Result<HashMap<i64, String>> {
    let mut statement = connection.prepare(sql)?;
    let map = |row: &rusqlite::Row| Ok((row.get(0)?, row.get(1)?));
    match size {
        Some(size) => statement.query_map([size], map)?.collect(),
        None => statement.query_map([], map)?.collect(),
    }
}

/// The job for one photo (or video, when `videos`) when its `size` thumbnail is missing
/// or stale; `None` when it is up to date, failed for this version or cannot be made.
pub(crate) fn job_for(
    connection: &Connection,
    media_id: i64,
    size: ThumbSize,
    videos: bool,
) -> rusqlite::Result<Option<Job>> {
    let job = connection
        .query_row(
            &format!(
                "SELECT {MEDIA_COLUMNS} FROM media_items m
                 WHERE m.id = ?1 AND (m.media_type = 'photo' OR (?2 AND m.media_type = 'video'))
                   AND m.indexing_status != 'missing_file'"
            ),
            params![media_id, videos],
            |row| job_from_row(row, size, Priority::Requested),
        )
        .optional()?;
    let Some(job) = job else {
        return Ok(None);
    };
    let settled: bool = connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM thumbnails
                        WHERE media_id = ?1 AND size = ?2 AND source_fingerprint = ?3)
             OR EXISTS (SELECT 1 FROM thumbnail_failures
                        WHERE media_id = ?1 AND source_fingerprint = ?3)",
        params![media_id, size.as_column(), job.fingerprint],
        |row| row.get(0),
    )?;
    Ok((!settled).then_some(job))
}

/// The thumbnail file for the UI when it is up to date (and marks it used for the LRU),
/// else `None`: ask the service to make it.
pub fn thumbnail_for(
    connection: &Connection,
    root: &Path,
    media_id: i64,
    size: ThumbSize,
    now: &str,
) -> rusqlite::Result<Option<PathBuf>> {
    if job_for(connection, media_id, size, true)?.is_some() {
        return Ok(None);
    }
    let path = thumb_path(root, media_id, size);
    let touched = connection.execute(
        "UPDATE thumbnails SET last_accessed_at = ?3 WHERE media_id = ?1 AND size = ?2",
        params![media_id, size.as_column(), now],
    )?;
    Ok((touched == 1 && path.is_file()).then_some(path))
}

/// Writes next to the final name and renames, so a crash never leaves half a JPEG.
pub(crate) fn write_thumb(path: &Path, jpeg: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let partial = path.with_extension("jpg.partial");
    fs::write(&partial, jpeg)?;
    fs::rename(&partial, path).inspect_err(|_| {
        let _ = fs::remove_file(&partial);
    })
}

pub(crate) fn record_done(
    connection: &Connection,
    job: &Job,
    width: u32,
    height: u32,
    bytes: u64,
    now: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT OR REPLACE INTO thumbnails
           (media_id, size, width, height, bytes, source_fingerprint, created_at, last_accessed_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![job.media_id, job.size.as_column(), width, height, bytes as i64, job.fingerprint, now],
    )?;
    connection.execute("DELETE FROM thumbnail_failures WHERE media_id = ?1", [job.media_id])?;
    Ok(())
}

/// F13: what ffprobe read goes to `media_items`; a value it could not read keeps the old one.
pub(crate) fn record_video(
    connection: &Connection,
    media_id: i64,
    probe: &VideoProbe,
) -> rusqlite::Result<()> {
    connection.execute(
        "UPDATE media_items SET duration_ms = COALESCE(?2, duration_ms),
           width = COALESCE(?3, width), height = COALESCE(?4, height)
         WHERE id = ?1 AND media_type = 'video'",
        params![
            media_id,
            probe.duration_ms.map(|value| value as i64),
            probe.width,
            probe.height
        ],
    )?;
    Ok(())
}

pub(crate) fn record_failure(
    connection: &Connection,
    job: &Job,
    error: ThumbError,
    now: &str,
) -> rusqlite::Result<()> {
    connection.execute(
        "INSERT OR REPLACE INTO thumbnail_failures (media_id, source_fingerprint, reason, created_at)
         VALUES (?1, ?2, ?3, ?4)",
        params![job.media_id, job.fingerprint, error.as_column(), now],
    )?;
    Ok(())
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Eviction {
    pub count: u64,
    pub bytes: u64,
    /// Total after evicting.
    pub cache_bytes: u64,
}

/// Brings the cache under `limit`: least recently used medium and large first, then
/// small only when `allow_small` (the UI asked for something). `keep` is never evicted.
pub(crate) fn evict(
    connection: &Connection,
    root: &Path,
    limit: u64,
    allow_small: bool,
    keep: &HashSet<(i64, ThumbSize)>,
) -> rusqlite::Result<Eviction> {
    let mut eviction = Eviction {
        cache_bytes: cache_bytes(connection)?,
        ..Eviction::default()
    };
    if eviction.cache_bytes <= limit {
        return Ok(eviction);
    }
    let candidates: Vec<(i64, String, i64)> = connection
        .prepare(
            "SELECT media_id, size, bytes FROM thumbnails
             ORDER BY size = 'small', last_accessed_at, media_id",
        )?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (media_id, size, bytes) in candidates {
        if eviction.cache_bytes <= limit {
            break;
        }
        let Some(size) = ThumbSize::from_column(&size) else {
            continue;
        };
        if (size == ThumbSize::Small && !allow_small) || keep.contains(&(media_id, size)) {
            continue;
        }
        match fs::remove_file(thumb_path(root, media_id, size)) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            // Still in use or protected: keep the row so the bytes stay counted.
            Err(_) => continue,
        }
        connection.execute(
            "DELETE FROM thumbnails WHERE media_id = ?1 AND size = ?2",
            params![media_id, size.as_column()],
        )?;
        let bytes = bytes.max(0) as u64;
        eviction.count += 1;
        eviction.bytes += bytes;
        eviction.cache_bytes = eviction.cache_bytes.saturating_sub(bytes);
    }
    Ok(eviction)
}

/// Worker threads: decoding is CPU-bound, and the library disk is the other limit.
pub fn default_workers() -> usize {
    std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(2)
        .clamp(1, 8)
}

/// Makes every missing `size` thumbnail of the library at `root`, newest first, and
/// hands the connection back. Videos too when ffmpeg is found (F13). When `stop()` says
/// so (F17 pause), the jobs in progress finish and the queued ones wait for next time.
pub fn generate_library(
    connection: Connection,
    root: &Path,
    size: ThumbSize,
    workers: usize,
    progress: &mut dyn FnMut(u64, u64),
    stop: &dyn Fn() -> bool,
) -> (Connection, Result<ThumbSummary, rusqlite::Error>) {
    let ffmpeg = video_tool(&connection);
    let dir = video::describe(ffmpeg.as_ref());
    let tool = ffmpeg.map(|tool| Arc::new(tool) as Arc<dyn VideoTool>);
    let (connection, result) = generate_with(connection, root, size, workers, tool, progress, stop);
    let result = result.map(|summary| ThumbSummary {
        ffmpeg_dir: dir,
        ..summary
    });
    (connection, result)
}

pub(crate) fn generate_with(
    connection: Connection,
    root: &Path,
    size: ThumbSize,
    workers: usize,
    video: Option<Arc<dyn VideoTool>>,
    progress: &mut dyn FnMut(u64, u64),
    stop: &dyn Fn() -> bool,
) -> (Connection, Result<ThumbSummary, rusqlite::Error>) {
    let prepared = pending(&connection, root, size, video.is_some()).and_then(|jobs| {
        let skipped = if video.is_some() {
            0
        } else {
            pending(&connection, root, size, true)?.len() - jobs.len()
        };
        Ok((jobs, skipped as u64, cache_limit(&connection)?))
    });
    let (jobs, skipped, limit) = match prepared {
        Ok(prepared) => prepared,
        Err(error) => return (connection, Err(error)),
    };
    let service = ThumbService::start_with(connection, root.to_path_buf(), workers, limit, video);
    service.enqueue(jobs);
    service.wait_idle_until(stop, progress);
    let (connection, summary) = service.shutdown();
    (
        connection,
        Ok(ThumbSummary {
            videos_skipped: skipped,
            ..summary
        }),
    )
}

/// Finds the library disk (F11) and makes its missing `size` thumbnails. Errors are codes.
pub fn generate_default_library(
    size: ThumbSize,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<ThumbSummary, &'static str> {
    let (root, connection) = db::open_located().map_err(|error| error.as_code())?;
    let (connection, result) = generate_library(connection, &root, size, default_workers(), progress, &|| false);
    db::close(connection);
    result.map_err(|_| "thumbnail_database_failed")
}

pub(crate) fn now() -> String {
    iso_utc(SystemTime::now()).unwrap_or_default()
}

pub(crate) fn original_path(root: &Path, job: &Job) -> PathBuf {
    absolute_path(root, &job.relative_path)
}
