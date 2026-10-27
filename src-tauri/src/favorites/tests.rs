use rusqlite::{params, Connection};

use super::{import_takeout_favorites, set_favorite};
use crate::db::MIGRATIONS;

fn database() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    for migration in MIGRATIONS {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
        .execute("INSERT INTO libraries (name, root_path, created_at) VALUES ('E', 'E:\\', 't')", [])
        .unwrap();
    connection
}

fn media(connection: &Connection, path: &str, json: Option<&str>) -> i64 {
    connection
        .execute(
            "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, created_at, updated_at)
             VALUES (1, ?1, ?1, 'jpg', 'photo', 't', 't')",
            [path],
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
fn fav01_set_favorite_marks_and_unmarks_only_in_the_database() {
    let connection = database();
    let id = media(&connection, "a.jpg", None);
    assert!(set_favorite(&connection, id, true, "2026-10-27T12:00:00Z").unwrap());
    assert!(favorite(&connection, id));
    let updated: String = connection.query_row("SELECT updated_at FROM media_items WHERE id = ?1", [id], |row| row.get(0)).unwrap();
    assert_eq!(updated, "2026-10-27T12:00:00Z");
    assert!(set_favorite(&connection, id, false, "t").unwrap());
    assert!(!favorite(&connection, id));
    assert!(!set_favorite(&connection, 999, true, "t").unwrap(), "no such item");
}

#[test]
fn fav02_takeout_favorites_come_in_once_per_item() {
    let mut connection = database();
    let starred = media(&connection, "a.jpg", Some(r#"{"title":"a.jpg","favorited":true}"#));
    let plain = media(&connection, "b.jpg", Some(r#"{"title":"b.jpg","favorited":false}"#));
    let bare = media(&connection, "c.jpg", Some(r#"{"title":"c.jpg"}"#));
    let broken = media(&connection, "d.jpg", Some("{não é json"));
    let none = media(&connection, "e.jpg", None);
    assert_eq!(import_takeout_favorites(&mut connection).unwrap(), 1);
    assert_eq!(
        [starred, plain, bare, broken, none].map(|id| favorite(&connection, id)),
        [true, false, false, false, false]
    );

    // Unmarked in the app: a later run does not bring it back.
    set_favorite(&connection, starred, false, "t").unwrap();
    assert_eq!(import_takeout_favorites(&mut connection).unwrap(), 0);
    assert!(!favorite(&connection, starred));

    // A newly indexed favorite does come in.
    let newer = media(&connection, "f.jpg", Some(r#"{"favorited":true}"#));
    assert_eq!(import_takeout_favorites(&mut connection).unwrap(), 1);
    assert!(favorite(&connection, newer));
    assert!(!favorite(&connection, starred));
}
