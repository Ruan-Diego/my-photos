// Reachable from F10. The F05 spec gives the app no trigger: only scan() and the
// ignored real_library_scan test call it.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::bench::hash::{hash_reader, to_hex};
use crate::takeout::{self, Confidence, MediaHit, ProblemKind};

/// Media written per transaction: an unplugged disk loses at most one batch.
const BATCH: usize = 500;

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "mkv", "avi", "3gp", "webm", "m4v"];

const UNREADABLE_MESSAGE: &str = "não foi possível ler o arquivo";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanError {
    RootMissing,
    RootLost,
    DatabaseFailed,
}

impl ScanError {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::RootMissing => "root_missing",
            Self::RootLost => "root_lost",
            Self::DatabaseFailed => "database_failed",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::RootMissing => "pasta da biblioteca não encontrada",
            Self::RootLost => "a pasta da biblioteca sumiu durante a varredura",
            Self::DatabaseFailed => "falha ao gravar no banco",
        }
    }
}

impl From<rusqlite::Error> for ScanError {
    fn from(_: rusqlite::Error) -> Self {
        Self::DatabaseFailed
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunType {
    Full,
    Incremental,
}

impl RunType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Incremental => "incremental",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanSummary {
    pub run_id: i64,
    pub run_type: RunType,
    /// Media files seen on disk.
    pub files_scanned: u64,
    /// Rows inserted, or updated because something about the file changed.
    pub files_indexed: u64,
    /// Media that could not be read.
    pub files_errored: u64,
    /// SHA-256 computations; zero on a scan of an unchanged disk.
    pub hashed: u64,
    /// Rows that became `missing_file` in this scan.
    pub missing: u64,
    /// Missing rows that took the path of a new file with the same content.
    pub moved: u64,
}

pub fn scan(connection: &mut Connection, root: &Path) -> Result<ScanSummary, ScanError> {
    scan_with(connection, root, BATCH, &mut hash_file)
}

pub(crate) fn hash_file(path: &Path) -> io::Result<String> {
    Ok(to_hex(&hash_reader(File::open(path)?)?))
}

pub(crate) fn scan_with(
    connection: &mut Connection,
    root: &Path,
    batch: usize,
    hash: &mut dyn FnMut(&Path) -> io::Result<String>,
) -> Result<ScanSummary, ScanError> {
    if !root.is_dir() {
        return Err(ScanError::RootMissing);
    }
    let (library_id, mut summary) = start_run(connection, root)?;

    let tree = match takeout::read_library_tree(root) {
        // A root that vanished mid-walk reads as empty folders; never trust that walk.
        Ok(tree) if root.is_dir() => tree,
        _ => return fail_run(connection, &summary),
    };
    let folders = tree.folders.clone();
    let report = takeout::discover_tree(root, tree, |path| {
        fs::read(path).map_err(|error| error.to_string())
    });
    let unreadable: Vec<String> = report
        .problems
        .iter()
        .filter(|problem| problem.kind == ProblemKind::UnreadableDirectory)
        .map(|problem| fold(&problem.relative_path))
        .collect();

    {
        let transaction = connection.transaction()?;
        for problem in &report.problems {
            record_error(
                &transaction,
                summary.run_id,
                &problem.relative_path,
                problem_type(problem.kind),
            )?;
        }
        transaction.commit()?;
    }

    let known = load_known(connection, library_id)?;
    let mut visit = Visit {
        root,
        library_id,
        known: &known,
        seen: HashSet::new(),
        inserted: Vec::new(),
        hash,
    };
    for chunk in report.media.chunks(batch.max(1)) {
        let transaction = connection.transaction()?;
        let mut lost = false;
        for hit in chunk {
            if visit.media(&transaction, hit, &mut summary)? == Outcome::RootLost {
                lost = true;
                break;
            }
        }
        // Rows already written in this batch are whole; keep them.
        transaction.commit()?;
        if lost {
            return fail_run(connection, &summary);
        }
    }

    let Visit { seen, inserted, .. } = visit;
    let shielded = |key: &str| {
        unreadable
            .iter()
            .any(|folder| key == folder || key.starts_with(&format!("{folder}/")))
    };
    let transaction = connection.transaction()?;
    let now = now();
    settle_missing(
        &transaction,
        &known,
        &seen,
        &inserted,
        &shielded,
        &now,
        &mut summary,
    )?;
    sync_folders(&transaction, library_id, &folders, &shielded, &now)?;
    transaction.execute(
        "UPDATE indexing_runs SET status = 'completed', finished_at = ?2, files_scanned = ?3,
           files_indexed = ?4, files_errored = ?5
         WHERE id = ?1",
        params![
            summary.run_id,
            now,
            summary.files_scanned as i64,
            summary.files_indexed as i64,
            summary.files_errored as i64
        ],
    )?;
    transaction.execute(
        "UPDATE libraries SET last_indexed_at = ?2 WHERE id = ?1",
        params![library_id, now],
    )?;
    transaction.commit()?;
    Ok(summary)
}

/// Fails any run a previous process left `running`, finds or creates the library row
/// and opens this run.
fn start_run(connection: &mut Connection, root: &Path) -> Result<(i64, ScanSummary), ScanError> {
    let root_path = root.to_string_lossy().into_owned();
    let transaction = connection.transaction()?;
    let now = now();
    transaction.execute(
        "UPDATE indexing_runs SET status = 'failed', finished_at = ?1 WHERE status = 'running'",
        [&now],
    )?;
    let existing: Option<i64> = transaction
        .query_row(
            "SELECT id FROM libraries WHERE root_path = ?1",
            [&root_path],
            |row| row.get(0),
        )
        .optional()?;
    let library_id = match existing {
        Some(id) => id,
        None => {
            transaction.execute(
                "INSERT INTO libraries (name, root_path, status, created_at)
                 VALUES (?1, ?2, 'active', ?3)",
                params![library_name(&root_path), root_path, now],
            )?;
            transaction.last_insert_rowid()
        }
    };
    let completed: bool = transaction.query_row(
        "SELECT EXISTS (SELECT 1 FROM indexing_runs WHERE library_id = ?1 AND status = 'completed')",
        [library_id],
        |row| row.get(0),
    )?;
    let run_type = if completed {
        RunType::Incremental
    } else {
        RunType::Full
    };
    transaction.execute(
        "INSERT INTO indexing_runs (library_id, started_at, run_type) VALUES (?1, ?2, ?3)",
        params![library_id, now, run_type.as_str()],
    )?;
    let run_id = transaction.last_insert_rowid();
    transaction.commit()?;
    Ok((
        library_id,
        ScanSummary {
            run_id,
            run_type,
            files_scanned: 0,
            files_indexed: 0,
            files_errored: 0,
            hashed: 0,
            missing: 0,
            moved: 0,
        },
    ))
}

/// `E:\` → `E:`.
fn library_name(root_path: &str) -> &str {
    let trimmed = root_path.trim_end_matches(['\\', '/']);
    if trimmed.is_empty() {
        root_path
    } else {
        trimmed
    }
}

fn fail_run(connection: &Connection, summary: &ScanSummary) -> Result<ScanSummary, ScanError> {
    connection.execute(
        "UPDATE indexing_runs SET status = 'failed', finished_at = ?2, files_scanned = ?3,
           files_indexed = ?4, files_errored = ?5
         WHERE id = ?1",
        params![
            summary.run_id,
            now(),
            summary.files_scanned as i64,
            summary.files_indexed as i64,
            summary.files_errored as i64
        ],
    )?;
    Err(ScanError::RootLost)
}

struct Known {
    id: i64,
    size: Option<i64>,
    modified: Option<String>,
    hash: Option<String>,
    status: String,
}

/// Rows of the library keyed by the case-folded path, as `COLLATE NOCASE` compares them.
fn load_known(
    connection: &Connection,
    library_id: i64,
) -> Result<HashMap<String, Known>, ScanError> {
    let mut statement = connection.prepare(
        "SELECT id, relative_path, size_bytes, fs_modified_at, file_hash_sha256, indexing_status
         FROM media_items WHERE library_id = ?1",
    )?;
    let rows = statement.query_map([library_id], |row| {
        Ok((
            fold(&row.get::<_, String>(1)?),
            Known {
                id: row.get(0)?,
                size: row.get(2)?,
                modified: row.get(3)?,
                hash: row.get(4)?,
                status: row.get(5)?,
            },
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Written,
    RootLost,
}

struct Visit<'a> {
    root: &'a Path,
    library_id: i64,
    known: &'a HashMap<String, Known>,
    seen: HashSet<String>,
    /// New rows with a hash, candidates for a move out of a missing row.
    inserted: Vec<(i64, Row)>,
    hash: &'a mut dyn FnMut(&Path) -> io::Result<String>,
}

impl Visit<'_> {
    fn media(
        &mut self,
        transaction: &Transaction,
        hit: &MediaHit,
        summary: &mut ScanSummary,
    ) -> Result<Outcome, ScanError> {
        let key = fold(&hit.relative_path);
        if !self.seen.insert(key.clone()) {
            // Two names that differ only in case cannot both live in one NTFS folder.
            record_error(
                transaction,
                summary.run_id,
                &hit.relative_path,
                "invalid_name",
            )?;
            return Ok(Outcome::Written);
        }
        summary.files_scanned += 1;
        let path = absolute(self.root, &hit.relative_path);
        let existing = self.known.get(&key);
        let mut row = Row::from_hit(hit);
        match fs::metadata(&path) {
            Err(_) if !self.root.is_dir() => return Ok(Outcome::RootLost),
            Err(_) => row.fail(),
            Ok(metadata) => {
                row.size = Some(metadata.len() as i64);
                row.modified = metadata.modified().ok().map(iso_utc);
                row.created = metadata.created().ok().map(iso_utc);
                let unchanged = existing.filter(|known| {
                    known.hash.is_some() && known.size == row.size && known.modified == row.modified
                });
                match unchanged {
                    Some(known) => row.hash = known.hash.clone(),
                    None => match (self.hash)(&path) {
                        Ok(digest) => {
                            summary.hashed += 1;
                            row.hash = Some(digest);
                        }
                        Err(_) if !self.root.is_dir() => return Ok(Outcome::RootLost),
                        Err(_) => row.fail(),
                    },
                }
            }
        }
        if row.status == "error" {
            summary.files_errored += 1;
            record_error(
                transaction,
                summary.run_id,
                &hit.relative_path,
                "unreadable",
            )?;
        }
        match existing {
            Some(known) => {
                if update_row(transaction, known.id, &row)? {
                    summary.files_indexed += 1;
                }
            }
            None => {
                let id = insert_row(transaction, self.library_id, &row)?;
                summary.files_indexed += 1;
                if row.hash.is_some() {
                    self.inserted.push((id, row));
                }
            }
        }
        Ok(Outcome::Written)
    }
}

#[derive(Debug, Clone)]
struct Row {
    relative_path: String,
    filename: String,
    extension: String,
    media_type: &'static str,
    size: Option<i64>,
    hash: Option<String>,
    created: Option<String>,
    modified: Option<String>,
    json_path: Option<String>,
    confidence: &'static str,
    status: &'static str,
    error: Option<&'static str>,
}

impl Row {
    fn from_hit(hit: &MediaHit) -> Self {
        let filename = hit
            .relative_path
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string();
        let extension = filename
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .unwrap_or_default();
        let media_type = if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
            "video"
        } else {
            "photo"
        };
        Self {
            relative_path: hit.relative_path.clone(),
            filename,
            extension,
            media_type,
            size: None,
            hash: None,
            created: None,
            modified: None,
            json_path: hit.json_relative_path.clone(),
            confidence: match hit.confidence {
                Confidence::Exact => "exact",
                Confidence::Heuristic => "heuristic",
                Confidence::None => "none",
            },
            status: "ok",
            error: None,
        }
    }

    fn fail(&mut self) {
        self.hash = None;
        self.status = "error";
        self.error = Some(UNREADABLE_MESSAGE);
    }
}

fn insert_row(transaction: &Transaction, library_id: i64, row: &Row) -> Result<i64, ScanError> {
    transaction.execute(
        "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type,
           size_bytes, file_hash_sha256, fs_created_at, fs_modified_at, has_takeout_json,
           json_relative_path, json_match_confidence, indexing_status, indexing_error,
           created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
        params![
            library_id,
            row.relative_path,
            row.filename,
            row.extension,
            row.media_type,
            row.size,
            row.hash,
            row.created,
            row.modified,
            row.json_path.is_some(),
            row.json_path,
            row.confidence,
            row.status,
            row.error,
            now()
        ],
    )?;
    Ok(transaction.last_insert_rowid())
}

/// Writes the row only when a field differs, so an unchanged file keeps `updated_at`.
/// The path compares with BINARY: a case-only rename is a change even under NOCASE.
fn update_row(transaction: &Transaction, id: i64, row: &Row) -> Result<bool, ScanError> {
    let changed = transaction.execute(
        "UPDATE media_items SET relative_path = ?2, filename = ?3, extension = ?4,
           media_type = ?5, size_bytes = ?6, file_hash_sha256 = ?7, fs_created_at = ?8,
           fs_modified_at = ?9, has_takeout_json = ?10, json_relative_path = ?11,
           json_match_confidence = ?12, indexing_status = ?13, indexing_error = ?14,
           updated_at = ?15
         WHERE id = ?1 AND NOT (
           relative_path IS ?2 COLLATE BINARY AND filename IS ?3 AND extension IS ?4
           AND media_type IS ?5 AND size_bytes IS ?6 AND file_hash_sha256 IS ?7
           AND fs_created_at IS ?8 AND fs_modified_at IS ?9 AND has_takeout_json IS ?10
           AND json_relative_path IS ?11 AND json_match_confidence IS ?12
           AND indexing_status IS ?13 AND indexing_error IS ?14)",
        params![
            id,
            row.relative_path,
            row.filename,
            row.extension,
            row.media_type,
            row.size,
            row.hash,
            row.created,
            row.modified,
            row.json_path.is_some(),
            row.json_path,
            row.confidence,
            row.status,
            row.error,
            now()
        ],
    )?;
    Ok(changed == 1)
}

/// Known rows not seen in this walk: a single new file with the same SHA-256 and size
/// takes over the row (moved outside the app); every other one becomes `missing_file`.
/// Rows under a folder that could not be read are left as they were.
fn settle_missing(
    transaction: &Transaction,
    known: &HashMap<String, Known>,
    seen: &HashSet<String>,
    inserted: &[(i64, Row)],
    shielded: &dyn Fn(&str) -> bool,
    now: &str,
    summary: &mut ScanSummary,
) -> Result<(), ScanError> {
    let mut gone: Vec<&Known> = known
        .iter()
        .filter(|(key, _)| !seen.contains(*key) && !shielded(key))
        .map(|(_, row)| row)
        .collect();
    gone.sort_by_key(|row| row.id);

    let mut gone_by_content: HashMap<(&str, i64), Vec<&Known>> = HashMap::new();
    for row in &gone {
        if let (Some(hash), Some(size)) = (row.hash.as_deref(), row.size) {
            gone_by_content.entry((hash, size)).or_default().push(*row);
        }
    }
    let mut new_by_content: HashMap<(&str, i64), Vec<&(i64, Row)>> = HashMap::new();
    for entry in inserted {
        if let (Some(hash), Some(size)) = (entry.1.hash.as_deref(), entry.1.size) {
            new_by_content.entry((hash, size)).or_default().push(entry);
        }
    }

    let mut moved = HashSet::new();
    for (content, olds) in &gone_by_content {
        let Some(news) = new_by_content.get(content) else {
            continue;
        };
        if olds.len() != 1 || news.len() != 1 {
            continue;
        }
        let (new_id, row) = news[0];
        transaction.execute("DELETE FROM media_items WHERE id = ?1", [new_id])?;
        update_row(transaction, olds[0].id, row)?;
        moved.insert(olds[0].id);
        summary.moved += 1;
    }

    for row in gone {
        if moved.contains(&row.id) || row.status == "missing_file" {
            continue;
        }
        transaction.execute(
            "UPDATE media_items SET indexing_status = 'missing_file', updated_at = ?2 WHERE id = ?1",
            params![row.id, now],
        )?;
        summary.missing += 1;
    }
    Ok(())
}

fn sync_folders(
    transaction: &Transaction,
    library_id: i64,
    folders: &[String],
    shielded: &dyn Fn(&str) -> bool,
    now: &str,
) -> Result<(), ScanError> {
    for folder in folders {
        transaction.execute(
            "INSERT INTO folders (library_id, relative_path, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT (library_id, relative_path) DO UPDATE SET relative_path = excluded.relative_path
             WHERE relative_path IS NOT excluded.relative_path COLLATE BINARY",
            params![library_id, folder, now],
        )?;
    }
    let present: HashSet<String> = folders.iter().map(|folder| fold(folder)).collect();
    let stored: Vec<(i64, String)> = {
        let mut statement =
            transaction.prepare("SELECT id, relative_path FROM folders WHERE library_id = ?1")?;
        let rows = statement.query_map([library_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        rows.collect::<Result<_, _>>()?
    };
    for (id, path) in stored {
        let key = fold(&path);
        if !present.contains(&key) && !shielded(&key) {
            transaction.execute("DELETE FROM folders WHERE id = ?1", [id])?;
        }
    }
    Ok(())
}

fn record_error(
    transaction: &Transaction,
    run_id: i64,
    relative_path: &str,
    error_type: &str,
) -> Result<(), ScanError> {
    transaction.execute(
        "INSERT INTO indexing_errors (run_id, relative_path, error_type, message, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            run_id,
            relative_path,
            error_type,
            error_message(error_type),
            now()
        ],
    )?;
    Ok(())
}

fn problem_type(kind: ProblemKind) -> &'static str {
    match kind {
        ProblemKind::InvalidJson => "invalid_json",
        ProblemKind::Unreadable => "unreadable",
        ProblemKind::OrphanJson => "orphan_json",
        ProblemKind::UnreadableDirectory => "unreadable_directory",
        ProblemKind::InvalidName => "invalid_name",
    }
}

fn error_message(error_type: &str) -> &'static str {
    match error_type {
        "invalid_json" => "JSON do Takeout inválido",
        "orphan_json" => "JSON do Takeout sem mídia",
        "unreadable_directory" => "não foi possível ler a pasta",
        "invalid_name" => "nome de arquivo inválido",
        _ => UNREADABLE_MESSAGE,
    }
}

/// ASCII case fold, the same folding as SQLite's `COLLATE NOCASE`.
fn fold(relative_path: &str) -> String {
    relative_path.to_ascii_lowercase()
}

fn absolute(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

fn now() -> String {
    iso_utc(SystemTime::now())
}

/// ISO-8601 UTC with `Z`, whole seconds (the F04 date format).
fn iso_utc(time: SystemTime) -> String {
    let seconds = match time.duration_since(UNIX_EPOCH) {
        Ok(after) => after.as_secs() as i64,
        Err(before) => {
            let before = before.duration();
            -(before.as_secs() as i64) - i64::from(before.subsec_nanos() > 0)
        }
    };
    format_utc(seconds)
}

fn format_utc(seconds: i64) -> String {
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    let second_of_day = seconds.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        second_of_day / 3600,
        second_of_day % 3600 / 60,
        second_of_day % 60
    )
}

/// Days since 1970-01-01 → proleptic Gregorian date (H. Hinnant, `civil_from_days`).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month as u32, day as u32)
}

#[cfg(test)]
mod tests;
