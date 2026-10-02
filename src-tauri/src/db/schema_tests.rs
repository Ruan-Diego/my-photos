//! Schema v1 (DB-04, DB-05, DB-06) exercised with plain SQL on a temp database.

use rusqlite::{params, Connection, ErrorCode};

use super::tests::Scratch;
use super::{latest_version, open};

fn library() -> (Scratch, Connection) {
    let scratch = Scratch::new("schema");
    let connection = open(&scratch.path.join("library.db")).expect("open");
    connection
        .execute(
            "INSERT INTO libraries (name, root_path, created_at) VALUES ('E', 'E:\\', '2026-10-02T12:00:00Z')",
            [],
        )
        .unwrap();
    (scratch, connection)
}

fn insert_media(connection: &Connection, relative_path: &str) -> rusqlite::Result<i64> {
    connection.execute(
        "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, created_at, updated_at)
         VALUES (1, ?1, 'foto.jpg', 'jpg', 'photo', '2026-10-02T12:00:00Z', '2026-10-02T12:00:00Z')",
        [relative_path],
    )?;
    Ok(connection.last_insert_rowid())
}

fn code(error: rusqlite::Error) -> ErrorCode {
    error.sqlite_error_code().expect("sqlite error")
}

fn names(connection: &Connection, kind: &str) -> Vec<String> {
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_schema WHERE type = ?1 AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap();
    statement
        .query_map([kind], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

fn count(connection: &Connection, sql: &str, id: i64) -> i64 {
    connection.query_row(sql, [id], |row| row.get(0)).unwrap()
}

#[test]
fn v1_has_exactly_the_spec_tables_and_indexes() {
    let (_scratch, connection) = library();
    assert_eq!(latest_version(), 1);
    let mut expected = vec![
        "libraries",
        "folders",
        "media_items",
        "media_locations",
        "people",
        "media_people",
        "albums",
        "media_albums",
        "media_metadata_raw",
        "duplicate_groups",
        "duplicate_group_members",
        "user_notes",
        "trash_entries",
        "indexing_runs",
        "indexing_errors",
        "app_settings",
    ];
    expected.sort_unstable();
    assert_eq!(names(&connection, "table"), expected);
    assert_eq!(
        names(&connection, "index"),
        vec!["idx_media_hash", "idx_media_taken_at", "idx_media_type", "idx_trash_purge"]
    );
}

#[test]
fn duplicate_paths_violate_unique_for_media_and_folders() {
    let (_scratch, connection) = library();
    insert_media(&connection, "Fotos de 2025/foto.jpg").unwrap();
    let error = insert_media(&connection, "Fotos de 2025/foto.jpg").unwrap_err();
    assert_eq!(code(error), ErrorCode::ConstraintViolation);

    let folder = "INSERT INTO folders (library_id, relative_path, created_at) VALUES (1, 'Fotos de 2025', '2026-10-02T12:00:00Z')";
    connection.execute(folder, []).unwrap();
    let error = connection.execute(folder, []).unwrap_err();
    assert_eq!(code(error), ErrorCode::ConstraintViolation);
}

#[test]
fn media_domains_reject_values_outside_the_spec() {
    let (_scratch, connection) = library();
    let id = insert_media(&connection, "a.jpg").unwrap();
    for (column, bad) in [
        ("media_type", "audio"),
        ("json_match_confidence", "maybe"),
        ("taken_at_source", "guess"),
        ("indexing_status", "trashed"),
    ] {
        let error = connection
            .execute(
                &format!("UPDATE media_items SET {column} = ?1 WHERE id = ?2"),
                params![bad, id],
            )
            .unwrap_err();
        assert_eq!(code(error), ErrorCode::ConstraintViolation, "{column}");
    }
    for (column, good) in [
        ("media_type", "video"),
        ("json_match_confidence", "heuristic"),
        ("taken_at_source", "takeout_json"),
        ("indexing_status", "missing_file"),
    ] {
        connection
            .execute(
                &format!("UPDATE media_items SET {column} = ?1 WHERE id = ?2"),
                params![good, id],
            )
            .unwrap_or_else(|error| panic!("{column}: {error}"));
    }
}

#[test]
fn unknown_library_violates_the_foreign_key() {
    let (_scratch, connection) = library();
    let error = connection
        .execute(
            "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, created_at, updated_at)
             VALUES (99, 'x.jpg', 'x.jpg', 'jpg', 'photo', 't', 't')",
            [],
        )
        .unwrap_err();
    assert_eq!(code(error), ErrorCode::ConstraintViolation);
}

fn link_everything(connection: &Connection, media: i64) {
    connection
        .execute_batch(&format!(
            "UPDATE media_items SET is_favorite = 1 WHERE id = {media};
             INSERT INTO media_locations (media_id, latitude, longitude) VALUES ({media}, -23.5, -46.6);
             INSERT INTO people (library_id, name, normalized_name) VALUES (1, 'Pessoa A', 'pessoa a');
             INSERT INTO media_people (media_id, person_id) VALUES ({media}, last_insert_rowid());
             INSERT INTO albums (library_id, name, source, created_at) VALUES (1, 'Viagem', 'local', 't');
             INSERT INTO media_albums (media_id, album_id) VALUES ({media}, last_insert_rowid());
             INSERT INTO media_metadata_raw (media_id, takeout_json) VALUES ({media}, '{{}}');
             INSERT INTO user_notes (media_id, note) VALUES ({media}, 'nota');
             INSERT INTO duplicate_groups (library_id, match_type, representative_media_id) VALUES (1, 'exact_hash', {media});
             INSERT INTO duplicate_group_members (group_id, media_id) VALUES (last_insert_rowid(), {media});
             INSERT INTO trash_entries (library_id, kind, media_id, original_relative_path, trash_relative_path, trashed_at)
               VALUES (1, 'file', {media}, 'Fotos de 2025/foto.jpg', '.myphotos/trash/1/foto.jpg', '2026-10-02T12:00:00Z');"
        ))
        .unwrap();
}

const CHILD_COUNTS: [&str; 6] = [
    "SELECT count(*) FROM media_locations WHERE media_id = ?1",
    "SELECT count(*) FROM media_people WHERE media_id = ?1",
    "SELECT count(*) FROM media_albums WHERE media_id = ?1",
    "SELECT count(*) FROM media_metadata_raw WHERE media_id = ?1",
    "SELECT count(*) FROM user_notes WHERE media_id = ?1",
    "SELECT count(*) FROM duplicate_group_members WHERE media_id = ?1",
];

#[test]
fn moving_a_media_keeps_its_id_and_every_link() {
    let (_scratch, connection) = library();
    let id = insert_media(&connection, "Fotos de 2025/foto.jpg").unwrap();
    link_everything(&connection, id);

    connection
        .execute(
            "UPDATE media_items SET relative_path = 'Viagem/foto.jpg' WHERE id = ?1",
            [id],
        )
        .unwrap();

    let moved: i64 = connection
        .query_row(
            "SELECT id FROM media_items WHERE relative_path = 'Viagem/foto.jpg'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(moved, id);
    assert_eq!(count(&connection, "SELECT is_favorite FROM media_items WHERE id = ?1", id), 1);
    for sql in CHILD_COUNTS {
        assert_eq!(count(&connection, sql, id), 1, "{sql}");
    }
}

#[test]
fn deleted_highest_id_is_never_reused() {
    let (_scratch, connection) = library();
    insert_media(&connection, "a.jpg").unwrap();
    let highest = insert_media(&connection, "b.jpg").unwrap();
    connection
        .execute("DELETE FROM media_items WHERE id = ?1", [highest])
        .unwrap();
    let next = insert_media(&connection, "c.jpg").unwrap();
    assert!(next > highest, "{next} <= {highest}");
}

#[test]
fn deleting_a_media_cascades_children_and_keeps_the_trash_paths() {
    let (_scratch, connection) = library();
    let id = insert_media(&connection, "Fotos de 2025/foto.jpg").unwrap();
    link_everything(&connection, id);

    connection
        .execute("DELETE FROM media_items WHERE id = ?1", [id])
        .unwrap();

    for sql in CHILD_COUNTS {
        assert_eq!(count(&connection, sql, id), 0, "{sql}");
    }
    let (media_id, original, trash): (Option<i64>, String, String) = connection
        .query_row(
            "SELECT media_id, original_relative_path, trash_relative_path FROM trash_entries",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(media_id, None);
    assert_eq!(original, "Fotos de 2025/foto.jpg");
    assert_eq!(trash, ".myphotos/trash/1/foto.jpg");
}

#[test]
fn trash_entry_is_purged_thirty_days_after_it_was_trashed() {
    let (_scratch, connection) = library();
    connection
        .execute(
            "INSERT INTO trash_entries (library_id, kind, original_relative_path, trash_relative_path, trashed_at)
             VALUES (1, 'folder', 'Antiga', '.myphotos/trash/2/Antiga', '2026-10-02T12:00:00Z')",
            [],
        )
        .unwrap();
    let purge_after: String = connection
        .query_row("SELECT purge_after FROM trash_entries", [], |row| row.get(0))
        .unwrap();
    assert_eq!(purge_after, "2026-11-01T12:00:00Z");
}
