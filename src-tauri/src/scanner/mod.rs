// Reachable from F10, which runs the scan. The shell does not call it yet.
#![allow(dead_code)]

//! F05: walks the library, hashes new and changed media, and keeps `folders`,
//! `media_items`, `indexing_runs` and `indexing_errors` in step with the disk.

mod time;

use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rusqlite::{params, Connection, OptionalExtension, Transaction};

use crate::bench::hash::{hash_reader, to_hex};
use crate::takeout::{self, Confidence, DiscoveryReport, MediaHit, ProblemKind};

use time::{iso_utc, now_iso};

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "mkv", "avi", "3gp", "webm", "m4v"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanError {
    RootUnreadable,
    Database,
}

impl ScanError {
    pub fn as_code(self) -> &'static str {
        match self {
            ScanError::RootUnreadable => "root_unreadable",
            ScanError::Database => "scan_db_failed",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            ScanError::RootUnreadable => "não foi possível ler a pasta da biblioteca",
            ScanError::Database => "falha ao gravar a varredura no banco",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunType {
    Full,
    Incremental,
}

impl RunType {
    fn as_sql(self) -> &'static str {
        match self {
            RunType::Full => "full",
            RunType::Incremental => "incremental",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanSummary {
    pub run_id: i64,
    pub library_id: i64,
    pub run_type: RunType,
    /// Media files found on disk.
    pub files_scanned: u32,
    pub added: u32,
    pub modified: u32,
    pub moved: u32,
    pub unchanged: u32,
    /// Rows newly marked `missing_file` by this run.
    pub missing: u32,
    pub errored: u32,
    pub folders: u32,
}

/// Scans `root` into the library whose `root_path` is `root`, creating that library row
/// on the first scan.
pub fn scan(connection: &mut Connection, root: &Path) -> Result<ScanSummary, ScanError> {
    scan_with(connection, root, hash_file)
}

pub(crate) fn scan_with(
    connection: &mut Connection,
    root: &Path,
    hash: impl Fn(&Path) -> io::Result<[u8; 32]>,
) -> Result<ScanSummary, ScanError> {
    // Walk before touching the database: a disk that is gone changes nothing.
    let report = takeout::discover(root).map_err(|_| ScanError::RootUnreadable)?;

    let library_id = ensure_library(connection, root)?;
    let existing = load_existing(connection, library_id)?;
    let run_type = if existing.is_empty() {
        RunType::Full
    } else {
        RunType::Incremental
    };
    let started_at = now_iso();
    connection
        .execute(
            "INSERT INTO indexing_runs (library_id, started_at, run_type) VALUES (?1, ?2, ?3)",
            params![library_id, started_at, run_type.as_sql()],
        )
        .map_err(|_| ScanError::Database)?;
    let run_id = connection.last_insert_rowid();

    let plan = plan(root, &report, existing, &hash);
    match write(connection, library_id, run_id, run_type, &report, plan) {
        Ok(summary) => Ok(summary),
        Err(error) => {
            let _ = connection.execute(
                "UPDATE indexing_runs SET status = 'failed', finished_at = ?2 WHERE id = ?1",
                params![run_id, now_iso()],
            );
            Err(error)
        }
    }
}

fn hash_file(path: &Path) -> io::Result<[u8; 32]> {
    hash_reader(File::open(path)?)
}

fn ensure_library(connection: &Connection, root: &Path) -> Result<i64, ScanError> {
    let root_path = root.to_string_lossy().into_owned();
    let found: Option<i64> = connection
        .query_row(
            "SELECT id FROM libraries WHERE root_path = ?1 ORDER BY id LIMIT 1",
            [&root_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| ScanError::Database)?;
    if let Some(id) = found {
        return Ok(id);
    }
    connection
        .execute(
            "INSERT INTO libraries (name, root_path, created_at) VALUES (?1, ?1, ?2)",
            params![root_path, now_iso()],
        )
        .map_err(|_| ScanError::Database)?;
    Ok(connection.last_insert_rowid())
}

struct Existing {
    id: i64,
    relative_path: String,
    size_bytes: Option<i64>,
    fs_modified_at: Option<String>,
    hash: Option<String>,
    status: String,
}

/// Keyed by the lowercased path: `relative_path` is `COLLATE NOCASE` (NTFS).
fn load_existing(
    connection: &Connection,
    library_id: i64,
) -> Result<HashMap<String, Existing>, ScanError> {
    let mut statement = connection
        .prepare(
            "SELECT id, relative_path, size_bytes, fs_modified_at, file_hash_sha256, indexing_status
             FROM media_items WHERE library_id = ?1",
        )
        .map_err(|_| ScanError::Database)?;
    let rows = statement
        .query_map([library_id], |row| {
            Ok(Existing {
                id: row.get(0)?,
                relative_path: row.get(1)?,
                size_bytes: row.get(2)?,
                fs_modified_at: row.get(3)?,
                hash: row.get(4)?,
                status: row.get(5)?,
            })
        })
        .map_err(|_| ScanError::Database)?;
    let mut existing = HashMap::new();
    for row in rows {
        let row = row.map_err(|_| ScanError::Database)?;
        existing.insert(row.relative_path.to_lowercase(), row);
    }
    Ok(existing)
}

/// What the disk says about one media file.
struct FileFacts {
    size_bytes: i64,
    fs_created_at: Option<String>,
    fs_modified_at: Option<String>,
}

enum Action {
    Unchanged {
        id: i64,
    },
    Update {
        id: i64,
        hash: String,
    },
    Insert {
        hash: String,
    },
    /// The file could not be stat'ed or hashed. `id` when the row already exists.
    Error {
        id: Option<i64>,
        message: String,
    },
}

struct Planned<'a> {
    hit: &'a MediaHit,
    facts: Option<FileFacts>,
    action: Action,
}

struct Plan<'a> {
    items: Vec<Planned<'a>>,
    /// Rows whose file is gone, unless a folder above them could not be read.
    gone: Vec<Existing>,
}

fn plan<'a>(
    root: &Path,
    report: &'a DiscoveryReport,
    mut existing: HashMap<String, Existing>,
    hash: &impl Fn(&Path) -> io::Result<[u8; 32]>,
) -> Plan<'a> {
    let mut items = Vec::with_capacity(report.media.len());
    for hit in &report.media {
        let known = existing.remove(&hit.relative_path.to_lowercase());
        let absolute = absolute_from_relative(root, &hit.relative_path);
        let facts = match stat(&absolute) {
            Ok(facts) => facts,
            Err(error) => {
                items.push(Planned {
                    hit,
                    facts: None,
                    action: Action::Error {
                        id: known.map(|row| row.id),
                        message: error.to_string(),
                    },
                });
                continue;
            }
        };
        let unchanged = known.as_ref().is_some_and(|row| {
            row.hash.is_some()
                && row.status == "ok"
                && row.size_bytes == Some(facts.size_bytes)
                && row.fs_modified_at == facts.fs_modified_at
        });
        let action = if unchanged {
            Action::Unchanged {
                id: known.as_ref().map(|row| row.id).unwrap_or_default(),
            }
        } else {
            match hash(&absolute) {
                Ok(digest) => {
                    let hash = to_hex(&digest);
                    match &known {
                        Some(row) => Action::Update { id: row.id, hash },
                        None => Action::Insert { hash },
                    }
                }
                Err(error) => Action::Error {
                    id: known.as_ref().map(|row| row.id),
                    message: error.to_string(),
                },
            }
        };
        items.push(Planned {
            hit,
            facts: Some(facts),
            action,
        });
    }

    // A folder that failed to list hides its files; they are not gone.
    let unreadable: Vec<String> = report
        .problems
        .iter()
        .filter(|problem| problem.kind == ProblemKind::UnreadableDirectory)
        .map(|problem| format!("{}/", problem.relative_path.to_lowercase()))
        .collect();
    let mut gone: Vec<Existing> = existing
        .into_values()
        .filter(|row| {
            let path = row.relative_path.to_lowercase();
            !unreadable.iter().any(|prefix| path.starts_with(prefix))
        })
        .collect();
    gone.sort_by_key(|row| row.id);
    Plan { items, gone }
}

fn stat(path: &Path) -> io::Result<FileFacts> {
    let metadata = fs::metadata(path)?;
    let unix = |time: io::Result<SystemTime>| {
        time.ok()
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|duration| iso_utc(duration.as_secs() as i64))
    };
    Ok(FileFacts {
        size_bytes: metadata.len() as i64,
        fs_created_at: unix(metadata.created()),
        fs_modified_at: unix(metadata.modified()),
    })
}

