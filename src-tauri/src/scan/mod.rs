//! F05: walk the library, record folders and media, hash what is new or changed.
//! Reads the library only; every write goes to the database (AD-008, AD-011).

mod walk;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::Serialize;

use crate::bench::hash::{hash_reader, to_hex};
use crate::db;
use crate::time::iso_utc;
use walk::{absolute, walk, FileFact, Walk};
pub(crate) use walk::{absolute as absolute_path, FileSystem, RealFs};

/// AD-007 and AD-010: the library is the whole `E:` volume. F11 finds it on another
/// letter when the disk moves (`volume::locate_library`).
#[allow(dead_code)]
pub const LIBRARY_ROOT: &str = r"E:\";

/// Hash results are committed in batches this size; a crash loses at most one batch.
const HASH_BATCH: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanError {
    RootNotAbsolute,
    RootMissing,
    RootNotDirectory,
    DbWriteFailed,
    /// F17: the user paused the run; what was committed stays.
    Cancelled,
}

impl ScanError {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::RootNotAbsolute => "root_not_absolute",
            Self::RootMissing => "root_missing",
            Self::RootNotDirectory => "root_not_directory",
            Self::DbWriteFailed => "db_write_failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::RootNotAbsolute => "pasta da biblioteca não é um caminho absoluto",
            Self::RootMissing => "pasta da biblioteca não encontrada",
            Self::RootNotDirectory => "a biblioteca não é uma pasta",
            Self::DbWriteFailed => "falha ao gravar no banco",
            Self::Cancelled => "indexação pausada",
        }
    }
}

impl From<rusqlite::Error> for ScanError {
    fn from(_: rusqlite::Error) -> Self {
        Self::DbWriteFailed
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanSummary {
    pub run_id: i64,
    pub run_type: String,
    /// Folders on disk (root excluded).
    pub folders: u64,
    /// Media files on disk.
    pub media: u64,
    pub new: u64,
    pub modified: u64,
    pub unchanged: u64,
    /// Became `missing_file` in this run.
    pub missing: u64,
    /// Came back from `missing_file` in this run.
    pub restored: u64,
    /// Read and hashed successfully in this run.
    pub hashed: u64,
    /// Rows written to `indexing_errors` in this run.
    pub errors: u64,
    pub bytes_hashed: u64,
}

pub fn scan_library(connection: &mut Connection, root: &Path) -> Result<ScanSummary, ScanError> {
    let now = iso_utc(SystemTime::now()).unwrap_or_default();
    scan_with(connection, root, &RealFs, &now)
}

/// Finds the library disk (F11), scans it into its `.myphotos\library.db` and closes it. Errors are codes.
pub fn scan_default_library() -> Result<ScanSummary, &'static str> {
    let (root, mut connection) = db::open_located().map_err(|error| error.as_code())?;
    let result = scan_library(&mut connection, &root);
    db::close(connection);
    result.map_err(|error| error.as_code())
}

pub(crate) fn scan_with(
    connection: &mut Connection,
    root: &Path,
    fs: &dyn FileSystem,
    now: &str,
) -> Result<ScanSummary, ScanError> {
    check_root(root)?;
    let (library_id, mut summary) = start_run(connection, &root.to_string_lossy(), now)?;
    let outcome = scan_run(connection, root, fs, now, library_id, &mut summary, &mut |_| {}, &|| false);
    let status = if outcome.is_ok() {
        "completed"
    } else {
        "failed"
    };
    let finished = finish_run(connection, library_id, &summary, status, now);
    outcome?;
    finished?;
    Ok(summary)
}

pub(crate) fn check_root(root: &Path) -> Result<(), ScanError> {
    if !root.is_absolute() {
        return Err(ScanError::RootNotAbsolute);
    }
    match std::fs::metadata(root) {
        Err(_) => Err(ScanError::RootMissing),
        Ok(meta) if !meta.is_dir() => Err(ScanError::RootNotDirectory),
        Ok(_) => Ok(()),
    }
}

/// F17: where the hashing stands, sent after each file read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct HashProgress {
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    /// Media on disk by type.
    pub photos: u64,
    pub videos: u64,
}

