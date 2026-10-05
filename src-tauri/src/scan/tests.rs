//! SCAN-01..06 on a temp tree and a temp v1 database. Never `E:\`.

use std::cell::RefCell;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::Connection;

use super::walk::{absolute, Entry};
use super::{scan_with, FileSystem, RealFs, ScanError, ScanSummary};
use crate::db::tests::Scratch;
use crate::db::MIGRATIONS;

const HELLO_SHA256: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const T1: &str = "2026-10-05T10:00:00.000Z";
const T2: &str = "2026-10-05T11:00:00.000Z";
const T3: &str = "2026-10-05T12:00:00.000Z";

struct Library {
    scratch: Scratch,
    root: PathBuf,
    connection: Connection,
}

impl Library {
    fn new(label: &str) -> Self {
        let scratch = Scratch::new(&format!("f05-{label}"));
        let root = scratch.path.join("root");
        fs::create_dir_all(&root).unwrap();
        let connection = database(&scratch.path.join("library.db"));
        Self {
            scratch,
            root,
            connection,
        }
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = absolute(&self.root, relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn scan(&mut self, now: &str) -> ScanSummary {
        scan_with(&mut self.connection, &self.root, &RealFs, now).expect("scan")
    }

    fn scan_using(&mut self, fs: &dyn FileSystem, now: &str) -> Result<ScanSummary, ScanError> {
        scan_with(&mut self.connection, &self.root, fs, now)
    }

    fn media(&self, relative: &str) -> Media {
        self.connection
            .query_row(
                "SELECT id, relative_path, filename, extension, media_type, size_bytes, file_hash_sha256,
                        fs_modified_at, indexing_status, indexing_error, created_at, updated_at, library_id
                 FROM media_items WHERE relative_path = ?1",
                [relative],
                |row| {
                    Ok(Media {
                        id: row.get(0)?,
                        relative_path: row.get(1)?,
                        filename: row.get(2)?,
                        extension: row.get(3)?,
                        media_type: row.get(4)?,
                        size: row.get(5)?,
                        hash: row.get(6)?,
                        modified: row.get(7)?,
                        status: row.get(8)?,
                        error: row.get(9)?,
                        created_at: row.get(10)?,
                        updated_at: row.get(11)?,
                        library_id: row.get(12)?,
                    })
                },
            )
            .unwrap_or_else(|_| panic!("no media row for {relative}"))
    }

    fn strings(&self, sql: &str) -> Vec<String> {
        let mut statement = self.connection.prepare(sql).unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn count(&self, table: &str) -> i64 {
        self.connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    /// Every column of every media and folder row, for "nothing changed" checks.
    fn dump(&self) -> Vec<String> {
        let mut rows = self.strings(
            "SELECT quote(id)||'|'||relative_path||'|'||quote(size_bytes)||'|'||quote(file_hash_sha256)||'|'||
                    quote(fs_modified_at)||'|'||quote(fs_created_at)||'|'||indexing_status||'|'||
                    quote(indexing_error)||'|'||created_at||'|'||updated_at
             FROM media_items ORDER BY id",
        );
        rows.extend(self.strings(
            "SELECT 'folder|'||id||'|'||relative_path||'|'||created_at FROM folders ORDER BY id",
        ));
        rows
    }

    fn run(&self, run_id: i64) -> (String, String, Option<String>, i64, i64, i64) {
        self.connection
            .query_row(
                "SELECT run_type, status, finished_at, files_scanned, files_indexed, files_errored
                 FROM indexing_runs WHERE id = ?1",
                [run_id],
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
            .unwrap()
    }

    fn errors(&self, run_id: i64) -> Vec<String> {
        self.strings(&format!(
            "SELECT error_type||' '||relative_path FROM indexing_errors WHERE run_id = {run_id} ORDER BY id"
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Media {
    id: i64,
    relative_path: String,
    filename: String,
    extension: String,
    media_type: String,
    size: Option<i64>,
    hash: Option<String>,
    modified: Option<String>,
    status: String,
    error: Option<String>,
    created_at: String,
    updated_at: String,
    library_id: i64,
}

fn database(path: &Path) -> Connection {
    let connection = Connection::open(path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .unwrap();
    for migration in MIGRATIONS {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
}

fn set_modified(path: &Path, time: SystemTime) {
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(time)
        .unwrap();
}

fn modified(path: &Path) -> SystemTime {
    fs::metadata(path).unwrap().modified().unwrap()
}

type ListHook = Box<dyn Fn(&Path) -> bool>;

/// Records every file opened; can fail opens, fail listings, or run a hook first.
#[derive(Default)]
struct Probe {
    opened: RefCell<Vec<String>>,
    fail_open: Vec<&'static str>,
    fail_list: Vec<&'static str>,
    on_open: Option<Box<dyn Fn(usize) -> bool>>,
    on_list: Option<ListHook>,
}

impl FileSystem for Probe {
    fn list(&self, dir: &Path) -> io::Result<Vec<Entry>> {
        if self.fail_list.iter().any(|name| dir.ends_with(name)) {
            return Err(io::Error::other("denied"));
        }
        if self.on_list.as_ref().is_some_and(|hook| hook(dir)) {
            return Err(io::Error::other("gone"));
        }
        RealFs.list(dir)
    }

    fn open(&self, file: &Path) -> io::Result<Box<dyn Read>> {
        let name = file.file_name().unwrap().to_string_lossy().into_owned();
        self.opened.borrow_mut().push(name.clone());
        if self.fail_open.contains(&name.as_str()) {
            return Err(io::Error::other("unreadable"));
        }
        let count = self.opened.borrow().len();
        if self.on_open.as_ref().is_some_and(|hook| hook(count)) {
            return Err(io::Error::other("gone"));
        }
        RealFs.open(file)
    }
}

// ---------------------------------------------------------------- SCAN-01

#[test]
fn error_codes_and_messages_match_the_design() {
    let expected = [
        (
            ScanError::RootNotAbsolute,
            "root_not_absolute",
            "pasta da biblioteca não é um caminho absoluto",
        ),
        (
            ScanError::RootMissing,
            "root_missing",
            "pasta da biblioteca não encontrada",
        ),
        (
            ScanError::RootNotDirectory,
            "root_not_directory",
            "a biblioteca não é uma pasta",
        ),
        (
            ScanError::DbWriteFailed,
            "db_write_failed",
            "falha ao gravar no banco",
        ),
    ];
    for (error, code, message) in expected {
        assert_eq!(error.as_code(), code);
        assert_eq!(error.message(), message);
    }
}

#[test]
fn invalid_roots_write_nothing_anywhere() {
    let mut library = Library::new("invalid-root");
    library.write("arquivo.jpg", b"x");
    let file = library.root.join("arquivo.jpg");
    let missing = library.scratch.path.join("absent");
    let cases: [(&Path, &str); 3] = [
        (Path::new("relative/root"), "root_not_absolute"),
        (&missing, "root_missing"),
        (&file, "root_not_directory"),
    ];
    for (root, code) in cases {
        let error = scan_with(&mut library.connection, root, &RealFs, T1).unwrap_err();
        assert_eq!(error.as_code(), code);
    }
    assert!(!missing.exists());
    for table in [
        "libraries",
        "indexing_runs",
        "indexing_errors",
        "folders",
        "media_items",
    ] {
        assert_eq!(library.count(table), 0, "{table}");
    }
}

#[test]
fn first_run_creates_the_library_and_a_completed_full_run() {
    let mut library = Library::new("first-run");
    library.write("a.jpg", b"hello");
    let summary = library.scan(T1);
    let root_path = library.root.to_string_lossy().into_owned();
    let (stored_root, last_indexed, status, name): (String, Option<String>, String, String) =
        library
            .connection
            .query_row(
                "SELECT root_path, last_indexed_at, status, name FROM libraries",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
    // `E:\` is named `E:`; any root is named after itself without the trailing separator.
    assert_eq!(name, root_path.trim_end_matches(['\\', '/']));
    assert_eq!(library.count("libraries"), 1);
    assert_eq!(stored_root, root_path);
    assert_eq!(last_indexed.as_deref(), Some(T1));
    assert_eq!(status, "active");
    assert_eq!(summary.run_type, "full");
    let (run_type, status, finished, ..) = library.run(summary.run_id);
    assert_eq!(run_type, "full");
    assert_eq!(status, "completed");
    assert_eq!(finished.as_deref(), Some(T1));
    assert_eq!(library.media("a.jpg").library_id, 1);

    let second = library.scan(T2);
    assert_eq!(library.count("libraries"), 1);
    assert_eq!(library.run(second.run_id).0, "incremental");
}

#[test]
fn empty_root_completes_with_nothing() {
    let mut library = Library::new("empty");
    let summary = library.scan(T1);
    assert_eq!((summary.folders, summary.media, summary.errors), (0, 0, 0));
    assert_eq!(library.run(summary.run_id).1, "completed");
    assert_eq!(library.count("folders"), 0);
    assert_eq!(library.count("media_items"), 0);
}

// ---------------------------------------------------------------- SCAN-02

#[test]
fn folders_are_recorded_as_on_disk_without_the_root() {
    let mut library = Library::new("folders");
    library.write(
        "Takeout/takeout-20260921T183943Z-1-001/Google Fotos/a.jpg",
        b"a",
    );
    library.write(
        "Takeout/takeout-20260921T183943Z-1-002/Google Fotos/a.jpg.json",
        b"{}",
    );
    fs::create_dir_all(library.root.join("Vazia")).unwrap();
    library.scan(T1);
    assert_eq!(
        library.strings("SELECT relative_path FROM folders ORDER BY relative_path"),
        [
            "Takeout",
            "Takeout/takeout-20260921T183943Z-1-001",
            "Takeout/takeout-20260921T183943Z-1-001/Google Fotos",
            "Takeout/takeout-20260921T183943Z-1-002",
            "Takeout/takeout-20260921T183943Z-1-002/Google Fotos",
            "Vazia",
        ]
    );
    assert_eq!(
        library.strings("SELECT DISTINCT created_at FROM folders"),
        [T1]
    );
}

#[test]
fn media_rows_carry_file_facts_and_the_sha256() {
    let mut library = Library::new("media");
    library.write("2020/IMG_1.JPG", b"hello");
    library.write("2020/clip.MoV", b"");
    let summary = library.scan(T1);

    let photo = library.media("2020/IMG_1.JPG");
    assert_eq!(photo.filename, "IMG_1.JPG");
    assert_eq!(photo.extension, "jpg");
    assert_eq!(photo.media_type, "photo");
    assert_eq!(photo.size, Some(5));
    assert_eq!(photo.hash.as_deref(), Some(HELLO_SHA256));
    assert_eq!(photo.status, "ok");
    assert_eq!(photo.error, None);
    assert_eq!(
        (photo.created_at.as_str(), photo.updated_at.as_str()),
        (T1, T1)
    );
    let expected_modified =
        super::time::iso_utc(modified(&library.root.join("2020").join("IMG_1.JPG"))).unwrap();
    assert_eq!(photo.modified.as_deref(), Some(expected_modified.as_str()));
    let text = photo.modified.unwrap();
    assert_eq!(text.len(), "2026-10-02T12:00:00.123Z".len());
    assert!(text.ends_with('Z'));

    let video = library.media("2020/clip.MoV");
    assert_eq!(video.extension, "mov");
    assert_eq!(video.media_type, "video");
    assert_eq!(video.size, Some(0));
    assert_eq!(video.hash.as_deref(), Some(EMPTY_SHA256));

    assert_eq!(
        (
            summary.media,
            summary.new,
            summary.hashed,
            summary.bytes_hashed
        ),
        (2, 2, 2, 5)
    );
}

#[test]
fn every_media_extension_maps_to_its_type() {
    let mut library = Library::new("types");
    let videos = ["mp4", "mov", "mkv", "avi", "3gp", "webm", "m4v"];
    let photos = [
        "jpg", "jpeg", "png", "heic", "heif", "gif", "webp", "dng", "cr2", "nef", "arw",
    ];
    for extension in videos.iter().chain(photos.iter()) {
        library.write(&format!("f.{}", extension.to_ascii_uppercase()), b"x");
    }
    library.scan(T1);
    for extension in videos {
        let media = library.media(&format!("f.{}", extension.to_ascii_uppercase()));
        assert_eq!(
            (media.extension.as_str(), media.media_type.as_str()),
            (extension, "video")
        );
    }
    for extension in photos {
        let media = library.media(&format!("f.{}", extension.to_ascii_uppercase()));
        assert_eq!(
            (media.extension.as_str(), media.media_type.as_str()),
            (extension, "photo")
        );
    }
}

#[test]
fn non_media_files_leave_no_rows_and_no_errors() {
    let mut library = Library::new("non-media");
    library.write("a.jpg.json", b"{}");
    library.write("metadata.json", b"{}");
    library.write("takeout.zip", b"z");
    library.write("archive_browser.html", b"h");
    library.write("README", b"r");
    let summary = library.scan(T1);
    assert_eq!(library.count("media_items"), 0);
    assert_eq!(library.count("indexing_errors"), 0);
    assert_eq!(summary.media, 0);
}

#[test]
fn app_folder_at_the_root_is_skipped_in_any_case() {
    let mut library = Library::new("app-folder");
    library.write(".MYPHOTOS/trash/x.jpg", b"x");
    library.write("sub/.myphotos/y.jpg", b"y");
    library.scan(T1);
    assert_eq!(
        library.strings("SELECT relative_path FROM media_items"),
        ["sub/.myphotos/y.jpg"]
    );
    assert_eq!(
        library.strings("SELECT relative_path FROM folders ORDER BY relative_path"),
        ["sub", "sub/.myphotos"]
    );
}

#[cfg(unix)]
#[test]
fn links_are_neither_followed_nor_recorded() {
    let mut library = Library::new("links");
    let outside = library.scratch.path.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("secret.jpg"), b"s").unwrap();
    std::os::unix::fs::symlink(&outside, library.root.join("dir-link")).unwrap();
    std::os::unix::fs::symlink(outside.join("secret.jpg"), library.root.join("file.jpg")).unwrap();
    library.scan(T1);
    assert_eq!(library.count("media_items"), 0);
    assert_eq!(library.count("folders"), 0);
}

// ---------------------------------------------------------------- SCAN-03

/// `(path, is_dir, len, modified, bytes)` for everything under `root` except `.myphotos`.
fn snapshot(root: &Path) -> Vec<(String, bool, u64, SystemTime, Vec<u8>)> {
    fn visit(dir: &Path, relative: &str, out: &mut Vec<(String, bool, u64, SystemTime, Vec<u8>)>) {
        let mut entries: Vec<_> = fs::read_dir(dir).unwrap().map(Result::unwrap).collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            if relative.is_empty() && name.eq_ignore_ascii_case(".myphotos") {
                continue;
            }
            let path = super::walk::join(relative, &name);
            let meta = fs::symlink_metadata(entry.path()).unwrap();
            let bytes = if meta.is_file() {
                fs::read(entry.path()).unwrap()
            } else {
                Vec::new()
            };
            out.push((
                path.clone(),
                meta.is_dir(),
                meta.len(),
                meta.modified().unwrap(),
                bytes,
            ));
            if meta.is_dir() {
                visit(&entry.path(), &path, out);
            }
        }
    }
    let mut out = Vec::new();
    visit(root, "", &mut out);
    out
}

#[test]
fn scanning_never_changes_the_library() {
    let scratch = Scratch::new("f05-readonly");
    let root = scratch.path.join("root");
    for (relative, bytes) in [
        ("Takeout/Fotos/a.jpg", &b"hello"[..]),
        ("Takeout/Fotos/a.jpg.json", b"{}"),
        ("Takeout/Fotos/b.mp4", b"video"),
        ("Outros/c.png", b""),
        ("notas.txt", b"n"),
    ] {
        let path = absolute(&root, relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
    fs::create_dir_all(root.join("Vazia")).unwrap();
    // The production layout: the database inside the library (AD-011).
    fs::create_dir_all(root.join(".myphotos")).unwrap();
    let mut connection = database(&root.join(".myphotos").join("library.db"));

    let before = snapshot(&root);
    let first = scan_with(&mut connection, &root, &RealFs, T1).unwrap();
    assert_eq!(snapshot(&root), before);
    fs::remove_file(root.join("Outros/c.png")).unwrap();
    let before = snapshot(&root);
    scan_with(&mut connection, &root, &RealFs, T2).unwrap();
    assert_eq!(snapshot(&root), before);
    assert_eq!(first.media, 3);
}

// ---------------------------------------------------------------- SCAN-04

#[test]
fn second_run_without_changes_reads_nothing_and_changes_nothing() {
    let mut library = Library::new("idle");
    library.write("a/1.jpg", b"one");
    library.write("a/2.mp4", b"two");
    fs::create_dir_all(library.root.join("b")).unwrap();
    library.scan(T1);
    let before = library.dump();

    let probe = Probe::default();
    let summary = library.scan_using(&probe, T2).unwrap();
    assert!(probe.opened.borrow().is_empty());
    assert_eq!(library.dump(), before);
    assert_eq!(summary.run_type, "incremental");
    assert_eq!(
        (
            summary.unchanged,
            summary.new,
            summary.modified,
            summary.hashed
        ),
        (2, 0, 0, 0)
    );
    assert_eq!(library.run(summary.run_id).0, "incremental");
}

#[test]
fn new_media_gets_a_row_and_old_ids_stay() {
    let mut library = Library::new("new");
    library.write("a.jpg", b"a");
    library.scan(T1);
    let id = library.media("a.jpg").id;
    library.write("b.jpg", b"hello");
    let probe = Probe::default();
    let summary = library.scan_using(&probe, T2).unwrap();
    assert_eq!(*probe.opened.borrow(), ["b.jpg"]);
    assert_eq!(library.media("a.jpg").id, id);
    let new = library.media("b.jpg");
    assert!(new.id > id);
    assert_eq!(new.hash.as_deref(), Some(HELLO_SHA256));
    assert_eq!(new.created_at, T2);
    assert_eq!((summary.new, summary.unchanged), (1, 1));
}

#[test]
fn modified_content_size_or_date_rehashes_only_that_file_under_the_same_id() {
    let mut library = Library::new("modified");
    library.write("same.jpg", b"same");
    library.write("size.jpg", b"x");
    library.write("date.jpg", b"hello");
    library.scan(T1);
    let size_before = library.media("size.jpg");
    let date_before = library.media("date.jpg");

    library.write("size.jpg", b"hello");
    let date_path = library.root.join("date.jpg");
    set_modified(
        &date_path,
        modified(&date_path) + Duration::from_millis(1500),
    );

    let probe = Probe::default();
    let summary = library.scan_using(&probe, T2).unwrap();
    assert_eq!(*probe.opened.borrow(), ["date.jpg", "size.jpg"]);

    let size_after = library.media("size.jpg");
    assert_eq!(size_after.id, size_before.id);
    assert_eq!(size_after.size, Some(5));
    assert_eq!(size_after.hash.as_deref(), Some(HELLO_SHA256));
    assert_eq!(size_after.updated_at, T2);
    assert_eq!(size_after.created_at, T1);

    let date_after = library.media("date.jpg");
    assert_eq!(date_after.id, date_before.id);
    assert_ne!(date_after.modified, date_before.modified);
    assert_eq!(date_after.hash.as_deref(), Some(HELLO_SHA256));
    assert_eq!(date_after.updated_at, T2);

    assert_eq!(library.media("same.jpg").updated_at, T1);
    assert_eq!(
        (summary.modified, summary.unchanged, summary.hashed),
        (2, 1, 2)
    );
}

#[test]
fn removed_media_turns_missing_and_comes_back_with_the_same_id() {
    let mut library = Library::new("missing");
    library.write("a.jpg", b"hello");
    library.write("b.jpg", b"b");
    library.scan(T1);
    let a_path = library.root.join("a.jpg");
    let a_bytes = fs::read(&a_path).unwrap();
    let a_time = modified(&a_path);
    let a = library.media("a.jpg");
    let b_bytes = fs::read(library.root.join("b.jpg")).unwrap();
    let b = library.media("b.jpg");
    fs::remove_file(&a_path).unwrap();
    fs::remove_file(library.root.join("b.jpg")).unwrap();

    let summary = library.scan(T2);
    for before in [&a, &b] {
        let after = library.media(&before.relative_path);
        assert_eq!(after.id, before.id);
        assert_eq!(after.status, "missing_file");
        assert_eq!(after.hash, before.hash);
        assert_eq!(after.updated_at, T2);
    }
    assert_eq!(summary.missing, 2);

    // A third run with the files still gone does not count them again.
    assert_eq!(library.scan(T3).missing, 0);

    // a.jpg returns untouched; b.jpg returns with a new date.
    fs::write(&a_path, &a_bytes).unwrap();
    set_modified(&a_path, a_time);
    fs::write(library.root.join("b.jpg"), &b_bytes).unwrap();
    set_modified(
        &library.root.join("b.jpg"),
        a_time + Duration::from_secs(60),
    );
    let probe = Probe::default();
    let summary = library
        .scan_using(&probe, "2026-10-05T13:00:00.000Z")
        .unwrap();
    assert_eq!(*probe.opened.borrow(), ["b.jpg"]);
    let a_back = library.media("a.jpg");
    assert_eq!((a_back.id, a_back.status.as_str()), (a.id, "ok"));
    assert_eq!(a_back.hash.as_deref(), Some(HELLO_SHA256));
    let b_back = library.media("b.jpg");
    assert_eq!((b_back.id, b_back.status.as_str()), (b.id, "ok"));
    assert_eq!(
        (summary.restored, summary.modified, summary.unchanged),
        (2, 1, 0)
    );
}

#[test]
fn folders_follow_the_disk() {
    let mut library = Library::new("folders-follow");
    fs::create_dir_all(library.root.join("velha/sub")).unwrap();
    fs::create_dir_all(library.root.join("fica")).unwrap();
    library.scan(T1);
    let kept: i64 = library
        .connection
        .query_row(
            "SELECT id FROM folders WHERE relative_path = 'fica'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    fs::remove_dir_all(library.root.join("velha")).unwrap();
    fs::create_dir_all(library.root.join("nova")).unwrap();
    library.scan(T2);
    assert_eq!(
        library.strings("SELECT relative_path FROM folders ORDER BY relative_path"),
        ["fica", "nova"]
    );
    let still: i64 = library
        .connection
        .query_row(
            "SELECT id FROM folders WHERE relative_path = 'fica'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(still, kept);
}

#[test]
fn missing_hash_is_read_again_even_when_size_and_date_match() {
    let mut library = Library::new("null-hash");
    library.write("a.jpg", b"hello");
    library.scan(T1);
    library
        .connection
        .execute("UPDATE media_items SET file_hash_sha256 = NULL", [])
        .unwrap();
    let probe = Probe::default();
    let summary = library.scan_using(&probe, T2).unwrap();
    assert_eq!(*probe.opened.borrow(), ["a.jpg"]);
    assert_eq!(library.media("a.jpg").hash.as_deref(), Some(HELLO_SHA256));
    assert_eq!((summary.hashed, summary.unchanged), (1, 0));
}

#[test]
fn run_counters_and_summary_agree() {
    let mut library = Library::new("counters");
    library.write("keep.jpg", b"k");
    library.write("edit.jpg", b"e");
    library.write("gone.jpg", b"g");
    library.write("bad.jpg", b"b");
    let first = library.scan(T1);
    let (.., scanned, indexed, errored) = library.run(first.run_id);
    assert_eq!((scanned, indexed, errored), (4, 4, 0));

    library.write("edit.jpg", b"edited");
    fs::remove_file(library.root.join("gone.jpg")).unwrap();
    library.write("new.jpg", b"n");
    library.write("bad.jpg", b"bb");
    let probe = Probe {
        fail_open: vec!["bad.jpg"],
        ..Probe::default()
    };
    let summary = library.scan_using(&probe, T2).unwrap();
    assert_eq!(
        summary,
        ScanSummary {
            run_id: summary.run_id,
            run_type: "incremental".to_string(),
            folders: 0,
            media: 4,
            new: 1,
            modified: 2,
            unchanged: 1,
            missing: 1,
            restored: 0,
            hashed: 2,
            errors: 1,
            bytes_hashed: 7,
        }
    );
    let (_, status, finished, scanned, indexed, errored) = library.run(summary.run_id);
    assert_eq!(
        (status.as_str(), finished.as_deref()),
        ("completed", Some(T2))
    );
    assert_eq!((scanned, indexed, errored), (4, 2, 1));
}

#[test]
fn case_only_rename_updates_the_path_and_keeps_the_id() {
    let mut library = Library::new("case");
    library.write("Foto.JPG", b"hello");
    library.scan(T1);
    let before = library.media("Foto.JPG");
    fs::rename(library.root.join("Foto.JPG"), library.root.join("foto.jpg")).unwrap();
    let probe = Probe::default();
    library.scan_using(&probe, T2).unwrap();
    assert!(probe.opened.borrow().is_empty());
    assert_eq!(library.count("media_items"), 1);
    let after = library.media("foto.jpg");
    assert_eq!(after.id, before.id);
    assert_eq!(
        (after.relative_path.as_str(), after.filename.as_str()),
        ("foto.jpg", "foto.jpg")
    );
    assert_eq!(after.status, "ok");
}

// ---------------------------------------------------------------- SCAN-05

#[test]
fn unreadable_file_is_an_error_row_and_is_retried_next_run() {
    let mut library = Library::new("unreadable");
    library.write("bad.jpg", b"hello");
    library.write("good.jpg", b"g");
    let probe = Probe {
        fail_open: vec!["bad.jpg"],
        ..Probe::default()
    };
    let summary = library.scan_using(&probe, T1).unwrap();
    assert_eq!(*probe.opened.borrow(), ["bad.jpg", "good.jpg"]);
    let bad = library.media("bad.jpg");
    assert_eq!(bad.status, "error");
    assert_eq!(bad.error.as_deref(), Some("unreadable"));
    assert_eq!(bad.hash, None);
    assert!(library.media("good.jpg").hash.is_some());
    assert_eq!(library.errors(summary.run_id), ["unreadable bad.jpg"]);
    assert_eq!(library.run(summary.run_id).1, "completed");

    let probe = Probe::default();
    let retry = library.scan_using(&probe, T2).unwrap();
    assert_eq!(*probe.opened.borrow(), ["bad.jpg"]);
    let fixed = library.media("bad.jpg");
    assert_eq!(fixed.id, bad.id);
    assert_eq!(fixed.status, "ok");
    assert_eq!(fixed.error, None);
    assert_eq!(fixed.hash.as_deref(), Some(HELLO_SHA256));
    assert!(library.errors(retry.run_id).is_empty());
}

#[test]
fn unreadable_folder_is_reported_and_its_known_contents_stay() {
    let mut library = Library::new("denied-dir");
    library.write("trancada/sub/a.jpg", b"a");
    library.write("livre/b.jpg", b"b");
    library.scan(T1);
    let before = library.dump();
    library.write("livre/c.jpg", b"c");

    let probe = Probe {
        fail_list: vec!["trancada"],
        ..Probe::default()
    };
    let summary = library.scan_using(&probe, T2).unwrap();
    assert_eq!(
        library.errors(summary.run_id),
        ["unreadable_directory trancada"]
    );
    assert_eq!(library.media("trancada/sub/a.jpg").status, "ok");
    assert_eq!(summary.missing, 0);
    assert_eq!(
        library.strings("SELECT relative_path FROM folders ORDER BY relative_path"),
        ["livre", "trancada", "trancada/sub"]
    );
    assert!(library.media("livre/c.jpg").hash.is_some());
    // Every row from before is still identical.
    let after = library.dump();
    assert!(before.iter().all(|row| after.contains(row)));
}

#[cfg(target_os = "linux")]
#[test]
fn invalid_unicode_name_is_skipped_with_an_error() {
    use std::os::unix::ffi::OsStrExt;
    let mut library = Library::new("bad-name");
    fs::write(
        library
            .root
            .join(std::ffi::OsStr::from_bytes(b"bad\xff.jpg")),
        b"x",
    )
    .unwrap();
    library.write("ok.jpg", b"o");
    let summary = library.scan(T1);
    assert_eq!(
        library.errors(summary.run_id),
        ["invalid_name bad\u{fffd}.jpg"]
    );
    assert_eq!(
        library.strings("SELECT relative_path FROM media_items"),
        ["ok.jpg"]
    );
}

// ---------------------------------------------------------------- SCAN-06

#[test]
fn root_vanishing_while_hashing_fails_the_run_and_keeps_what_was_saved() {
    let mut library = Library::new("vanish-hash");
    library.write("a.jpg", b"hello");
    library.write("b.jpg", b"b");
    library.write("c.jpg", b"c");
    let root = library.root.clone();
    let away = library.scratch.path.join("away");
    let (root_hook, away_hook) = (root.clone(), away.clone());
    let probe = Probe {
        on_open: Some(Box::new(move |count| {
            if count == 2 {
                fs::rename(&root_hook, &away_hook).unwrap();
                return true;
            }
            false
        })),
        ..Probe::default()
    };
    let error = library.scan_using(&probe, T1).unwrap_err();
    assert_eq!(error, ScanError::RootMissing);
    let run_id: i64 = library
        .connection
        .query_row("SELECT max(id) FROM indexing_runs", [], |r| r.get(0))
        .unwrap();
    let (_, status, finished, ..) = library.run(run_id);
    assert_eq!((status.as_str(), finished.as_deref()), ("failed", Some(T1)));
    assert_eq!(library.media("a.jpg").hash.as_deref(), Some(HELLO_SHA256));
    for name in ["b.jpg", "c.jpg"] {
        let media = library.media(name);
        assert_eq!((media.status.as_str(), media.hash), ("ok", None));
    }
    assert!(library.errors(run_id).is_empty());

    // Reconnected: only the files without a hash are read; still "full", nothing completed yet.
    fs::rename(&away, &root).unwrap();
    let probe = Probe::default();
    let summary = library.scan_using(&probe, T2).unwrap();
    assert_eq!(*probe.opened.borrow(), ["b.jpg", "c.jpg"]);
    assert_eq!(summary.run_type, "full");
    assert_eq!(summary.missing, 0);
}

#[test]
fn root_vanishing_while_listing_marks_nothing_missing() {
    let mut library = Library::new("vanish-walk");
    library.write("a/x.jpg", b"x");
    library.write("z.jpg", b"z");
    library.scan(T1);
    let before = library.dump();
    let root = library.root.clone();
    let away = library.scratch.path.join("away");
    let (root_hook, away_hook) = (root.clone(), away.clone());
    let probe = Probe {
        on_list: Some(Box::new(move |dir| {
            if dir.ends_with("a") {
                fs::rename(&root_hook, &away_hook).unwrap();
                return true;
            }
            false
        })),
        ..Probe::default()
    };
    let error = library.scan_using(&probe, T2).unwrap_err();
    assert_eq!(error.as_code(), "root_missing");
    assert_eq!(library.dump(), before);
    let statuses = library.strings("SELECT status FROM indexing_runs ORDER BY id");
    assert_eq!(statuses, ["completed", "failed"]);
    fs::rename(&away, &root).unwrap();
}

// ---------------------------------------------------------------- Verifier iteration 1

#[test]
fn library_name_drops_the_trailing_separator() {
    let mut library = Library::new("name");
    let root = PathBuf::from(format!(
        "{}{}",
        library.root.display(),
        std::path::MAIN_SEPARATOR
    ));
    scan_with(&mut library.connection, &root, &RealFs, T1).unwrap();
    let (name, root_path): (String, String) = library
        .connection
        .query_row("SELECT name, root_path FROM libraries", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert!(root_path.ends_with(std::path::MAIN_SEPARATOR));
    assert_eq!(name, library.root.to_string_lossy());
}

#[test]
fn size_change_with_the_same_date_is_a_modification() {
    let mut library = Library::new("size-only");
    library.write("a.jpg", b"x");
    library.scan(T1);
    let before = library.media("a.jpg");
    let path = library.root.join("a.jpg");
    let time = modified(&path);
    fs::write(&path, b"hello").unwrap();
    set_modified(&path, time);
    assert_eq!(super::time::iso_utc(modified(&path)), before.modified);

    let probe = Probe::default();
    let summary = library.scan_using(&probe, T2).unwrap();
    assert_eq!(*probe.opened.borrow(), ["a.jpg"]);
    let after = library.media("a.jpg");
    assert_eq!(after.id, before.id);
    assert_eq!(after.size, Some(5));
    assert_eq!(after.hash.as_deref(), Some(HELLO_SHA256));
    assert_eq!(summary.modified, 1);
}

#[test]
fn modified_media_loses_its_old_hash_until_it_is_read_again() {
    let mut library = Library::new("modify-vanish");
    library.write("a.jpg", b"x");
    library.scan(T1);
    let old_hash = library.media("a.jpg").hash;
    library.write("a.jpg", b"hello");
    let root = library.root.clone();
    let away = library.scratch.path.join("away");
    let (root_hook, away_hook) = (root.clone(), away.clone());
    let probe = Probe {
        on_open: Some(Box::new(move |_| {
            fs::rename(&root_hook, &away_hook).unwrap();
            true
        })),
        ..Probe::default()
    };
    assert_eq!(
        library.scan_using(&probe, T2).unwrap_err(),
        ScanError::RootMissing
    );
    let pending = library.media("a.jpg");
    assert_eq!(pending.size, Some(5));
    assert_eq!(pending.hash, None);
    assert_ne!(pending.hash, old_hash);

    fs::rename(&away, &root).unwrap();
    let probe = Probe::default();
    library.scan_using(&probe, T3).unwrap();
    assert_eq!(*probe.opened.borrow(), ["a.jpg"]);
    assert_eq!(library.media("a.jpg").hash.as_deref(), Some(HELLO_SHA256));
}

#[test]
fn hashes_are_committed_every_hundred_files() {
    let mut library = Library::new("batches");
    for index in 0..150 {
        library.write(&format!("f{index:03}.jpg"), b"x");
    }
    let db_path = library.scratch.path.join("library.db");
    let seen = std::rc::Rc::new(RefCell::new(Vec::new()));
    let seen_hook = seen.clone();
    let probe = Probe {
        on_open: Some(Box::new(move |count| {
            if [1, 50, 100, 101, 150].contains(&count) {
                let reader = Connection::open(&db_path).unwrap();
                let hashed: i64 = reader
                    .query_row(
                        "SELECT count(*) FROM media_items WHERE file_hash_sha256 IS NOT NULL",
                        [],
                        |row| row.get(0),
                    )
                    .unwrap();
                seen_hook.borrow_mut().push((count, hashed));
            }
            false
        })),
        ..Probe::default()
    };
    library.scan_using(&probe, T1).unwrap();
    // Seen by another connection just before file N is read: nothing until the first 100
    // are committed, then exactly 100 until the end.
    assert_eq!(
        *seen.borrow(),
        [(1, 0), (50, 0), (100, 0), (101, 100), (150, 100)]
    );
    let hashed: i64 = library
        .connection
        .query_row(
            "SELECT count(*) FROM media_items WHERE file_hash_sha256 IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(hashed, 150);
}

#[test]
fn folder_case_rename_updates_the_path_and_keeps_the_id() {
    let mut library = Library::new("folder-case");
    fs::create_dir_all(library.root.join("Viagem")).unwrap();
    library.scan(T1);
    let id: i64 = library
        .connection
        .query_row("SELECT id FROM folders", [], |r| r.get(0))
        .unwrap();
    fs::rename(library.root.join("Viagem"), library.root.join("viagem")).unwrap();
    library.scan(T2);
    let (after_id, path): (i64, String) = library
        .connection
        .query_row("SELECT id, relative_path FROM folders", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!((after_id, path.as_str()), (id, "viagem"));
}

#[test]
fn protection_stops_at_the_folder_boundary() {
    let mut library = Library::new("sibling");
    library.write("trancada/a.jpg", b"a");
    library.write("trancada2/b.jpg", b"b");
    fs::create_dir_all(library.root.join("trancada2/sub")).unwrap();
    library.scan(T1);
    fs::remove_file(library.root.join("trancada2/b.jpg")).unwrap();
    fs::remove_dir_all(library.root.join("trancada2/sub")).unwrap();
    let probe = Probe {
        fail_list: vec!["trancada"],
        ..Probe::default()
    };
    let summary = library.scan_using(&probe, T2).unwrap();
    assert_eq!(library.media("trancada/a.jpg").status, "ok");
    assert_eq!(library.media("trancada2/b.jpg").status, "missing_file");
    assert_eq!(summary.missing, 1);
    assert_eq!(
        library.strings("SELECT relative_path FROM folders ORDER BY relative_path"),
        ["trancada", "trancada2"]
    );
}

#[cfg(target_os = "linux")]
#[test]
fn names_that_differ_only_in_case_keep_the_first_and_report_the_rest() {
    let mut library = Library::new("case-twins");
    library.write("A.jpg", b"a");
    library.write("a.jpg", b"b");
    library.write("Pasta/x.jpg", b"x");
    library.write("pasta/x.jpg", b"y");
    library.write("pasta/only.jpg", b"o");
    fs::create_dir_all(library.root.join("pasta/sub")).unwrap();
    let summary = library.scan(T1);
    assert_eq!(library.run(summary.run_id).1, "completed");
    assert_eq!(
        library.strings("SELECT relative_path FROM media_items ORDER BY relative_path"),
        ["A.jpg", "Pasta/x.jpg"]
    );
    assert_eq!(
        library.strings("SELECT relative_path FROM folders"),
        ["Pasta"]
    );
    assert_eq!(
        library.errors(summary.run_id),
        ["invalid_name a.jpg", "invalid_name pasta"]
    );
    assert_eq!((summary.media, summary.folders, summary.hashed), (2, 1, 2));
    // A second run is not blocked either.
    let second = library.scan(T2);
    assert_eq!(library.run(second.run_id).1, "completed");
    assert_eq!(second.unchanged, 2);
}
