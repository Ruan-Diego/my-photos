//! F05 scanner (SC-01..SC-08) against real temp folders and a migrated temp database.

use std::cell::Cell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use super::{scan, scan_with, RunType, ScanError};
use crate::bench::hash::{hash_reader, to_hex};
use crate::db::open_unchecked;

struct Library {
    root: PathBuf,
    connection: Connection,
}

impl Library {
    /// The database sits in `<root>/.myphotos`, as in production (AD-011).
    fn new(label: &str) -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "my-photos-f05-{label}-{}-{nanos}",
            std::process::id()
        ));
        fs::create_dir_all(root.join(".myphotos")).unwrap();
        let connection = open_unchecked(&root.join(".myphotos").join("library.db"));
        Self { root, connection }
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn path(&self, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(self.root.clone(), |path, part| path.join(part))
    }

    fn media(&self, relative: &str) -> Media {
        self.connection
            .query_row(
                "SELECT id, relative_path, filename, extension, media_type, size_bytes, file_hash_sha256,
                        has_takeout_json, json_relative_path, json_match_confidence, indexing_status,
                        fs_modified_at
                 FROM media_items WHERE relative_path = ?1",
                [relative],
                |row| {
                    Ok(Media {
                        id: row.get(0)?,
                        relative_path: row.get(1)?,
                        filename: row.get(2)?,
                        extension: row.get(3)?,
                        media_type: row.get(4)?,
                        size_bytes: row.get(5)?,
                        hash: row.get(6)?,
                        has_json: row.get(7)?,
                        json: row.get(8)?,
                        confidence: row.get(9)?,
                        status: row.get(10)?,
                        fs_modified_at: row.get(11)?,
                    })
                },
            )
            .unwrap_or_else(|error| panic!("{relative}: {error}"))
    }

    fn count(&self, sql: &str) -> i64 {
        self.connection
            .query_row(sql, [], |row| row.get(0))
            .unwrap()
    }

    fn folders(&self) -> Vec<String> {
        let mut statement = self
            .connection
            .prepare("SELECT relative_path FROM folders ORDER BY relative_path")
            .unwrap();
        let rows = statement.query_map([], |row| row.get(0)).unwrap();
        rows.map(Result::unwrap).collect()
    }

    fn errors(&self, run_id: i64) -> Vec<(String, String)> {
        let mut statement = self
            .connection
            .prepare(
                "SELECT relative_path, error_type FROM indexing_errors WHERE run_id = ?1
                 ORDER BY relative_path, error_type",
            )
            .unwrap();
        let rows = statement
            .query_map([run_id], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();
        rows.map(Result::unwrap).collect()
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[derive(Debug)]
struct Media {
    id: i64,
    relative_path: String,
    filename: String,
    extension: String,
    media_type: String,
    size_bytes: Option<i64>,
    hash: Option<String>,
    has_json: bool,
    json: Option<String>,
    confidence: Option<String>,
    status: String,
    fs_modified_at: Option<String>,
}

fn sha(bytes: &[u8]) -> String {
    to_hex(&hash_reader(bytes).unwrap())
}

/// Hashes for real and counts the calls.
fn counting(calls: &Cell<u32>) -> impl Fn(&Path) -> io::Result<[u8; 32]> + '_ {
    move |path| {
        calls.set(calls.get() + 1);
        hash_reader(fs::File::open(path)?)
    }
}

const SIDECAR: &[u8] = br#"{"title":"foto.jpg","photoTakenTime":{"timestamp":"1660142220"}}"#;

#[test]
fn sc01_first_scan_records_every_media_file_with_hash_type_and_sidecar() {
    let mut library = Library::new("first");
    library.write("Fotos de 2022/foto.jpg", b"jpeg bytes");
    library.write("Fotos de 2022/foto.jpg.json", SIDECAR);
    library.write("Fotos de 2022/clip.MP4", b"video bytes!");
    library.write("notas.txt", b"not media");
    fs::create_dir_all(library.path("Vazia")).unwrap();

    let summary = scan(&mut library.connection, &library.root).expect("scan");

    assert_eq!(summary.run_type, RunType::Full);
    assert_eq!(summary.files_scanned, 2);
    assert_eq!(summary.added, 2);
    assert_eq!(
        (summary.modified, summary.moved, summary.unchanged),
        (0, 0, 0)
    );
    assert_eq!((summary.missing, summary.errored), (0, 0));
    assert_eq!(summary.folders, 2);

    let photo = library.media("Fotos de 2022/foto.jpg");
    assert_eq!(photo.filename, "foto.jpg");
    assert_eq!(photo.extension, "jpg");
    assert_eq!(photo.media_type, "photo");
    assert_eq!(photo.size_bytes, Some(10));
    assert_eq!(photo.hash.as_deref(), Some(sha(b"jpeg bytes").as_str()));
    assert!(photo.has_json);
    assert_eq!(photo.json.as_deref(), Some("Fotos de 2022/foto.jpg.json"));
    assert_eq!(photo.confidence.as_deref(), Some("exact"));
    assert_eq!(photo.status, "ok");
    let modified = photo.fs_modified_at.expect("mtime");
    assert_eq!(modified.len(), 20, "{modified}");
    assert!(modified.ends_with('Z'));

    let clip = library.media("Fotos de 2022/clip.MP4");
    assert_eq!(clip.extension, "mp4");
    assert_eq!(clip.media_type, "video");
    assert!(!clip.has_json);
    assert_eq!(clip.json, None);
    assert_eq!(clip.confidence.as_deref(), Some("none"));

    assert_eq!(library.folders(), ["Fotos de 2022", "Vazia"]);
    let (root_path, last_indexed): (String, Option<String>) = library
        .connection
        .query_row(
            "SELECT root_path, last_indexed_at FROM libraries",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(root_path, library.root.to_string_lossy());
    assert!(last_indexed.is_some());
    let run: (String, String, i64, i64, i64, Option<String>) = library
        .connection
        .query_row(
            "SELECT run_type, status, files_scanned, files_indexed, files_errored, finished_at
             FROM indexing_runs WHERE id = ?1",
            [summary.run_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        (run.0.as_str(), run.1.as_str(), run.2, run.3, run.4),
        ("full", "completed", 2, 2, 0)
    );
    assert!(run.5.is_some());
}

#[test]
fn sc01_app_folder_is_never_scanned() {
    let mut library = Library::new("app-folder");
    library.write(".myphotos/trash/apagada.jpg", b"trashed");
    library.write("foto.jpg", b"kept");

    let summary = scan(&mut library.connection, &library.root).expect("scan");

    assert_eq!(summary.files_scanned, 1);
    assert_eq!(library.count("SELECT COUNT(*) FROM media_items"), 1);
    assert!(library.folders().is_empty());
}

#[test]
fn sc02_rescan_of_an_unchanged_library_hashes_nothing_and_keeps_ids() {
    let mut library = Library::new("unchanged");
    library.write("a.jpg", b"a");
    library.write("b/c.png", b"cc");
    scan(&mut library.connection, &library.root).expect("first");
    let before = (library.media("a.jpg").id, library.media("b/c.png").id);

    let calls = Cell::new(0);
    let summary =
        scan_with(&mut library.connection, &library.root, counting(&calls)).expect("again");

    assert_eq!(calls.get(), 0);
    assert_eq!(summary.run_type, RunType::Incremental);
    assert_eq!(summary.unchanged, 2);
    assert_eq!(
        (summary.added, summary.modified, summary.missing),
        (0, 0, 0)
    );
    assert_eq!(
        before,
        (library.media("a.jpg").id, library.media("b/c.png").id)
    );
    assert_eq!(library.count("SELECT COUNT(*) FROM indexing_runs"), 2);
    assert_eq!(library.count("SELECT COUNT(*) FROM libraries"), 1);
}

#[test]
fn sc03_changed_file_is_rehashed_under_the_same_id() {
    let mut library = Library::new("modified");
    library.write("a.jpg", b"first");
    scan(&mut library.connection, &library.root).expect("first");
    let id = library.media("a.jpg").id;

    library.write("a.jpg", b"second, longer");
    let calls = Cell::new(0);
    let summary =
        scan_with(&mut library.connection, &library.root, counting(&calls)).expect("again");

    assert_eq!(calls.get(), 1);
    assert_eq!(summary.modified, 1);
    let media = library.media("a.jpg");
    assert_eq!(media.id, id);
    assert_eq!(media.size_bytes, Some(14));
    assert_eq!(media.hash.as_deref(), Some(sha(b"second, longer").as_str()));
}

#[test]
fn sc04_removed_file_keeps_its_row_as_missing_and_comes_back_with_the_same_id() {
    let mut library = Library::new("missing");
    library.write("a.jpg", b"a");
    scan(&mut library.connection, &library.root).expect("first");
    let id = library.media("a.jpg").id;
    library
        .connection
        .execute("UPDATE media_items SET is_favorite = 1 WHERE id = ?1", [id])
        .unwrap();

    fs::remove_file(library.path("a.jpg")).unwrap();
    let summary = scan(&mut library.connection, &library.root).expect("gone");
    assert_eq!(summary.missing, 1);
    assert_eq!(library.media("a.jpg").status, "missing_file");

    // Still missing: not counted again.
    let summary = scan(&mut library.connection, &library.root).expect("still gone");
    assert_eq!(summary.missing, 0);

    library.write("a.jpg", b"a");
    let summary = scan(&mut library.connection, &library.root).expect("back");
    assert_eq!(summary.modified, 1);
    let media = library.media("a.jpg");
    assert_eq!((media.id, media.status.as_str()), (id, "ok"));
    assert_eq!(library.count("SELECT is_favorite FROM media_items"), 1);
}

#[test]
fn sc05_file_moved_outside_the_app_keeps_its_id() {
    let mut library = Library::new("moved");
    library.write("Antiga/foto.jpg", b"same bytes");
    library.write("Antiga/outra.jpg", b"other bytes");
    scan(&mut library.connection, &library.root).expect("first");
    let id = library.media("Antiga/foto.jpg").id;

    fs::create_dir_all(library.path("Nova")).unwrap();
    fs::rename(
        library.path("Antiga/foto.jpg"),
        library.path("Nova/renomeada.jpg"),
    )
    .unwrap();
    let summary = scan(&mut library.connection, &library.root).expect("again");

    assert_eq!((summary.moved, summary.added, summary.missing), (1, 0, 0));
    let media = library.media("Nova/renomeada.jpg");
    assert_eq!(media.id, id);
    assert_eq!(media.filename, "renomeada.jpg");
    assert_eq!(media.relative_path, "Nova/renomeada.jpg");
    assert_eq!(library.count("SELECT COUNT(*) FROM media_items"), 2);
    assert_eq!(library.folders(), ["Antiga", "Nova"]);
}

#[test]
fn sc05_copy_of_a_present_file_is_a_new_item() {
    let mut library = Library::new("copy");
    library.write("a.jpg", b"same");
    scan(&mut library.connection, &library.root).expect("first");
    library.write("b.jpg", b"same");

    let summary = scan(&mut library.connection, &library.root).expect("again");

    assert_eq!((summary.added, summary.moved), (1, 0));
    assert_ne!(library.media("a.jpg").id, library.media("b.jpg").id);
    assert_eq!(library.media("a.jpg").hash, library.media("b.jpg").hash);
}

#[test]
fn sc05_case_only_rename_updates_the_path() {
    let mut library = Library::new("case");
    library.write("foto.jpg", b"a");
    scan(&mut library.connection, &library.root).expect("first");
    let id = library.media("foto.jpg").id;

    fs::rename(library.path("foto.jpg"), library.path("FOTO.jpg")).unwrap();
    scan(&mut library.connection, &library.root).expect("again");

    let media = library.media("FOTO.jpg");
    assert_eq!(media.id, id);
    assert_eq!(media.relative_path, "FOTO.jpg");
    assert_eq!(media.filename, "FOTO.jpg");
}

#[test]
fn sc06_unreadable_file_gets_an_error_row_and_the_scan_goes_on() {
    let mut library = Library::new("unreadable");
    library.write("bad.jpg", b"bad");
    library.write("good.jpg", b"good");
    let failing = |path: &Path| {
        if path.ends_with("bad.jpg") {
            Err(io::Error::new(io::ErrorKind::Other, "disk error"))
        } else {
            hash_reader(fs::File::open(path)?)
        }
    };

    let summary = scan_with(&mut library.connection, &library.root, failing).expect("scan");

    assert_eq!((summary.added, summary.errored), (1, 1));
    let bad = library.media("bad.jpg");
    assert_eq!(bad.status, "error");
    assert_eq!(bad.hash, None);
    assert_eq!(
        library.errors(summary.run_id),
        [("bad.jpg".to_string(), "unreadable".to_string())]
    );
    let errored: i64 = library
        .connection
        .query_row(
            "SELECT files_errored FROM indexing_runs WHERE id = ?1",
            [summary.run_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(errored, 1);

    // The next scan that can read it heals the same row.
    let summary = scan(&mut library.connection, &library.root).expect("again");
    assert_eq!((summary.modified, summary.errored), (1, 0));
    let healed = library.media("bad.jpg");
    assert_eq!((healed.id, healed.status.as_str()), (bad.id, "ok"));
    assert_eq!(healed.hash.as_deref(), Some(sha(b"bad").as_str()));
}

#[test]
fn sc06_unreadable_known_file_keeps_its_row_and_hash() {
    let mut library = Library::new("unreadable-known");
    library.write("a.jpg", b"first");
    scan(&mut library.connection, &library.root).expect("first");
    library.write("a.jpg", b"changed bytes");

    let failing = |_: &Path| Err(io::Error::new(io::ErrorKind::Other, "disk error"));
    let summary = scan_with(&mut library.connection, &library.root, failing).expect("scan");

    assert_eq!(summary.errored, 1);
    let media = library.media("a.jpg");
    assert_eq!(media.status, "error");
    assert_eq!(media.hash.as_deref(), Some(sha(b"first").as_str()));
}

#[test]
fn sc07_discovery_problems_become_indexing_errors() {
    let mut library = Library::new("problems");
    library.write("orfao.jpg.json", SIDECAR);
    library.write("quebrada.jpg", b"x");
    library.write("quebrada.jpg.json", b"{not json");

    let summary = scan(&mut library.connection, &library.root).expect("scan");

    assert_eq!(
        library.errors(summary.run_id),
        [
            ("orfao.jpg.json".to_string(), "orphan_json".to_string()),
            ("quebrada.jpg.json".to_string(), "invalid_json".to_string()),
        ]
    );
    // Problems are reported, not counted as failed media.
    assert_eq!(summary.errored, 0);
    assert_eq!(library.media("quebrada.jpg").status, "ok");
}

#[test]
fn sc07_sidecar_added_later_links_to_the_unchanged_file() {
    let mut library = Library::new("late-sidecar");
    library.write("foto.jpg", b"a");
    scan(&mut library.connection, &library.root).expect("first");
    assert!(!library.media("foto.jpg").has_json);

    library.write("foto.jpg.json", SIDECAR);
    let calls = Cell::new(0);
    let summary =
        scan_with(&mut library.connection, &library.root, counting(&calls)).expect("again");

    assert_eq!((calls.get(), summary.unchanged), (0, 1));
    let media = library.media("foto.jpg");
    assert!(media.has_json);
    assert_eq!(media.json.as_deref(), Some("foto.jpg.json"));
    assert_eq!(media.confidence.as_deref(), Some("exact"));
}

#[test]
fn sc08_missing_root_fails_without_writing_anything() {
    let mut library = Library::new("no-root");
    let absent = library.root.join("desconectado");

    let error = scan(&mut library.connection, &absent).unwrap_err();

    assert_eq!(error, ScanError::RootUnreadable);
    assert_eq!(error.as_code(), "root_unreadable");
    assert_eq!(library.count("SELECT COUNT(*) FROM libraries"), 0);
    assert_eq!(library.count("SELECT COUNT(*) FROM indexing_runs"), 0);
}

#[test]
fn sc08_deleted_folder_row_is_dropped() {
    let mut library = Library::new("folders");
    fs::create_dir_all(library.path("Vazia")).unwrap();
    fs::create_dir_all(library.path("Fica")).unwrap();
    scan(&mut library.connection, &library.root).expect("first");
    let kept: i64 = library
        .connection
        .query_row(
            "SELECT id FROM folders WHERE relative_path = 'Fica'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    fs::remove_dir(library.path("Vazia")).unwrap();
    let summary = scan(&mut library.connection, &library.root).expect("again");

    assert_eq!(summary.folders, 1);
    assert_eq!(library.folders(), ["Fica"]);
    let still: i64 = library
        .connection
        .query_row(
            "SELECT id FROM folders WHERE relative_path = 'Fica'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(still, kept);
}

#[test]
fn error_codes_and_messages() {
    assert_eq!(ScanError::RootUnreadable.as_code(), "root_unreadable");
    assert_eq!(
        ScanError::RootUnreadable.message(),
        "não foi possível ler a pasta da biblioteca"
    );
    assert_eq!(ScanError::Database.as_code(), "scan_db_failed");
    assert_eq!(
        ScanError::Database.message(),
        "falha ao gravar a varredura no banco"
    );
}

#[test]
fn second_library_root_gets_its_own_row() {
    let mut library = Library::new("two-roots");
    library.write("A/a.jpg", b"a");
    library.write("B/b.jpg", b"b");
    let first = scan(&mut library.connection, &library.root.join("A")).expect("A");
    let second = scan(&mut library.connection, &library.root.join("B")).expect("B");

    assert_ne!(first.library_id, second.library_id);
    assert_eq!(second.run_type, RunType::Full);
    let owner: i64 = library
        .connection
        .query_row(
            "SELECT library_id FROM media_items WHERE relative_path = 'b.jpg'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(owner, second.library_id);
}
