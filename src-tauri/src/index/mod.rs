//! F10: one indexing run. F05 scans and hashes, F02/F06/F08 match and parse the Takeout
//! JSON, F07 reads EXIF, F09 resolves, and the result is written to the database.
//! Reads the library only; every write goes to the database (AD-008, AD-011).

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::SystemTime;

use rusqlite::{params, Connection, Transaction};
use serde::Serialize;

use crate::db;
use crate::photo_meta::{read_embedded, EmbeddedError, EmbeddedMetadata};
use crate::resolve::{resolve, DateSource, FsTimes, ResolvedMetadata, DEFAULT_FALLBACK_OFFSET_MINUTES};
use crate::scan::{
    check_root, finish_run, insert_error, scan_run, start_run, FileSystem, HashProgress, RealFs,
    ScanError, ScanSummary,
};
use crate::takeout::{
    discover, takeout_export, normalized_person_name, Confidence, DiscoveryReport, MediaHit, MediaMetadata,
    ProblemKind,
};
use crate::time::iso_utc;

/// Metadata results are committed in batches this size; a crash loses at most one batch.
const RESOLVE_BATCH: usize = 100;

/// Never indexed: the app's own folder (AD-011).
const APP_FOLDER: &str = ".myphotos";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    /// Walking the disk and reconciling with the database.
    Scanning,
    /// F17: reading and hashing new or changed files; `done`/`total` are files.
    Hashing,
    /// Reading and pairing the Takeout JSON.
    Matching,
    /// EXIF, F09 and the writes; `done`/`total` are media.
    Resolving,
    /// F17: small thumbnails (F12/F13); `done`/`total` are thumbnails.
    Thumbnails,
}

/// F17: the counters on the progress screen (artboard `Indexing`), for the whole library.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub photos: u64,
    pub videos: u64,
    pub date_from_json: u64,
    pub date_from_exif: u64,
    /// The sidecar was found in another part (`-1-002`) of the export (F08).
    pub json_in_other_part: u64,
    pub people: u64,
    /// Rows written to `indexing_errors` in this run.
    pub problems: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RecentKind {
    /// Paired with its Takeout JSON.
    Paired,
    /// No JSON: the date came from EXIF or the file.
    NoJson,
}

/// F17: one line of the log under "Chegando agora".
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recent {
    pub kind: RecentKind,
    pub relative_path: String,
    /// The sidecar's file name when paired.
    pub json_name: Option<String>,
    /// `exif`, `filesystem` or `None` when no date was found; only for `NoJson`.
    pub date_source: Option<String>,
}

/// Sent before each stage, after each hashed file and after each resolved batch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub stage: Stage,
    pub done: u64,
    pub total: u64,
    /// Hashing only: bytes read of the bytes to read.
    pub bytes_done: u64,
    pub bytes_total: u64,
    pub counts: Counts,
    /// The last media of the batch just written, oldest first.
    pub recent: Vec<Recent>,
}

/// Log lines sent with each resolved batch.
const RECENT: usize = 3;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexSummary {
    pub scan: ScanSummary,
    /// Media whose metadata was (re)resolved and written in this run.
    pub resolved: u64,
    pub date_from_json: u64,
    pub date_from_exif: u64,
    /// EXIF time with no zone; `DEFAULT_FALLBACK_OFFSET_MINUTES` applied (AD-012).
    pub date_offset_assumed: u64,
    pub date_from_filesystem: u64,
    pub no_date: u64,
    pub with_location: u64,
    pub with_json: u64,
}

/// What F10 needs from the outside world; tests swap in fakes.
pub(crate) struct Sources<'a> {
    pub fs: &'a dyn FileSystem,
    pub discover: &'a dyn Fn(&Path) -> Result<DiscoveryReport, String>,
    pub read_exif: &'a dyn Fn(&Path) -> Result<EmbeddedMetadata, EmbeddedError>,
    /// F17: the time the run ends, read when it ends.
    pub clock: &'a dyn Fn() -> String,
    /// F17: pause; checked between files and batches.
    pub stop: &'a dyn Fn() -> bool,
}

fn utc_now() -> String {
    iso_utc(SystemTime::now()).unwrap_or_default()
}

