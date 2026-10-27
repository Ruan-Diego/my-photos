use rusqlite::Connection;

use super::{distance_km, fill_place_labels, label, nearest};
use crate::db::MIGRATIONS;

#[test]
fn plc01_coordinates_name_the_nearest_town() {
    // Avenida Paulista, Praça do Comércio, Ribeira do Porto, Times Square.
    assert_eq!(label(-23.5614, -46.6559).as_deref(), Some("São Paulo, SP, Brasil"));
    assert_eq!(label(38.7076, -9.1365).as_deref(), Some("Lisbon, Portugal"));
    assert_eq!(label(41.1406, -8.6110).as_deref(), Some("Porto, Portugal"));
    assert_eq!(label(-27.5954, -48.5480).as_deref(), Some("Florianópolis, SC, Brasil"));
    assert_eq!(label(40.7580, -73.9855).as_deref(), Some("New York City, NY, Estados Unidos"));
    // Downtown Osasco, a city of its own next to São Paulo.
    assert_eq!(label(-23.5329, -46.7917).as_deref(), Some("Osasco, SP, Brasil"));
    // Mid-Atlantic, (0, 0) and nonsense: no town.
    assert_eq!(label(-20.0, -25.0), None);
    assert_eq!(label(0.0, 0.0), None);
    assert_eq!(label(f64::NAN, 10.0), None);
    assert_eq!(label(95.0, 10.0), None);
    assert!((distance_km((-23.5505, -46.6333), (-22.9068, -43.1729)) - 357.0).abs() < 5.0);
}

#[test]
fn plc02_located_rows_without_a_place_get_one_and_the_index_follows() {
    let mut connection = Connection::open_in_memory().unwrap();
    for migration in MIGRATIONS {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
        .execute_batch(
            "INSERT INTO libraries (name, root_path, created_at) VALUES ('E', 'E:\\', 't');
             INSERT INTO media_items (id, library_id, relative_path, filename, extension, media_type, created_at, updated_at)
               VALUES (1, 1, 'a.jpg', 'a.jpg', 'jpg', 'photo', 't', 't'),
                      (2, 1, 'b.jpg', 'b.jpg', 'jpg', 'photo', 't', 't'),
                      (3, 1, 'c.jpg', 'c.jpg', 'jpg', 'photo', 't', 't');
             INSERT INTO media_locations (media_id, latitude, longitude, source) VALUES
               (1, 38.7076, -9.1365, 'takeout_geoData'),
               (2, -20.0, -25.0, 'exif');
             INSERT INTO media_locations (media_id, latitude, longitude, source, place_label) VALUES
               (3, 38.7076, -9.1365, 'exif', 'Já tinha nome');",
        )
        .unwrap();

    assert_eq!(fill_place_labels(&mut connection).unwrap(), 1);
    let labels: Vec<Option<String>> = connection
        .prepare("SELECT place_label FROM media_locations ORDER BY media_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(labels, vec![Some("Lisbon, Portugal".into()), None, Some("Já tinha nome".into())]);
    let found: Vec<i64> = connection
        .prepare("SELECT rowid FROM media_search WHERE media_search MATCH 'place : lisbon' ORDER BY rowid")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(found, vec![1], "the search index reads the new place by trigger");
    assert_eq!(fill_place_labels(&mut connection).unwrap(), 0, "the one at sea stays unnamed");
}