fn write(
    connection: &mut Connection,
    library_id: i64,
    run_id: i64,
    run_type: RunType,
    report: &DiscoveryReport,
    plan: Plan<'_>,
) -> Result<ScanSummary, ScanError> {
    let now = now_iso();
    let mut summary = ScanSummary {
        run_id,
        library_id,
        run_type,
        files_scanned: plan.items.len() as u32,
        added: 0,
        modified: 0,
        moved: 0,
        unchanged: 0,
        missing: 0,
        errored: 0,
        folders: report.folders.len() as u32,
    };
    let db = |_| ScanError::Database;
    let transaction = connection.transaction().map_err(db)?;

    sync_folders(&transaction, library_id, &report.folders, &now)?;

    // A file that is new here but has the size and hash of a gone row was moved or
    // renamed outside the app: keep the id so favorites, notes and albums follow it.
    let mut gone = plan.gone;
    for item in plan.items {
        let hit = item.hit;
        match item.action {
            Action::Unchanged { id } => {
                update_link(&transaction, id, hit, &now)?;
                summary.unchanged += 1;
            }
            Action::Update { id, hash } => {
                let facts = item.facts.as_ref().expect("hashed files were stat'ed");
                update_file(&transaction, id, hit, facts, &hash, &now)?;
                summary.modified += 1;
            }
            Action::Insert { hash } => {
                let facts = item.facts.as_ref().expect("hashed files were stat'ed");
                let twin = gone.iter().position(|row| {
                    row.hash.as_deref() == Some(hash.as_str())
                        && row.size_bytes == Some(facts.size_bytes)
                });
                if let Some(index) = twin {
                    let row = gone.remove(index);
                    update_file(&transaction, row.id, hit, facts, &hash, &now)?;
                    summary.moved += 1;
                } else {
                    insert_file(&transaction, library_id, hit, facts, &hash, &now)?;
                    summary.added += 1;
                }
            }
            Action::Error { id, message } => {
                match id {
                    Some(id) => {
                        transaction
                            .execute(
                                "UPDATE media_items SET indexing_status = 'error', indexing_error = ?2, updated_at = ?3
                                 WHERE id = ?1",
                                params![id, message, now],
                            )
                            .map_err(db)?;
                    }
                    None => {
                        insert_error_row(&transaction, library_id, hit, &message, &now)?;
                    }
                }
                record_error(
                    &transaction,
                    run_id,
                    &hit.relative_path,
                    "unreadable",
                    &message,
                    &now,
                )?;
                summary.errored += 1;
            }
        }
    }

    for row in &gone {
        if row.status == "missing_file" {
            continue;
        }
        transaction
            .execute(
                "UPDATE media_items SET indexing_status = 'missing_file', indexing_error = NULL, updated_at = ?2
                 WHERE id = ?1",
                params![row.id, now],
            )
            .map_err(db)?;
        summary.missing += 1;
    }

    for problem in &report.problems {
        let kind = problem_type(problem.kind);
        record_error(&transaction, run_id, &problem.relative_path, kind, "", &now)?;
    }

    transaction
        .execute(
            "UPDATE indexing_runs SET finished_at = ?2, status = 'completed',
               files_scanned = ?3, files_indexed = ?4, files_errored = ?5
             WHERE id = ?1",
            params![
                run_id,
                now,
                summary.files_scanned,
                summary.added + summary.modified + summary.moved,
                summary.errored,
            ],
        )
        .map_err(db)?;
    transaction
        .execute(
            "UPDATE libraries SET last_indexed_at = ?2 WHERE id = ?1",
            params![library_id, now],
        )
        .map_err(db)?;
    transaction.commit().map_err(db)?;
    Ok(summary)
}