pub fn index_library(
    connection: &mut Connection,
    root: &Path,
    progress: &mut dyn FnMut(Progress),
    stop: &dyn Fn() -> bool,
) -> Result<IndexSummary, ScanError> {
    let now = utc_now();
    let sources = Sources {
        fs: &RealFs,
        discover: &discover,
        read_exif: &read_embedded,
        clock: &utc_now,
        stop,
    };
    index_with(connection, root, &sources, &now, progress)
}

/// Finds the library disk (F11), indexes it into its `.myphotos\library.db` and closes it. Errors are codes.
pub fn index_default_library(
    progress: &mut dyn FnMut(Progress),
) -> Result<IndexSummary, &'static str> {
    let (root, mut connection) = db::open_located().map_err(|error| error.as_code())?;
    let result = index_library(&mut connection, &root, progress, &|| false);
    db::close(connection);
    result.map_err(|error| error.as_code())
}

/// F17: the app's run. Indexes the library disk, then makes the missing small
/// thumbnails (F12/F13) with the last counters. `stop()` pauses either step; a paused
/// run returns `cancelled`.
pub fn index_and_thumbnail_default_library(
    progress: &mut dyn FnMut(Progress),
    stop: &dyn Fn() -> bool,
) -> Result<IndexSummary, &'static str> {
    let (root, mut connection) = db::open_located().map_err(|error| error.as_code())?;
    let mut last = Counts::default();
    let result = index_library(
        &mut connection,
        &root,
        &mut |update: Progress| {
            last = update.counts.clone();
            progress(update);
        },
        stop,
    );
    let connection = if result.is_ok() {
        let mut on_thumb = |done: u64, total: u64| {
            progress(Progress {
                stage: Stage::Thumbnails,
                done,
                total,
                bytes_done: 0,
                bytes_total: 0,
                counts: last.clone(),
                recent: Vec::new(),
            })
        };
        // A thumbnail that fails is recorded or retried next run; the index stands.
        let workers = crate::thumbs::default_workers();
        crate::thumbs::generate_library(connection, &root, crate::thumbs::ThumbSize::Small, workers, &mut on_thumb, stop).0
    } else {
        connection
    };
    db::close(connection);
    let summary = result.map_err(|error| error.as_code())?;
    if stop() {
        return Err(ScanError::Cancelled.as_code());
    }
    Ok(summary)
}

pub(crate) fn index_with(
    connection: &mut Connection,
    root: &Path,
    sources: &Sources,
    now: &str,
    progress: &mut dyn FnMut(Progress),
) -> Result<IndexSummary, ScanError> {
    check_root(root)?;
    let (library_id, scan) = start_run(connection, &root.to_string_lossy(), now)?;
    let mut summary = IndexSummary {
        scan,
        ..IndexSummary::default()
    };
    let outcome = index_run(connection, root, sources, now, library_id, &mut summary, progress);
    let status = match outcome {
        Ok(()) => "completed",
        Err(ScanError::Cancelled) => "cancelled",
        Err(_) => "failed",
    };
    let finished_at = (sources.clock)();
    let finished = finish_run(connection, library_id, &summary.scan, status, &finished_at).and_then(|()| {
        connection
            .execute(
                "UPDATE indexing_runs SET files_indexed = ?2 WHERE id = ?1",
                params![summary.scan.run_id, summary.resolved as i64],
            )
            .map(|_| ())
            .map_err(ScanError::from)
    });
    // F26: Google Photos favorites of the items this run added.
    let finished = finished.and_then(|()| crate::favorites::import_takeout_favorites(connection).map(|_| ()).map_err(ScanError::from));
    outcome?;
    finished?;
    Ok(summary)
}

