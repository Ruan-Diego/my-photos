use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, UNIX_EPOCH};

use rusqlite::{params, Connection};

use super::{
    absolute, format_utc, hash_file, iso_utc, scan, scan_with, RunType, ScanError, ScanSummary,
};
use crate::db::open_in_memory;
use crate::db::tests::Scratch;

const ABC_SHA256: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
const SIDECAR: &[u8] = br#"{ "photoTakenTime": { "timestamp": "1660142220" } }"#;

/// A scratch dir with an empty `library` folder inside; never E:\.
fn library(label: &str) -> (Scratch, PathBuf) {
    let scratch = Scratch::new(&format!("f05-{label}"));
    let root = scratch.path.join("library");
    fs::create_dir(&root).unwrap();
    (scratch, root)
}

fn put(root: &Path, relative: &str, bytes: &[u8]) {
    let path = absolute(root, relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, bytes).unwrap();
}

fn set_modified(root: &Path, relative: &str, unix: u64) {
    File::options()
        .write(true)
        .open(absolute(root, relative))
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs(unix))
        .unwrap();
}

/// Scans with a hasher that counts its calls, so a test sees what was read again.
fn scan_counting(connection: &mut Connection, root: &Path) -> (ScanSummary, usize) {
    let mut calls = 0;
    let summary = scan_with(connection, root, 500, &mut |path| {
        calls += 1;
        hash_file(path)
    })
    .expect("scan");
    (summary, calls)
}

fn paths(connection: &Connection) -> Vec<String> {
    strings(
        connection,
        "SELECT relative_path FROM media_items ORDER BY relative_path",
    )
}

fn folders(connection: &Connection) -> Vec<String> {
    strings(
        connection,
        "SELECT relative_path FROM folders ORDER BY relative_path",
    )
}