/// Returns the ids of the media rows read (hashed or failed) in this run. `on_hash` is
/// called once before the first file and after each one; `stop()` is checked between
/// files and ends the run with `Cancelled`.
pub(crate) fn scan_run(
    connection: &mut Connection,
    root: &Path,
    fs: &dyn FileSystem,
    now: &str,
    library_id: i64,
    summary: &mut ScanSummary,
    on_hash: &mut dyn FnMut(HashProgress),
    stop: &dyn Fn() -> bool,
) -> Result<Vec<i64>, ScanError> {
    let found = walk(root, fs).map_err(|_| ScanError::RootMissing)?;
    summary.folders = found.folders.len() as u64;
    summary.media = found.files.len() as u64;
    let queue = reconcile(connection, library_id, &found, now, summary)?;
    let sizes: HashMap<String, u64> = found
        .files
        .iter()
        .map(|file| (key(&file.relative_path), file.size))
        .collect();
    let bytes_total = queue
        .iter()
        .map(|(_, path)| sizes.get(&key(path)).copied().unwrap_or(0))
        .sum();
    let photos = found.files.iter().filter(|file| file.media_type == "photo").count() as u64;
    let start = HashProgress {
        files_total: queue.len() as u64,
        bytes_total,
        photos,
        videos: summary.media - photos,
        ..HashProgress::default()
    };
    hash_queue(connection, root, fs, now, summary, &queue, start, on_hash, stop)?;
    Ok(queue.into_iter().map(|(id, _)| id).collect())
}

pub(crate) fn start_run(
    connection: &mut Connection,
    root_path: &str,
    now: &str,
) -> Result<(i64, ScanSummary), ScanError> {
    let volume = crate::volume::volume_info(Path::new(root_path));
    let transaction = connection.transaction()?;
    crate::volume::relink(&transaction, root_path, volume.as_ref())?;
    let existing: Option<i64> = transaction
        .query_row(
            "SELECT id FROM libraries WHERE root_path = ?1",
            [root_path],
            |row| row.get(0),
        )
        .optional()?;
    let library_id = match existing {
        Some(id) => id,
        None => {
            let name = root_path.trim_end_matches(['\\', '/']);
            transaction.execute(
                "INSERT INTO libraries (name, root_path, status, created_at) VALUES (?1, ?2, 'active', ?3)",
                params![name, root_path, now],
            )?;
            transaction.last_insert_rowid()
        }
    };
    let completed: bool = transaction.query_row(
        "SELECT EXISTS (SELECT 1 FROM indexing_runs WHERE library_id = ?1 AND status = 'completed')",
        [library_id],
        |row| row.get(0),
    )?;
    crate::volume::record_volume(&transaction, library_id, volume.as_ref())?;
    let run_type = if completed { "incremental" } else { "full" };
    transaction.execute(
        "INSERT INTO indexing_runs (library_id, started_at, run_type, status) VALUES (?1, ?2, ?3, 'running')",
        params![library_id, now, run_type],
    )?;
    let run_id = transaction.last_insert_rowid();
    transaction.commit()?;
    Ok((
        library_id,
        ScanSummary {
            run_id,
            run_type: run_type.to_string(),
            ..ScanSummary::default()
        },
    ))
}

/// A run left `running` by an app that closed (or crashed) mid-way: at startup nothing
/// is indexing, so every such run ended without saying so. Marks them `cancelled`.
pub fn close_orphan_runs(connection: &Connection, now: &str) -> rusqlite::Result<usize> {
    connection.execute(
        "UPDATE indexing_runs SET status = 'cancelled', finished_at = ?1 WHERE status = 'running'",
        [now],
    )
}

pub(crate) fn finish_run(
    connection: &Connection,
    library_id: i64,
    summary: &ScanSummary,
    status: &str,
    now: &str,
) -> Result<(), ScanError> {
    connection.execute(
        "UPDATE indexing_runs SET finished_at = ?2, status = ?3, files_scanned = ?4, files_indexed = ?5, files_errored = ?6
         WHERE id = ?1",
        params![
            summary.run_id,
            now,
            status,
            summary.media as i64,
            summary.hashed as i64,
            summary.errors as i64
        ],
    )?;
    if status == "completed" {
        connection.execute(
            "UPDATE libraries SET last_indexed_at = ?2 WHERE id = ?1",
            params![library_id, now],
        )?;
    }
    Ok(())
}

struct KnownMedia {
    id: i64,
    relative_path: String,
    size: Option<i64>,
    modified: Option<String>,
    has_hash: bool,
    status: String,
}

/// A media row that must be read: `(id, relative_path)`.
type Queue = Vec<(i64, String)>;

