use std::fs;

use rusqlite::Connection;

use super::{clear_thumbnails, flag, set_cache_limit, set_flag, settings, SCAN_ON_CONNECT, VIDEO_COVERS};
use crate::db::MIGRATIONS;
use crate::thumbs::{cache_limit, DEFAULT_CACHE_LIMIT_BYTES};

fn database() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    for migration in MIGRATIONS {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
        .execute("INSERT INTO libraries (name, root_path, volume_label, created_at) VALUES ('E', 'E:\\', 'FOTOS', 't')", [])
        .unwrap();
    connection
}

#[test]
fn set01_defaults_follow_the_canvas_except_scan_on_connect() {
    let connection = database();
    let current = settings(&connection, true).unwrap();
    assert_eq!(current.root_path.as_deref(), Some("E:\\"));
    assert_eq!(current.volume_label.as_deref(), Some("FOTOS"));
    assert!(current.video_covers);
    assert!(!current.scan_on_connect);
    assert_eq!(current.thumbnail_limit_bytes, DEFAULT_CACHE_LIMIT_BYTES);
    assert!(current.cache_limits.contains(&DEFAULT_CACHE_LIMIT_BYTES));
    assert_eq!(current.ignored_folders[0], ".myphotos");
}

#[test]
fn set02_switches_are_stored_and_unknown_keys_refused() {
    let connection = database();
    set_flag(&connection, VIDEO_COVERS, false).unwrap();
    set_flag(&connection, SCAN_ON_CONNECT, true).unwrap();
    assert!(!flag(&connection, VIDEO_COVERS, true).unwrap());
    assert!(flag(&connection, SCAN_ON_CONNECT, false).unwrap());
    set_flag(&connection, VIDEO_COVERS, true).unwrap();
    assert!(flag(&connection, VIDEO_COVERS, false).unwrap());
    assert!(set_flag(&connection, "thumbnail_cache_limit_bytes", true).is_err());
}

#[test]
fn set03_cache_limit_only_takes_offered_values() {
    let connection = database();
    set_cache_limit(&connection, 2 << 30).unwrap();
    assert_eq!(cache_limit(&connection).unwrap(), 2 << 30);
    assert!(set_cache_limit(&connection, 123).is_err());
    assert_eq!(cache_limit(&connection).unwrap(), 2 << 30);
}

#[test]
fn set04_clearing_thumbnails_empties_the_index_and_the_folder() {
    let connection = database();
    connection
        .execute(
            "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, created_at, updated_at)
             VALUES (1, 'a.jpg', 'a.jpg', 'jpg', 'photo', 't', 't')",
            [],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO thumbnails (media_id, size, width, height, bytes, source_fingerprint, created_at, last_accessed_at)
             VALUES (1, 'small', 256, 192, 5000, 'f', 't', 't')",
            [],
        )
        .unwrap();
    let root = std::env::temp_dir().join(format!("myphotos-set04-{}", std::process::id()));
    let thumb = root.join(".myphotos").join("thumbs").join("small").join("01");
    fs::create_dir_all(&thumb).unwrap();
    fs::write(thumb.join("1.jpg"), b"jpg").unwrap();
    fs::write(root.join(".myphotos").join("library.db"), b"keep").unwrap();

    assert_eq!(clear_thumbnails(&connection, &root).unwrap(), 5000);
    let left: i64 = connection.query_row("SELECT count(*) FROM thumbnails", [], |row| row.get(0)).unwrap();
    assert_eq!(left, 0);
    assert!(!root.join(".myphotos").join("thumbs").exists());
    assert!(root.join(".myphotos").join("library.db").exists());
    // Again, with nothing left: still fine.
    assert_eq!(clear_thumbnails(&connection, &root).unwrap(), 0);
    fs::remove_dir_all(&root).unwrap();
}