fn strings(connection: &Connection, sql: &str) -> Vec<String> {
    let mut statement = connection.prepare(sql).unwrap();
    let rows = statement.query_map([], |row| row.get(0)).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn count(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

fn id_of(connection: &Connection, path: &str) -> i64 {
    connection
        .query_row(
            "SELECT id FROM media_items WHERE relative_path = ?1 COLLATE BINARY",
            [path],
            |row| row.get(0),
        )
        .unwrap()
}

fn field<T: rusqlite::types::FromSql>(connection: &Connection, path: &str, column: &str) -> T {
    connection
        .query_row(
            &format!("SELECT {column} FROM media_items WHERE relative_path = ?1 COLLATE BINARY"),
            [path],
            |row| row.get(0),
        )
        .unwrap()
}

fn errors(connection: &Connection) -> Vec<(String, String)> {
    let mut statement = connection
        .prepare("SELECT relative_path, error_type FROM indexing_errors ORDER BY id")
        .unwrap();
    let rows = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn last_run(connection: &Connection) -> (String, String, i64, i64, Option<String>) {
    connection
        .query_row(
            "SELECT run_type, status, files_scanned, files_errored, finished_at
             FROM indexing_runs ORDER BY id DESC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            },
        )
        .unwrap()
}

/// Every column of every media row, in id order.
fn snapshot(connection: &Connection) -> Vec<Vec<String>> {
    let mut statement = connection
        .prepare("SELECT * FROM media_items ORDER BY id")
        .unwrap();
    let width = statement.column_count();
    let rows = statement
        .query_map([], |row| {
            (0..width)
                .map(|index| row.get_ref(index).map(|value| format!("{value:?}")))
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(directory: &Path, relative: &str, found: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().into_string().unwrap();
            let relative = format!("{relative}/{name}");
            if entry.file_type().unwrap().is_dir() {
                found.insert(format!("{relative}/"), Vec::new());
                walk(&entry.path(), &relative, found);
            } else {
                found.insert(relative, fs::read(entry.path()).unwrap());
            }
        }
    }
    let mut found = BTreeMap::new();
    walk(root, "", &mut found);
    found
}

// SCAN-01

#[test]
fn first_scan_records_media_and_folders_and_ignores_the_rest() {
    let (_scratch, root) = library("first");
    put(&root, "a/foto.jpg", b"foto");
    put(&root, "a/b/clip.mp4", b"clip");
    fs::create_dir(root.join("vazia")).unwrap();
    put(&root, "notas.txt", b"notas");
    let mut connection = open_in_memory();

    scan(&mut connection, &root).expect("scan");

    assert_eq!(paths(&connection), vec!["a/b/clip.mp4", "a/foto.jpg"]);
    assert_eq!(
        field::<String>(&connection, "a/b/clip.mp4", "media_type"),
        "video"
    );
    assert_eq!(
        field::<String>(&connection, "a/foto.jpg", "media_type"),
        "photo"
    );
    assert_eq!(folders(&connection), vec!["a", "a/b", "vazia"]);
}

#[test]
fn app_folder_at_the_root_is_skipped_in_any_case() {
    for name in [".myphotos", ".MyPhotos"] {
        let (_scratch, root) = library("app-folder");
        put(&root, &format!("{name}/trash/x.jpg"), b"x");
        put(&root, &format!("{name}/library.db"), b"db");
        put(&root, "a/.myphotos/y.jpg", b"y");
        let mut connection = open_in_memory();

        scan(&mut connection, &root).expect("scan");

        assert_eq!(paths(&connection), vec!["a/.myphotos/y.jpg"], "{name}");
        assert_eq!(folders(&connection), vec!["a", "a/.myphotos"], "{name}");
    }
}

#[cfg(windows)]
fn link_dir(target: &Path, link: &Path) {
    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .expect("mklink");
    assert!(output.status.success(), "mklink /J");
}

#[cfg(not(windows))]
fn link_dir(target: &Path, link: &Path) {
    std::os::unix::fs::symlink(target, link).unwrap();
}

#[test]
fn linked_folder_is_not_followed() {
    let (scratch, root) = library("link");
    put(&scratch.path, "outside/secret.jpg", b"secret");
    link_dir(&scratch.path.join("outside"), &root.join("link"));
    put(&root, "dentro.jpg", b"dentro");
    let mut connection = open_in_memory();

    scan(&mut connection, &root).expect("scan");

    assert_eq!(paths(&connection), vec!["dentro.jpg"]);
    assert!(folders(&connection).is_empty());
}

#[cfg(windows)]
#[test]
fn hidden_system_folder_is_skipped_below_the_root() {
    let (_scratch, root) = library("system");
    put(&root, "a/sys/x.jpg", b"x");
    let attrib = |flags: &[&str]| {
        let output = std::process::Command::new("attrib")
            .args(flags)
            .arg(root.join("a").join("sys"))
            .output()
            .expect("attrib");
        assert!(output.status.success(), "attrib {flags:?}");
    };
    attrib(&["+h", "+s"]);
    let mut connection = open_in_memory();

    let result = scan(&mut connection, &root);
    attrib(&["-h", "-s"]);

    result.expect("scan");
    assert!(paths(&connection).is_empty());
    assert_eq!(folders(&connection), vec!["a"]);
}

// SCAN-02

#[test]
fn media_row_has_the_file_fields_and_the_sha256() {
    let (_scratch, root) = library("fields");
    put(&root, "Fotos/IMG_1.JPG", b"abc");
    set_modified(&root, "Fotos/IMG_1.JPG", 1_660_142_220);
    let mut connection = open_in_memory();

    scan(&mut connection, &root).expect("scan");

    let row: (
        String,
        String,
        i64,
        String,
        Option<String>,
        String,
        Option<String>,
    ) = connection
        .query_row(
            "SELECT filename, extension, size_bytes, fs_modified_at, file_hash_sha256,
               indexing_status, taken_at
             FROM media_items",
            [],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(
        row,
        (
            "IMG_1.JPG".to_string(),
            "jpg".to_string(),
            3,
            "2022-08-10T14:37:00Z".to_string(),
            Some(ABC_SHA256.to_string()),
            "ok".to_string(),
            None,
        )
    );
}

#[test]
fn takeout_json_match_is_recorded() {
    let (_scratch, root) = library("json");
    put(&root, "foto.jpg", b"foto");
    put(&root, "foto.jpg.json", SIDECAR);
    put(&root, "outra.jpg", b"outra");
    let mut connection = open_in_memory();

    scan(&mut connection, &root).expect("scan");

    let matched = |path: &str| -> (i64, Option<String>, String) {
        (
            field(&connection, path, "has_takeout_json"),
            field(&connection, path, "json_relative_path"),
            field(&connection, path, "json_match_confidence"),
        )
    };
    assert_eq!(
        matched("foto.jpg"),
        (1, Some("foto.jpg.json".into()), "exact".into())
    );
    assert_eq!(matched("outra.jpg"), (0, None, "none".into()));
}

#[test]
fn first_scan_is_a_completed_full_run_of_one_library() {
    let (_scratch, root) = library("run");
    put(&root, "a.jpg", b"a");
    put(&root, "b.mp4", b"b");
    let mut connection = open_in_memory();

    let summary = scan(&mut connection, &root).expect("scan");

    assert_eq!(summary.run_type, RunType::Full);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM libraries"), 1);
    let (root_path, status): (String, String) = connection
        .query_row("SELECT root_path, status FROM libraries", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(root_path, root.to_string_lossy());
    assert_eq!(status, "active");
    let (run_type, status, scanned, errored, finished_at) = last_run(&connection);
    assert_eq!(
        (run_type.as_str(), status.as_str(), scanned, errored),
        ("full", "completed", 2, 0)
    );
    let last_indexed_at: Option<String> = connection
        .query_row("SELECT last_indexed_at FROM libraries", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(finished_at.is_some());
    assert_eq!(last_indexed_at, finished_at);
}

#[test]
fn scan_leaves_every_library_byte_as_it_was() {
    let (_scratch, root) = library("bytes");
    put(&root, "a/foto.jpg", b"foto");
    put(&root, "a/foto.jpg.json", SIDECAR);
    put(&root, "a/b/clip.mp4", b"clip");
    put(&root, "notas.txt", b"notas");
    put(&root, ".myphotos/trash/x.jpg", b"x");
    let before = tree(&root);
    let mut connection = open_in_memory();

    scan(&mut connection, &root).expect("scan");
    scan(&mut connection, &root).expect("scan");

    assert_eq!(tree(&root), before);
}

// SCAN-03

#[test]
fn second_scan_of_an_unchanged_disk_rehashes_nothing() {
    let (_scratch, root) = library("unchanged");
    put(&root, "a/1.jpg", b"one");
    put(&root, "a/2.mp4", b"two");
    put(&root, "a/2.mp4.json", SIDECAR);
    let mut connection = open_in_memory();
    let (_, first_calls) = scan_counting(&mut connection, &root);
    let before = snapshot(&connection);

    let (second, second_calls) = scan_counting(&mut connection, &root);

    assert_eq!(first_calls, 2);
    assert_eq!(second_calls, 0);
    assert_eq!(snapshot(&connection), before);
    assert_eq!(second.run_type, RunType::Incremental);
    assert_eq!(
        (second.hashed, second.files_indexed, second.files_scanned),
        (0, 0, 2)
    );
    assert_eq!(last_run(&connection).0, "incremental");
}

#[test]
fn new_file_is_inserted_and_known_ids_stay() {
    let (_scratch, root) = library("new");
    put(&root, "a.jpg", b"a");
    put(&root, "c.jpg", b"c");
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    let (a, c) = (id_of(&connection, "a.jpg"), id_of(&connection, "c.jpg"));

    put(&root, "b.jpg", b"b");
    let (summary, calls) = scan_counting(&mut connection, &root);

    assert_eq!(calls, 1);
    assert_eq!(summary.files_indexed, 1);
    assert_eq!(paths(&connection), vec!["a.jpg", "b.jpg", "c.jpg"]);
    assert_eq!(
        (id_of(&connection, "a.jpg"), id_of(&connection, "c.jpg")),
        (a, c)
    );
}

#[test]
fn changed_content_is_rehashed_under_the_same_id() {
    let (_scratch, root) = library("changed");
    put(&root, "foto.jpg", b"abc");
    set_modified(&root, "foto.jpg", 1_600_000_000);
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    let id = id_of(&connection, "foto.jpg");

    // Size changes.
    put(&root, "foto.jpg", b"abcd");
    set_modified(&root, "foto.jpg", 1_600_000_000);
    let (_, calls) = scan_counting(&mut connection, &root);
    assert_eq!(calls, 1);
    assert_eq!(id_of(&connection, "foto.jpg"), id);
    assert_eq!(field::<i64>(&connection, "foto.jpg", "size_bytes"), 4);
    let after_size: String = field(&connection, "foto.jpg", "file_hash_sha256");
    assert_eq!(after_size, hash_file(&root.join("foto.jpg")).unwrap());

    // Same size, only the modification time changes.
    put(&root, "foto.jpg", b"wxyz");
    set_modified(&root, "foto.jpg", 1_700_000_000);
    let (_, calls) = scan_counting(&mut connection, &root);
    assert_eq!(calls, 1);
    assert_eq!(id_of(&connection, "foto.jpg"), id);
    assert_eq!(
        field::<String>(&connection, "foto.jpg", "fs_modified_at"),
        "2023-11-14T22:13:20Z"
    );
    assert_eq!(
        field::<String>(&connection, "foto.jpg", "file_hash_sha256"),
        hash_file(&root.join("foto.jpg")).unwrap()
    );
}

#[test]
fn json_change_updates_the_match_without_rehash() {
    let (_scratch, root) = library("json-change");
    put(&root, "foto.jpg", b"foto");
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    assert_eq!(
        field::<String>(&connection, "foto.jpg", "json_match_confidence"),
        "none"
    );

    put(&root, "foto.jpg.json", SIDECAR);
    let (summary, calls) = scan_counting(&mut connection, &root);
    assert_eq!(calls, 0);
    assert_eq!(summary.files_indexed, 1);
    assert_eq!(
        field::<String>(&connection, "foto.jpg", "json_match_confidence"),
        "exact"
    );
    assert_eq!(field::<i64>(&connection, "foto.jpg", "has_takeout_json"), 1);

    fs::remove_file(root.join("foto.jpg.json")).unwrap();
    let (_, calls) = scan_counting(&mut connection, &root);
    assert_eq!(calls, 0);
    assert_eq!(
        field::<String>(&connection, "foto.jpg", "json_match_confidence"),
        "none"
    );
    assert_eq!(field::<i64>(&connection, "foto.jpg", "has_takeout_json"), 0);
    assert_eq!(
        field::<Option<String>>(&connection, "foto.jpg", "json_relative_path"),
        None
    );
}

// SCAN-04

#[test]
fn removed_file_is_missing_with_its_favorite_and_note_and_comes_back() {
    let (scratch, root) = library("removed");
    put(&root, "foto.jpg", b"abc");
    put(&root, "outra.jpg", b"outra");
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    let id = id_of(&connection, "foto.jpg");
    connection
        .execute("UPDATE media_items SET is_favorite = 1 WHERE id = ?1", [id])
        .unwrap();
    connection
        .execute(
            "INSERT INTO user_notes (media_id, note) VALUES (?1, 'praia')",
            [id],
        )
        .unwrap();

    let aside = scratch.path.join("aside.jpg");
    fs::rename(root.join("foto.jpg"), &aside).unwrap();
    let summary = scan(&mut connection, &root).expect("scan");

    assert_eq!(summary.missing, 1);
    assert_eq!(id_of(&connection, "foto.jpg"), id);
    assert_eq!(
        field::<String>(&connection, "foto.jpg", "indexing_status"),
        "missing_file"
    );
    assert_eq!(field::<i64>(&connection, "foto.jpg", "is_favorite"), 1);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM user_notes"), 1);
    assert_eq!(
        field::<String>(&connection, "outra.jpg", "indexing_status"),
        "ok"
    );

    fs::rename(&aside, root.join("foto.jpg")).unwrap();
    scan(&mut connection, &root).expect("scan");

    assert_eq!(id_of(&connection, "foto.jpg"), id);
    assert_eq!(
        field::<String>(&connection, "foto.jpg", "indexing_status"),
        "ok"
    );
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM media_items"), 2);
}

#[test]
fn file_moved_outside_the_app_keeps_its_id() {
    let (_scratch, root) = library("moved");
    put(&root, "a/foto.jpg", b"abc");
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    let id = id_of(&connection, "a/foto.jpg");
    connection
        .execute("UPDATE media_items SET is_favorite = 1 WHERE id = ?1", [id])
        .unwrap();

    fs::create_dir(root.join("b")).unwrap();
    fs::rename(
        root.join("a").join("foto.jpg"),
        root.join("b").join("foto2.jpg"),
    )
    .unwrap();
    let summary = scan(&mut connection, &root).expect("scan");

    assert_eq!((summary.moved, summary.missing), (1, 0));
    assert_eq!(paths(&connection), vec!["b/foto2.jpg"]);
    assert_eq!(id_of(&connection, "b/foto2.jpg"), id);
    assert_eq!(
        field::<String>(&connection, "b/foto2.jpg", "filename"),
        "foto2.jpg"
    );
    assert_eq!(
        field::<String>(&connection, "b/foto2.jpg", "indexing_status"),
        "ok"
    );
    assert_eq!(field::<i64>(&connection, "b/foto2.jpg", "is_favorite"), 1);
}

#[test]
fn two_missing_twins_do_not_adopt_a_new_copy() {
    let (_scratch, root) = library("twins");
    put(&root, "a/1.jpg", b"abc");
    put(&root, "a/2.jpg", b"abc");
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    let twins = [id_of(&connection, "a/1.jpg"), id_of(&connection, "a/2.jpg")];
    fs::remove_dir_all(root.join("a")).unwrap();
    scan(&mut connection, &root).expect("scan");

    put(&root, "b/3.jpg", b"abc");
    let summary = scan(&mut connection, &root).expect("scan");

    assert_eq!(summary.moved, 0);
    assert!(!twins.contains(&id_of(&connection, "b/3.jpg")));
    assert_eq!(
        field::<String>(&connection, "a/1.jpg", "indexing_status"),
        "missing_file"
    );
    assert_eq!(
        field::<String>(&connection, "a/2.jpg", "indexing_status"),
        "missing_file"
    );
    assert_eq!(
        field::<String>(&connection, "b/3.jpg", "indexing_status"),
        "ok"
    );
}

#[cfg(windows)]
#[test]
fn unreadable_folder_keeps_the_status_of_its_media() {
    let (_scratch, root) = library("locked");
    put(&root, "locked/foto.jpg", b"foto");
    put(&root, "outra.jpg", b"outra");
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    let locked = root.join("locked");
    // Same ACL trick as the F02 test: drop every inherited ACE, then /reset restores it.
    let acl = |args: &[&str]| {
        let output = std::process::Command::new("icacls")
            .arg(&locked)
            .args(args)
            .output()
            .expect("icacls");
        assert!(output.status.success(), "icacls {args:?}");
    };
    acl(&["/inheritance:r"]);

    let result = scan(&mut connection, &root);
    acl(&["/reset"]);

    let summary = result.expect("scan");
    assert_eq!(summary.missing, 0);
    assert_eq!(
        field::<String>(&connection, "locked/foto.jpg", "indexing_status"),
        "ok"
    );
    assert_eq!(folders(&connection), vec!["locked"]);
    assert!(errors(&connection).contains(&("locked".into(), "unreadable_directory".into())));
}

#[test]
fn root_lost_mid_scan_fails_the_run_and_marks_nothing_missing() {
    let (_scratch, root) = library("lost");
    put(&root, "a/1.jpg", b"1");
    put(&root, "a/2.jpg", b"2");
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    for index in 3..=7 {
        put(
            &root,
            &format!("n/{index}.jpg"),
            format!("{index}").as_bytes(),
        );
    }

    // Batches of 2: [a/1, a/2] and [n/3, n/4] commit; the disk goes at n/5.
    let mut calls = 0;
    let unplug = root.clone();
    let result = scan_with(&mut connection, &root, 2, &mut |path| {
        calls += 1;
        if calls == 3 {
            fs::remove_dir_all(&unplug).unwrap();
            return Err(io::Error::other("unplugged"));
        }
        hash_file(path)
    });

    assert_eq!(result.unwrap_err(), ScanError::RootLost);
    assert_eq!(last_run(&connection).1, "failed");
    assert!(last_run(&connection).4.is_some());
    assert_eq!(
        paths(&connection),
        vec!["a/1.jpg", "a/2.jpg", "n/3.jpg", "n/4.jpg"]
    );
    assert_eq!(
        count(
            &connection,
            "SELECT COUNT(*) FROM media_items WHERE indexing_status <> 'ok'"
        ),
        0
    );
    assert_eq!(folders(&connection), vec!["a"]);
}

#[test]
fn vanished_folder_leaves_the_folder_table() {
    let (_scratch, root) = library("vanished");
    put(&root, "a/x.jpg", b"x");
    put(&root, "b/c/y.jpg", b"y");
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    assert_eq!(folders(&connection), vec!["a", "b", "b/c"]);

    fs::remove_dir_all(root.join("b")).unwrap();
    scan(&mut connection, &root).expect("scan");

    assert_eq!(folders(&connection), vec!["a"]);
}

// SCAN-05, SCAN-06

#[test]
fn unreadable_file_is_an_error_row_and_its_neighbors_are_written() {
    let (_scratch, root) = library("unreadable");
    put(&root, "bad.jpg", b"bad");
    put(&root, "good.jpg", b"abc");
    let mut connection = open_in_memory();

    let summary = scan_with(&mut connection, &root, 500, &mut |path| {
        if path.ends_with("bad.jpg") {
            Err(io::Error::from(io::ErrorKind::PermissionDenied))
        } else {
            hash_file(path)
        }
    })
    .expect("scan");

    assert_eq!(summary.files_errored, 1);
    assert_eq!(
        field::<String>(&connection, "bad.jpg", "indexing_status"),
        "error"
    );
    assert_eq!(
        field::<Option<String>>(&connection, "bad.jpg", "file_hash_sha256"),
        None
    );
    assert!(field::<Option<String>>(&connection, "bad.jpg", "indexing_error").is_some());
    assert_eq!(
        field::<String>(&connection, "good.jpg", "indexing_status"),
        "ok"
    );
    assert_eq!(
        field::<String>(&connection, "good.jpg", "file_hash_sha256"),
        ABC_SHA256
    );
    assert_eq!(
        errors(&connection),
        vec![("bad.jpg".into(), "unreadable".into())]
    );
    let (_, status, scanned, errored, _) = last_run(&connection);
    assert_eq!((status.as_str(), scanned, errored), ("completed", 2, 1));

    // Readable again: the next scan hashes it and clears the error.
    let (_, calls) = scan_counting(&mut connection, &root);
    assert_eq!(calls, 1);
    assert_eq!(
        field::<String>(&connection, "bad.jpg", "indexing_status"),
        "ok"
    );
    assert_eq!(
        field::<Option<String>>(&connection, "bad.jpg", "indexing_error"),
        None
    );
}

#[test]
fn invalid_json_leaves_the_media_unmatched_with_an_error() {
    let (_scratch, root) = library("invalid-json");
    put(&root, "foto.jpg", b"foto");
    put(&root, "foto.jpg.json", b"{ nope");
    let mut connection = open_in_memory();

    scan(&mut connection, &root).expect("scan");

    assert_eq!(
        field::<String>(&connection, "foto.jpg", "json_match_confidence"),
        "none"
    );
    assert_eq!(field::<i64>(&connection, "foto.jpg", "has_takeout_json"), 0);
    assert_eq!(
        errors(&connection),
        vec![("foto.jpg.json".into(), "invalid_json".into())]
    );
}

#[cfg(windows)]
fn invalid_name() -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt;
    // An unpaired surrogate: legal on NTFS, not valid Unicode.
    let mut wide = vec![0xD800];
    wide.extend("a.jpg".encode_utf16());
    std::ffi::OsString::from_wide(&wide)
}

#[cfg(not(windows))]
fn invalid_name() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(b"\xffa.jpg".to_vec())
}

#[test]
fn invalid_name_is_an_error_outside_media_items() {
    let (_scratch, root) = library("invalid-name");
    fs::write(root.join(invalid_name()), b"x").unwrap();
    put(&root, "boa.jpg", b"boa");
    let mut connection = open_in_memory();

    scan(&mut connection, &root).expect("scan");

    assert_eq!(paths(&connection), vec!["boa.jpg"]);
    let found = errors(&connection);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1, "invalid_name");
    assert!(found[0].0.ends_with("a.jpg"), "{found:?}");
}

#[test]
fn run_left_running_by_a_previous_process_becomes_failed() {
    let (_scratch, root) = library("stale");
    let mut connection = open_in_memory();
    connection
        .execute(
            "INSERT INTO libraries (name, root_path, created_at) VALUES ('x', ?1, '2026-10-01T00:00:00Z')",
            [root.to_string_lossy()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO indexing_runs (library_id, started_at, run_type)
             VALUES (1, '2026-10-01T00:00:00Z', 'full')",
            [],
        )
        .unwrap();

    let summary = scan(&mut connection, &root).expect("scan");

    let (status, finished_at): (String, Option<String>) = connection
        .query_row(
            "SELECT status, finished_at FROM indexing_runs WHERE id = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "failed");
    assert!(finished_at.is_some());
    assert_ne!(summary.run_id, 1);
    assert_eq!(summary.run_type, RunType::Full);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM libraries"), 1);
}

#[test]
fn missing_root_is_refused_without_writing() {
    let (scratch, _root) = library("missing-root");
    let mut connection = open_in_memory();

    let error = scan(&mut connection, &scratch.path.join("absent")).unwrap_err();

    assert_eq!(error, ScanError::RootMissing);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM libraries"), 0);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM indexing_runs"), 0);
    let file = scratch.path.join("arquivo.jpg");
    fs::write(&file, b"x").unwrap();
    assert_eq!(
        scan(&mut connection, &file).unwrap_err(),
        ScanError::RootMissing
    );
}

// Edge cases

#[test]
fn empty_file_has_the_sha256_of_no_bytes() {
    let (_scratch, root) = library("empty");
    put(&root, "vazio.jpg", b"");
    let mut connection = open_in_memory();

    scan(&mut connection, &root).expect("scan");

    assert_eq!(
        field::<String>(&connection, "vazio.jpg", "file_hash_sha256"),
        EMPTY_SHA256
    );
    assert_eq!(field::<i64>(&connection, "vazio.jpg", "size_bytes"), 0);
}

#[test]
fn case_only_rename_keeps_the_id() {
    let (_scratch, root) = library("case");
    put(&root, "foto.jpg", b"foto");
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    let id = id_of(&connection, "foto.jpg");

    fs::rename(root.join("foto.jpg"), root.join("Foto.JPG")).unwrap();
    let summary = scan(&mut connection, &root).expect("scan");

    assert_eq!(paths(&connection), vec!["Foto.JPG"]);
    assert_eq!(id_of(&connection, "Foto.JPG"), id);
    assert_eq!(
        field::<String>(&connection, "Foto.JPG", "filename"),
        "Foto.JPG"
    );
    assert_eq!(field::<String>(&connection, "Foto.JPG", "extension"), "jpg");
    assert_eq!((summary.missing, summary.files_indexed), (0, 1));
}

#[test]
fn folder_case_rename_updates_the_paths_and_keeps_the_id() {
    let (_scratch, root) = library("folder-case");
    put(&root, "ferias/foto.jpg", b"foto");
    let mut connection = open_in_memory();
    scan(&mut connection, &root).expect("scan");
    let id = id_of(&connection, "ferias/foto.jpg");

    fs::rename(root.join("ferias"), root.join("Ferias")).unwrap();
    scan(&mut connection, &root).expect("scan");

    assert_eq!(paths(&connection), vec!["Ferias/foto.jpg"]);
    assert_eq!(id_of(&connection, "Ferias/foto.jpg"), id);
    assert_eq!(folders(&connection), vec!["Ferias"]);
}

#[cfg(not(windows))]
#[test]
fn names_that_differ_only_in_case_keep_the_first() {
    let (_scratch, root) = library("case-twins");
    put(&root, "Foto.jpg", b"1");
    put(&root, "foto.jpg", b"2");
    let mut connection = open_in_memory();

    scan(&mut connection, &root).expect("scan");

    assert_eq!(paths(&connection), vec!["Foto.jpg"]);
    assert_eq!(
        errors(&connection),
        vec![("foto.jpg".into(), "invalid_name".into())]
    );
}

#[test]
fn dates_are_utc_seconds_with_z() {
    assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
    assert_eq!(format_utc(1_660_142_220), "2022-08-10T14:37:00Z");
    assert_eq!(format_utc(951_782_400), "2000-02-29T00:00:00Z");
    assert_eq!(format_utc(-1), "1969-12-31T23:59:59Z");
    assert_eq!(
        iso_utc(UNIX_EPOCH + Duration::from_millis(1_660_142_220_999)),
        "2022-08-10T14:37:00Z"
    );
    assert_eq!(
        iso_utc(UNIX_EPOCH - Duration::from_millis(1_500)),
        "1969-12-31T23:59:58Z"
    );
}

#[test]
fn every_error_has_a_code_and_a_message() {
    let expected = [
        (
            ScanError::RootMissing,
            "root_missing",
            "pasta da biblioteca não encontrada",
        ),
        (
            ScanError::RootLost,
            "root_lost",
            "a pasta da biblioteca sumiu durante a varredura",
        ),
        (
            ScanError::DatabaseFailed,
            "database_failed",
            "falha ao gravar no banco",
        ),
    ];
    for (error, code, message) in expected {
        assert_eq!(error.as_code(), code);
        assert_eq!(error.message(), message);
    }
}

/// SCAN-07, manual. Scans the real library twice. Run with
/// `cargo test --manifest-path src-tauri/Cargo.toml real_library_scan -- --ignored --nocapture`.
#[test]
#[ignore]
fn real_library_scan() {
    let mut connection = crate::db::open_library().expect("E:");
    for pass in 1..=2 {
        let started = Instant::now();
        let summary = scan(&mut connection, Path::new(r"E:\")).expect("scan");
        println!(
            "pass={pass} elapsed_ms={} {summary:?}",
            started.elapsed().as_millis()
        );
    }
    let grouped = |sql: &str| -> Vec<(String, i64)> {
        let mut statement = connection.prepare(sql).unwrap();
        let rows = statement
            .query_map(params![], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap();
        rows.collect::<Result<_, _>>().unwrap()
    };
    println!(
        "type={:?}",
        grouped("SELECT media_type, COUNT(*) FROM media_items GROUP BY 1")
    );
    println!(
        "json={:?}",
        grouped("SELECT json_match_confidence, COUNT(*) FROM media_items GROUP BY 1")
    );
    println!(
        "status={:?}",
        grouped("SELECT indexing_status, COUNT(*) FROM media_items GROUP BY 1")
    );
    println!(
        "errors={:?}",
        grouped(
            "SELECT error_type, COUNT(*) FROM indexing_errors
             WHERE run_id = (SELECT MAX(id) FROM indexing_runs) GROUP BY 1"
        )
    );
    println!(
        "folders={}",
        count(&connection, "SELECT COUNT(*) FROM folders")
    );
}