/// `COLLATE NOCASE` folds ASCII only; the in-memory key does the same.
fn key(path: &str) -> String {
    path.to_ascii_lowercase()
}

fn is_protected(path: &str, protected: &[String]) -> bool {
    protected.iter().any(|dir| {
        dir.is_empty()
            || path.eq_ignore_ascii_case(dir)
            || (path.len() > dir.len()
                && path.as_bytes()[dir.len()] == b'/'
                && path[..dir.len()].eq_ignore_ascii_case(dir))
    })
}

fn reconcile(
    connection: &mut Connection,
    library_id: i64,
    found: &Walk,
    now: &str,
    summary: &mut ScanSummary,
) -> Result<Queue, ScanError> {
    let transaction = connection.transaction()?;
    reconcile_folders(&transaction, library_id, found, now)?;
    let queue = reconcile_media(&transaction, library_id, found, now, summary)?;
    for (relative_path, error_type) in &found.errors {
        insert_error(&transaction, summary, relative_path, error_type, now)?;
    }
    transaction.commit()?;
    Ok(queue)
}

fn reconcile_folders(
    transaction: &Transaction,
    library_id: i64,
    found: &Walk,
    now: &str,
) -> Result<(), ScanError> {
    let mut known: HashMap<String, (i64, String)> = HashMap::new();
    {
        let mut statement =
            transaction.prepare("SELECT id, relative_path FROM folders WHERE library_id = ?1")?;
        let rows = statement.query_map([library_id], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        for row in rows {
            let (id, path) = row?;
            known.insert(key(&path), (id, path));
        }
    }
    for folder in &found.folders {
        match known.remove(&key(folder)) {
            Some((id, path)) if &path != folder => {
                transaction.execute(
                    "UPDATE folders SET relative_path = ?2 WHERE id = ?1",
                    params![id, folder],
                )?;
            }
            Some(_) => {}
            None => {
                transaction.execute(
                    "INSERT INTO folders (library_id, relative_path, created_at) VALUES (?1, ?2, ?3)",
                    params![library_id, folder, now],
                )?;
            }
        }
    }
    for (id, path) in known.into_values() {
        if !is_protected(&path, &found.protected) {
            transaction.execute("DELETE FROM folders WHERE id = ?1", [id])?;
        }
    }
    Ok(())
}

fn reconcile_media(
    transaction: &Transaction,
    library_id: i64,
    found: &Walk,
    now: &str,
    summary: &mut ScanSummary,
) -> Result<Queue, ScanError> {
    let mut known: HashMap<String, KnownMedia> = HashMap::new();
    {
        let mut statement = transaction.prepare(
            "SELECT id, relative_path, size_bytes, fs_modified_at, file_hash_sha256 IS NOT NULL, indexing_status
             FROM media_items WHERE library_id = ?1",
        )?;
        let rows = statement.query_map([library_id], |row| {
            Ok(KnownMedia {
                id: row.get(0)?,
                relative_path: row.get(1)?,
                size: row.get(2)?,
                modified: row.get(3)?,
                has_hash: row.get(4)?,
                status: row.get(5)?,
            })
        })?;
        for row in rows {
            let media = row?;
            known.insert(key(&media.relative_path), media);
        }
    }

    let mut queue = Queue::new();
    for file in &found.files {
        match known.remove(&key(&file.relative_path)) {
            None => {
                transaction.execute(
                    "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, size_bytes,
                       fs_created_at, fs_modified_at, indexing_status, created_at, updated_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'ok', ?9, ?9)",
                    params![
                        library_id,
                        file.relative_path,
                        file.filename,
                        file.extension,
                        file.media_type,
                        file.size as i64,
                        file.created,
                        file.modified,
                        now
                    ],
                )?;
                summary.new += 1;
                queue.push((transaction.last_insert_rowid(), file.relative_path.clone()));
            }
            Some(media) => {
                if let Some(id) = update_known(transaction, &media, file, now, summary)? {
                    queue.push((id, file.relative_path.clone()));
                }
            }
        }
    }

    for media in known.into_values() {
        if media.status != "missing_file" && !is_protected(&media.relative_path, &found.protected) {
            transaction.execute(
                "UPDATE media_items SET indexing_status = 'missing_file', updated_at = ?2 WHERE id = ?1",
                params![media.id, now],
            )?;
            summary.missing += 1;
        }
    }
    queue.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(queue)
}