fn index_run(
    connection: &mut Connection,
    root: &Path,
    sources: &Sources,
    now: &str,
    library_id: i64,
    summary: &mut IndexSummary,
    progress: &mut dyn FnMut(Progress),
) -> Result<(), ScanError> {
    let mut counts = Counts::default();
    progress(update(Stage::Scanning, 0, 0, &counts, Vec::new()));
    let read = {
        let progress = &mut *progress;
        let mut on_hash = |hash: HashProgress| {
            // Photos and videos are known once the walk is reconciled, before any hashing.
            let counts = Counts {
                photos: hash.photos,
                videos: hash.videos,
                ..Counts::default()
            };
            progress(Progress {
                stage: Stage::Hashing,
                done: hash.files_done,
                total: hash.files_total,
                bytes_done: hash.bytes_done,
                bytes_total: hash.bytes_total,
                counts,
                recent: Vec::new(),
            })
        };
        scan_run(connection, root, sources.fs, now, library_id, &mut summary.scan, &mut on_hash, sources.stop)?
    };
    counts = library_counts(connection, library_id, 0, summary.scan.errors)?;

    progress(update(Stage::Matching, 0, 0, &counts, Vec::new()));
    let report = (sources.discover)(root).map_err(|_| ScanError::RootMissing)?;
    record_problems(connection, &report, &mut summary.scan, now)?;
    // F25: Takeout album folders, rebuilt from what this run found.
    crate::albums::rebuild_takeout_albums(connection, library_id, &report, now)?;
    let other_part = report.media.iter().filter(|hit| json_in_other_part(hit)).count() as u64;
    let hits: HashMap<String, &MediaHit> = report
        .media
        .iter()
        .map(|hit| (key(&hit.relative_path), hit))
        .collect();

    let pending = pending_rows(connection, library_id, &read, &hits)?;
    let total = pending.len() as u64;
    counts = library_counts(connection, library_id, other_part, summary.scan.errors)?;
    progress(update(Stage::Resolving, 0, total, &counts, Vec::new()));
    for batch in pending.chunks(RESOLVE_BATCH) {
        if (sources.stop)() {
            return Err(ScanError::Cancelled);
        }
        let transaction = connection.transaction()?;
        let mut vanished = false;
        let mut recent = Vec::new();
        for row in batch {
            let hit = hits.get(&key(&row.relative_path)).copied();
            let exif = if row.media_type == "photo" {
                match (sources.read_exif)(&crate::scan::absolute_path(root, &row.relative_path)) {
                    Ok(exif) => Some(exif),
                    Err(_) if !root.is_dir() => {
                        vanished = true;
                        break;
                    }
                    Err(_) => None,
                }
            } else {
                None
            };
            if exif.as_ref().is_some_and(|exif| exif.exif_corrupt) {
                insert_error(&transaction, &mut summary.scan, &row.relative_path, "corrupt_media", now)?;
            }
            let json = hit.and_then(|hit| hit.metadata.as_ref());
            let fs = FsTimes {
                created_at: row.fs_created_at.as_deref(),
                modified_at: row.fs_modified_at.as_deref(),
            };
            let resolved = resolve(json, exif.as_ref(), fs, DEFAULT_FALLBACK_OFFSET_MINUTES);
            write_media(&transaction, library_id, row.id, hit, json, exif.as_ref(), &resolved, now)?;
            count(summary, json.is_some(), &resolved);
            recent.push(recent_line(&row.relative_path, json.and(hit), &resolved));
        }
        // The database usually lives on the vanished disk too; report the disk, not SQLite.
        let committed = transaction.commit();
        if vanished {
            return Err(ScanError::RootMissing);
        }
        committed?;
        counts = library_counts(connection, library_id, other_part, summary.scan.errors)?;
        let keep = recent.len().saturating_sub(RECENT);
        progress(update(Stage::Resolving, summary.resolved, total, &counts, recent.split_off(keep)));
    }
    Ok(())
}

fn update(stage: Stage, done: u64, total: u64, counts: &Counts, recent: Vec<Recent>) -> Progress {
    Progress {
        stage,
        done,
        total,
        bytes_done: 0,
        bytes_total: 0,
        counts: counts.clone(),
        recent,
    }
}

fn recent_line(relative_path: &str, paired: Option<&MediaHit>, resolved: &ResolvedMetadata) -> Recent {
    match paired {
        Some(hit) => Recent {
            kind: RecentKind::Paired,
            relative_path: relative_path.to_string(),
            json_name: hit
                .json_relative_path
                .as_deref()
                .map(|path| path.rsplit('/').next().unwrap_or(path).to_string()),
            date_source: None,
        },
        None => Recent {
            kind: RecentKind::NoJson,
            relative_path: relative_path.to_string(),
            json_name: None,
            date_source: resolved
                .taken_at_source
                .map(|source| source.as_column().to_string()),
        },
    }
}

/// The export part (`takeout-<stamp>-1-002`) a path sits in, if any.
pub(crate) fn export_part(path: &str) -> Option<&str> {
    path.split('/').find(|segment| takeout_export(segment).is_some())
}

