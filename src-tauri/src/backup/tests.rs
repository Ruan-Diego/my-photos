use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use rusqlite::{params, Connection};

use super::*;
use crate::db::MIGRATIONS;

struct Scratch(PathBuf);

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A database on disk at `<temp>/.myphotos/library.db`, like the real one.
fn library(name: &str) -> (Scratch, Connection) {
    let root = std::env::temp_dir().join(format!("myphotos-backup-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".myphotos")).unwrap();
    let connection = Connection::open(root.join(".myphotos").join("library.db")).unwrap();
    for migration in MIGRATIONS {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
        .execute("INSERT INTO libraries (name, root_path, created_at) VALUES ('E', ?1, 't')", [root.to_string_lossy()])
        .unwrap();
    (Scratch(root), connection)
}

fn media(connection: &Connection, path: &str, sha: &str, json: Option<&str>) -> i64 {
    connection
        .execute(
            "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, file_hash_sha256, created_at, updated_at)
             VALUES (1, ?1, ?1, 'jpg', 'photo', ?2, 't', 't')",
            params![path, sha],
        )
        .unwrap();
    let id = connection.last_insert_rowid();
    connection
        .execute("INSERT INTO media_metadata_raw (media_id, takeout_json) VALUES (?1, ?2)", params![id, json])
        .unwrap();
    id
}

fn favorite(connection: &Connection, id: i64) -> bool {
    connection.query_row("SELECT is_favorite FROM media_items WHERE id = ?1", [id], |row| row.get(0)).unwrap()
}

#[test]
fn bak01_backup_copies_the_database_and_keeps_the_newest_seven() {
    let (_scratch, connection) = library("bak01");
    media(&connection, "a.jpg", "aa", None);
    let folder = folder(&connection).unwrap();
    assert!(folder.ends_with(".myphotos/backups") || folder.ends_with(".myphotos\\backups"));
    for day in 10..20 {
        backup_now(&connection, &folder, &format!("2026-10-{day}T12:00:00.000Z")).unwrap();
    }
    let files = list(&folder);
    assert_eq!(files.len(), KEEP);
    assert_eq!(files[0].name, "library-20261019120000.db");
    assert_eq!(files[KEEP - 1].name, "library-20261013120000.db");
    let copy = Connection::open(folder.join(&files[0].name)).unwrap();
    let count: i64 = copy.query_row("SELECT count(*) FROM media_items", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 1, "the copy is a working database");
}

#[test]
fn bak02_automatic_backup_runs_once_a_day() {
    let (_scratch, connection) = library("bak02");
    let folder = folder(&connection).unwrap();
    let now = SystemTime::now();
    assert!(auto_backup(&connection, &folder, now, "2026-10-28T08:00:00Z").unwrap().is_some());
    assert!(auto_backup(&connection, &folder, now + Duration::from_secs(3600), "2026-10-28T09:00:00Z").unwrap().is_none());
    assert!(auto_backup(&connection, &folder, now + EVERY + Duration::from_secs(1), "2026-10-29T08:00:01Z").unwrap().is_some());
    assert_eq!(list(&folder).len(), 2);
}

#[test]
fn bak03_export_then_import_into_a_new_index_restores_favorites_and_notes() {
    let (_old, old) = library("bak03-old");
    let starred = media(&old, "Takeout/a.jpg", "aa", None);
    let unstarred = media(&old, "Takeout/b.jpg", "bb", Some(r#"{"favorited":true}"#));
    let noted = media(&old, "Takeout/c.jpg", "cc", None);
    media(&old, "Takeout/gone.jpg", "dd", None);
    old.execute("UPDATE media_items SET is_favorite = 1 WHERE id IN (?1, ?2)", [starred, 4]).unwrap();
    old.execute("INSERT INTO user_notes (media_id, note, updated_at) VALUES (?1, 'Vó no sítio', 't')", [noted]).unwrap();
    let folder = folder(&old).unwrap();
    let file = write_export(&old, &folder, "2026-10-28T10:00:00Z").unwrap();
    let name = file.file_name().unwrap().to_string_lossy().into_owned();
    assert_eq!(name, "dados-20261028100000.json");
    assert_eq!(list(&folder)[0].kind, "export");
    let data = read_export(&folder, &name).unwrap();
    assert_eq!(data.favorites.len(), 2);
    assert_eq!(data.unfavorites, vec![MediaRef { path: "Takeout/b.jpg".into(), sha256: Some("bb".into()) }]);
    let _ = unstarred;

    // New index: "a" moved to another folder (same bytes), "b" came back as a Takeout
    // favorite, "gone" is not on the disk any more.
    let (_new, mut new) = library("bak03-new");
    let moved = media(&new, "Ruan/a.jpg", "aa", None);
    let again = media(&new, "Takeout/b.jpg", "bb", Some(r#"{"favorited":true}"#));
    new.execute("UPDATE media_items SET is_favorite = 1 WHERE id = ?1", [again]).unwrap();
    let noted_again = media(&new, "Takeout/c.jpg", "cc", None);
    let outcome = import(&mut new, &data, "2026-10-28T11:00:00Z").unwrap();
    assert_eq!(outcome, ImportOutcome { favorites: 1, unfavorites: 1, notes: 1, missing: 1 });
    assert!(favorite(&new, moved));
    assert!(!favorite(&new, again));
    let note: String = new.query_row("SELECT note FROM user_notes WHERE media_id = ?1", [noted_again], |row| row.get(0)).unwrap();
    assert_eq!(note, "Vó no sítio");
    let found: i64 = new
        .query_row("SELECT count(*) FROM media_search WHERE media_search MATCH 'sítio'", [], |row| row.get(0))
        .unwrap();
    assert_eq!(found, 1, "the note is searchable");
}

#[test]
fn bak04_import_reads_only_exports_in_the_backups_folder() {
    let (_scratch, connection) = library("bak04");
    let folder = folder(&connection).unwrap();
    std::fs::create_dir_all(&folder).unwrap();
    for name in ["../library.db", "dados-1.json/../../x.json", "library-1.db", "dados-..\\x.json", "outro.json"] {
        assert!(read_export(&folder, name).is_err(), "{name}");
    }
    std::fs::write(folder.join("dados-1.json"), "{não é json").unwrap();
    assert!(read_export(&folder, "dados-1.json").unwrap_err().contains("não é uma exportação"));
    std::fs::write(folder.join("dados-2.json"), r#"{"app":"outro","version":1,"exportedAt":"","favorites":[],"unfavorites":[],"notes":[]}"#).unwrap();
    assert!(read_export(&folder, "dados-2.json").is_err());
    std::fs::write(folder.join("dados-3.json"), r#"{"app":"my-photos","version":1,"exportedAt":"","favorites":[],"unfavorites":[],"notes":[]}"#).unwrap();
    assert!(read_export(&folder, "dados-3.json").is_ok());
}