/// Applies the design's reconciliation rules to a row that is still on disk. Returns the
/// id when the file must be read.
fn update_known(
    transaction: &Transaction,
    media: &KnownMedia,
    file: &FileFact,
    now: &str,
    summary: &mut ScanSummary,
) -> Result<Option<i64>, ScanError> {
    if media.relative_path != file.relative_path {
        transaction.execute(
            "UPDATE media_items SET relative_path = ?2, filename = ?3, updated_at = ?4 WHERE id = ?1",
            params![media.id, file.relative_path, file.filename, now],
        )?;
    }
    let was_missing = media.status == "missing_file";
    let same = media.size == Some(file.size as i64) && media.modified == file.modified;
    if !same {
        transaction.execute(
            "UPDATE media_items SET size_bytes = ?2, fs_modified_at = ?3, fs_created_at = ?4,
               file_hash_sha256 = NULL, indexing_status = 'ok', indexing_error = NULL, updated_at = ?5
             WHERE id = ?1",
            params![media.id, file.size as i64, file.modified, file.created, now],
        )?;
        summary.modified += 1;
        if was_missing {
            summary.restored += 1;
        }
        return Ok(Some(media.id));
    }
    if was_missing {
        transaction.execute(
            "UPDATE media_items SET indexing_status = 'ok', updated_at = ?2 WHERE id = ?1",
            params![media.id, now],
        )?;
        summary.restored += 1;
    }
    if !media.has_hash {
        return Ok(Some(media.id));
    }
    if !was_missing {
        summary.unchanged += 1;
    }
    Ok(None)
}

pub(crate) fn insert_error(
    connection: &Connection,
    summary: &mut ScanSummary,
    relative_path: &str,
    error_type: &str,
    now: &str,
) -> Result<(), ScanError> {
    connection.execute(
        "INSERT INTO indexing_errors (run_id, relative_path, error_type, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![summary.run_id, relative_path, error_type, now],
    )?;
    summary.errors += 1;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn hash_queue(
    connection: &mut Connection,
    root: &Path,
    fs: &dyn FileSystem,
    now: &str,
    summary: &mut ScanSummary,
    queue: &Queue,
    mut progress: HashProgress,
    on_hash: &mut dyn FnMut(HashProgress),
    stop: &dyn Fn() -> bool,
) -> Result<(), ScanError> {
    let bytes_before = summary.bytes_hashed;
    on_hash(progress);
    for batch in queue.chunks(HASH_BATCH) {
        let transaction = connection.transaction()?;
        let mut vanished = false;
        let mut stopped = false;
        for (id, relative_path) in batch {
            if stop() {
                stopped = true;
                break;
            }
            let digest = fs.open(&absolute(root, relative_path)).and_then(|reader| {
                hash_reader(CountingReader::new(reader, &mut summary.bytes_hashed))
            });
            match digest {
                Ok(digest) => {
                    transaction.execute(
                        "UPDATE media_items SET file_hash_sha256 = ?2, indexing_status = 'ok', indexing_error = NULL,
                           updated_at = ?3 WHERE id = ?1",
                        params![id, to_hex(&digest), now],
                    )?;
                    summary.hashed += 1;
                }
                Err(_) if !root.is_dir() => {
                    vanished = true;
                    break;
                }
                Err(_) => {
                    transaction.execute(
                        "UPDATE media_items SET file_hash_sha256 = NULL, indexing_status = 'error',
                           indexing_error = 'unreadable', updated_at = ?2 WHERE id = ?1",
                        params![id, now],
                    )?;
                    insert_error(&transaction, summary, relative_path, "unreadable", now)?;
                }
            }
            progress.files_done += 1;
            progress.bytes_done = summary.bytes_hashed - bytes_before;
            on_hash(progress);
        }
        // The database usually lives on the vanished disk too; report the disk, not SQLite.
        let committed = transaction.commit();
        if vanished {
            return Err(ScanError::RootMissing);
        }
        committed?;
        if stopped {
            return Err(ScanError::Cancelled);
        }
    }
    Ok(())
}

/// Adds every byte read to `bytes_hashed`; a failed read keeps what was counted.
struct CountingReader<'a> {
    inner: Box<dyn std::io::Read>,
    total: &'a mut u64,
}

impl<'a> CountingReader<'a> {
    fn new(inner: Box<dyn std::io::Read>, total: &'a mut u64) -> Self {
        Self { inner, total }
    }
}

impl std::io::Read for CountingReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.inner.read(buf)?;
        *self.total += read as u64;
        Ok(read)
    }
}