/// Adds folders new on disk and drops rows for folders that are gone. Folders under one
/// that could not be listed are not reported by the walk, so they are kept.
fn sync_folders(
    transaction: &Transaction<'_>,
    library_id: i64,
    folders: &[String],
    now: &str,
) -> Result<(), ScanError> {
    let db = |_| ScanError::Database;
    let mut known: Vec<(i64, String)> = Vec::new();
    {
        let mut statement = transaction
            .prepare("SELECT id, relative_path FROM folders WHERE library_id = ?1")
            .map_err(db)?;
        let rows = statement
            .query_map([library_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(db)?;
        for row in rows {
            known.push(row.map_err(db)?);
        }
    }
    let on_disk: std::collections::HashSet<String> =
        folders.iter().map(|folder| folder.to_lowercase()).collect();
    for (id, path) in &known {
        if !on_disk.contains(&path.to_lowercase()) {
            transaction
                .execute("DELETE FROM folders WHERE id = ?1", [id])
                .map_err(db)?;
        }
    }
    for folder in folders {
        transaction
            .execute(
                "INSERT INTO folders (library_id, relative_path, created_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT (library_id, relative_path) DO UPDATE SET relative_path = excluded.relative_path",
                params![library_id, folder, now],
            )
            .map_err(db)?;
    }
    Ok(())
}

fn insert_file(
    transaction: &Transaction<'_>,
    library_id: i64,
    hit: &MediaHit,
    facts: &FileFacts,
    hash: &str,
    now: &str,
) -> Result<(), ScanError> {
    let (filename, extension) = name_parts(&hit.relative_path);
    transaction
        .execute(
            "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type,
               size_bytes, file_hash_sha256, fs_created_at, fs_modified_at,
               has_takeout_json, json_relative_path, json_match_confidence,
               indexing_status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'ok', ?13, ?13)",
            params![
                library_id,
                hit.relative_path,
                filename,
                extension,
                media_type(&extension),
                facts.size_bytes,
                hash,
                facts.fs_created_at,
                facts.fs_modified_at,
                hit.json_relative_path.is_some(),
                hit.json_relative_path,
                confidence(hit.confidence),
                now,
            ],
        )
        .map_err(|_| ScanError::Database)?;
    Ok(())
}

/// A file the scan could not read still gets a row, so the library shows it with its error.
fn insert_error_row(
    transaction: &Transaction<'_>,
    library_id: i64,
    hit: &MediaHit,
    message: &str,
    now: &str,
) -> Result<(), ScanError> {
    let (filename, extension) = name_parts(&hit.relative_path);
    transaction
        .execute(
            "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type,
               has_takeout_json, json_relative_path, json_match_confidence,
               indexing_status, indexing_error, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'error', ?9, ?10, ?10)",
            params![
                library_id,
                hit.relative_path,
                filename,
                extension,
                media_type(&extension),
                hit.json_relative_path.is_some(),
                hit.json_relative_path,
                confidence(hit.confidence),
                message,
                now,
            ],
        )
        .map_err(|_| ScanError::Database)?;
    Ok(())
}