/// F08 pairs across parts of one export (STATE: 169 of 4247 on the real Takeout).
fn json_in_other_part(hit: &MediaHit) -> bool {
    match (hit.metadata.as_ref(), hit.json_relative_path.as_deref()) {
        (Some(_), Some(json)) => {
            let media = export_part(&hit.relative_path).map(str::to_ascii_lowercase);
            let sidecar = export_part(json).map(str::to_ascii_lowercase);
            media.is_some() && sidecar.is_some() && media != sidecar
        }
        _ => false,
    }
}

/// The counters for the whole library, read from the database.
fn library_counts(
    connection: &Connection,
    library_id: i64,
    json_in_other_part: u64,
    problems: u64,
) -> Result<Counts, ScanError> {
    let (photos, videos, date_from_json, date_from_exif): (i64, i64, i64, i64) = connection.query_row(
        "SELECT coalesce(sum(media_type = 'photo'), 0), coalesce(sum(media_type = 'video'), 0),
                coalesce(sum(taken_at_source = 'takeout_json'), 0), coalesce(sum(taken_at_source = 'exif'), 0)
         FROM media_items WHERE library_id = ?1 AND is_hidden = 0 AND indexing_status != 'missing_file'",
        [library_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let people: i64 = connection.query_row(
        "SELECT count(*) FROM people WHERE library_id = ?1",
        [library_id],
        |row| row.get(0),
    )?;
    Ok(Counts {
        photos: photos as u64,
        videos: videos as u64,
        date_from_json: date_from_json as u64,
        date_from_exif: date_from_exif as u64,
        json_in_other_part,
        people: people as u64,
        problems,
    })
}

/// `COLLATE NOCASE` folds ASCII only; the in-memory key does the same.
fn key(path: &str) -> String {
    path.to_ascii_lowercase()
}

fn in_app_folder(path: &str) -> bool {
    let first = path.split('/').next().unwrap_or_default();
    first.eq_ignore_ascii_case(APP_FOLDER)
}

/// Sidecar problems from discovery. Folder and name problems are F05's and already
/// recorded by the scan.
fn record_problems(
    connection: &Connection,
    report: &DiscoveryReport,
    scan: &mut ScanSummary,
    now: &str,
) -> Result<(), ScanError> {
    for problem in &report.problems {
        let error_type = match problem.kind {
            ProblemKind::InvalidJson => "invalid_json",
            ProblemKind::OrphanJson => "orphan_json",
            ProblemKind::Unreadable => "unreadable",
            ProblemKind::UnreadableDirectory | ProblemKind::InvalidName => continue,
        };
        if !in_app_folder(&problem.relative_path) {
            insert_error(connection, scan, &problem.relative_path, error_type, now)?;
        }
    }
    Ok(())
}

struct PendingRow {
    id: i64,
    relative_path: String,
    media_type: String,
    fs_created_at: Option<String>,
    fs_modified_at: Option<String>,
}

/// Rows to (re)resolve: read by this scan, never resolved, or whose JSON match changed.
fn pending_rows(
    connection: &Connection,
    library_id: i64,
    read: &[i64],
    hits: &HashMap<String, &MediaHit>,
) -> Result<Vec<PendingRow>, ScanError> {
    let read: HashSet<i64> = read.iter().copied().collect();
    let mut statement = connection.prepare(
        "SELECT id, relative_path, media_type, fs_created_at, fs_modified_at, taken_at_source,
                json_relative_path, json_match_confidence
         FROM media_items WHERE library_id = ?1 AND indexing_status != 'missing_file'
         ORDER BY relative_path",
    )?;
    let rows = statement.query_map([library_id], |row| {
        Ok((
            PendingRow {
                id: row.get(0)?,
                relative_path: row.get(1)?,
                media_type: row.get(2)?,
                fs_created_at: row.get(3)?,
                fs_modified_at: row.get(4)?,
            },
            row.get::<_, Option<String>>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, Option<String>>(7)?,
        ))
    })?;
    let mut pending = Vec::new();
    for row in rows {
        let (row, source, json_path, confidence) = row?;
        let hit = hits.get(&key(&row.relative_path)).copied();
        let (hit_json, hit_confidence) = json_columns(hit);
        let changed = json_path.as_deref() != hit_json || confidence.as_deref() != Some(hit_confidence);
        if read.contains(&row.id) || source.is_none() || changed {
            pending.push(row);
        }
    }
    Ok(pending)
}

/// `(json_relative_path, json_match_confidence)` for a discovery hit. A sidecar that
/// failed to parse counts as no match (F02).
fn json_columns(hit: Option<&MediaHit>) -> (Option<&str>, &'static str) {
    match hit.filter(|hit| hit.metadata.is_some()) {
        Some(hit) => (
            hit.json_relative_path.as_deref(),
            match hit.confidence {
                Confidence::Exact => "exact",
                Confidence::Heuristic => "heuristic",
                Confidence::None => "none",
            },
        ),
        None => (None, "none"),
    }
}

#[allow(clippy::too_many_arguments)]
fn write_media(
    transaction: &Transaction,
    library_id: i64,
    id: i64,
    hit: Option<&MediaHit>,
    json: Option<&MediaMetadata>,
    exif: Option<&EmbeddedMetadata>,
    resolved: &ResolvedMetadata,
    now: &str,
) -> Result<(), ScanError> {
    let (json_path, confidence) = json_columns(hit);
    transaction.execute(
        // Video dimensions come from F13 (ffprobe), not from here: keep them.
        "UPDATE media_items SET
           width = CASE WHEN media_type = 'video' THEN width ELSE ?2 END,
           height = CASE WHEN media_type = 'video' THEN height ELSE ?3 END,
           orientation = ?4, taken_at = ?5,
           taken_at_source = ?6, taken_at_timezone = ?7, has_takeout_json = ?8,
           json_relative_path = ?9, json_match_confidence = ?10, title = ?11, description = ?12,
           updated_at = ?13
         WHERE id = ?1",
        params![
            id,
            resolved.width,
            resolved.height,
            resolved.orientation,
            resolved.taken_at,
            resolved.taken_at_source.map(DateSource::as_column),
            resolved.taken_at_timezone,
            json.is_some(),
            json_path,
            confidence,
            resolved.title,
            resolved.description,
            now
        ],
    )?;

    transaction.execute("DELETE FROM media_locations WHERE media_id = ?1", [id])?;
    if let Some(location) = resolved.location {
        transaction.execute(
            "INSERT INTO media_locations (media_id, latitude, longitude, altitude, source, place_label)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                location.latitude,
                location.longitude,
                location.altitude,
                location.source,
                crate::places::label(location.latitude, location.longitude)
            ],
        )?;
    }

    let takeout_json = json.map(|json| json.raw_json.as_str());
    let exif_json = exif.and_then(|exif| exif.raw_json.as_deref());
    transaction.execute("DELETE FROM media_metadata_raw WHERE media_id = ?1", [id])?;
    if takeout_json.is_some() || exif_json.is_some() {
        transaction.execute(
            "INSERT INTO media_metadata_raw (media_id, takeout_json, exif_json) VALUES (?1, ?2, ?3)",
            params![id, takeout_json, exif_json],
        )?;
    }

    transaction.execute("DELETE FROM media_people WHERE media_id = ?1", [id])?;
    for name in json.map(|json| json.people.as_slice()).unwrap_or_default() {
        let normalized = normalized_person_name(name);
        transaction.execute(
            "INSERT OR IGNORE INTO people (library_id, name, normalized_name) VALUES (?1, ?2, ?3)",
            params![library_id, name, normalized],
        )?;
        let person_id: i64 = transaction.query_row(
            "SELECT id FROM people WHERE library_id = ?1 AND normalized_name = ?2",
            params![library_id, normalized],
            |row| row.get(0),
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO media_people (media_id, person_id) VALUES (?1, ?2)",
            params![id, person_id],
        )?;
    }
    Ok(())
}

fn count(summary: &mut IndexSummary, with_json: bool, resolved: &ResolvedMetadata) {
    summary.resolved += 1;
    match resolved.taken_at_source {
        Some(DateSource::TakeoutJson) => summary.date_from_json += 1,
        Some(DateSource::Exif) => summary.date_from_exif += 1,
        Some(DateSource::Filesystem) => summary.date_from_filesystem += 1,
        None => summary.no_date += 1,
    }
    if resolved.taken_at_offset_assumed {
        summary.date_offset_assumed += 1;
    }
    if resolved.location.is_some() {
        summary.with_location += 1;
    }
    if with_json {
        summary.with_json += 1;
    }
}