/// Rewrites the file facts, path included (a move keeps the id), and clears any error.
fn update_file(
    transaction: &Transaction<'_>,
    id: i64,
    hit: &MediaHit,
    facts: &FileFacts,
    hash: &str,
    now: &str,
) -> Result<(), ScanError> {
    let (filename, extension) = name_parts(&hit.relative_path);
    transaction
        .execute(
            "UPDATE media_items SET relative_path = ?2, filename = ?3, extension = ?4, media_type = ?5,
               size_bytes = ?6, file_hash_sha256 = ?7, fs_created_at = ?8, fs_modified_at = ?9,
               has_takeout_json = ?10, json_relative_path = ?11, json_match_confidence = ?12,
               indexing_status = 'ok', indexing_error = NULL, updated_at = ?13
             WHERE id = ?1",
            params![
                id,
                hit.relative_path,
                filename,
                extension,
                media_type(&extension),
                facts.size_bytes,
                hash,
                facts.fs_created_at,
                facts.fs_modified_at,
                hit.json_relative_path.is_some(),
                hit.json_relative_path,
                confidence(hit.confidence),
                now,
            ],
        )
        .map_err(|_| ScanError::Database)?;
    Ok(())
}

/// An unchanged file can still gain or lose its sidecar, or change only the case of its
/// name, between scans.
fn update_link(
    transaction: &Transaction<'_>,
    id: i64,
    hit: &MediaHit,
    now: &str,
) -> Result<(), ScanError> {
    let (filename, _) = name_parts(&hit.relative_path);
    // `IS NOT` on the path ignores NOCASE, so a case-only rename is picked up too.
    transaction
        .execute(
            "UPDATE media_items SET has_takeout_json = ?2, json_relative_path = ?3,
               json_match_confidence = ?4, updated_at = ?5, relative_path = ?6, filename = ?7
             WHERE id = ?1 AND (has_takeout_json IS NOT ?2 OR json_relative_path IS NOT ?3
               OR json_match_confidence IS NOT ?4 OR relative_path IS NOT ?6 COLLATE BINARY)",
            params![
                id,
                hit.json_relative_path.is_some(),
                hit.json_relative_path,
                confidence(hit.confidence),
                now,
                hit.relative_path,
                filename,
            ],
        )
        .map_err(|_| ScanError::Database)?;
    Ok(())
}

fn record_error(
    transaction: &Transaction<'_>,
    run_id: i64,
    relative_path: &str,
    error_type: &str,
    message: &str,
    now: &str,
) -> Result<(), ScanError> {
    let message = (!message.is_empty()).then_some(message);
    transaction
        .execute(
            "INSERT INTO indexing_errors (run_id, relative_path, error_type, message, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![run_id, relative_path, error_type, message, now],
        )
        .map_err(|_| ScanError::Database)?;
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

fn confidence(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::Exact => "exact",
        Confidence::Heuristic => "heuristic",
        Confidence::None => "none",
    }
}

fn media_type(extension: &str) -> &'static str {
    if VIDEO_EXTENSIONS.contains(&extension) {
        "video"
    } else {
        "photo"
    }
}

/// `Fotos/IMG_1.JPG` → (`IMG_1.JPG`, `jpg`).
fn name_parts(relative_path: &str) -> (String, String) {
    let filename = relative_path.rsplit('/').next().unwrap_or(relative_path);
    let extension = filename
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    (filename.to_string(), extension)
}

fn absolute_from_relative(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

#[cfg(test)]
mod tests;
