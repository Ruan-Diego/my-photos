//! F23: the people screen (artboard `People`): who Google Photos tagged, read from the
//! Takeout JSONs, with how many items each and a cover. Read only (AD-008).

use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

use super::health::DbState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonCard {
    pub id: i64,
    pub name: String,
    /// Items the timeline shows with this person.
    pub count: u64,
    /// The newest photo with them (a video only when there is no photo).
    pub cover: Option<i64>,
}

/// Everyone with at least one visible item, most items first, then by name.
pub fn people_cards(connection: &Connection) -> rusqlite::Result<Vec<PersonCard>> {
    let mut statement = connection.prepare(
        "SELECT p.id, p.name, count(*),
                (SELECT c.id FROM media_people cp JOIN media_items c ON c.id = cp.media_id
                  WHERE cp.person_id = p.id AND c.is_hidden = 0 AND c.indexing_status != 'missing_file'
                  ORDER BY c.media_type = 'photo' DESC, c.taken_at IS NULL, c.taken_at DESC, c.id DESC
                  LIMIT 1)
         FROM people p
         JOIN media_people mp ON mp.person_id = p.id
         JOIN media_items m ON m.id = mp.media_id
         WHERE m.is_hidden = 0 AND m.indexing_status != 'missing_file'
         GROUP BY p.id ORDER BY count(*) DESC, p.normalized_name",
    )?;
    let cards = statement
        .query_map([], |row| {
            Ok(PersonCard {
                id: row.get(0)?,
                name: row.get(1)?,
                count: row.get::<_, i64>(2)?.max(0) as u64,
                cover: row.get(3)?,
            })
        })?
        .collect();
    cards
}

#[tauri::command]
pub fn people_list(state: State<'_, DbState>) -> Result<Vec<PersonCard>, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    people_cards(connection).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::{people_cards, PersonCard};
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

    fn media(connection: &Connection, id: i64, kind: &str, taken_at: Option<&str>, hidden: bool, status: &str) {
        connection
            .execute(
                "INSERT INTO media_items (id, library_id, relative_path, filename, extension, media_type,
                   taken_at, is_hidden, indexing_status, created_at, updated_at)
                 VALUES (?1, 1, ?2, ?2, 'jpg', ?3, ?4, ?5, ?6, 't', 't')",
                params![id, format!("m{id}.jpg"), kind, taken_at, hidden, status],
            )
            .unwrap();
    }

    fn person(connection: &Connection, id: i64, name: &str, media: &[i64]) {
        connection
            .execute(
                "INSERT INTO people (id, library_id, name, normalized_name) VALUES (?1, 1, ?2, lower(?2))",
                params![id, name],
            )
            .unwrap();
        for media_id in media {
            connection
                .execute("INSERT INTO media_people (media_id, person_id) VALUES (?1, ?2)", [media_id, &id])
                .unwrap();
        }
    }

    #[test]
    fn ppl01_people_with_counts_and_newest_photo_as_cover() {
        let connection = database();
        media(&connection, 1, "photo", Some("2021-01-01T00:00:00Z"), false, "ok");
        media(&connection, 2, "photo", Some("2023-05-01T00:00:00Z"), false, "ok");
        media(&connection, 3, "video", Some("2024-01-01T00:00:00Z"), false, "ok");
        media(&connection, 4, "photo", Some("2025-01-01T00:00:00Z"), true, "ok");
        media(&connection, 5, "photo", Some("2025-02-01T00:00:00Z"), false, "missing_file");
        media(&connection, 6, "video", None, false, "ok");
        person(&connection, 10, "Bia", &[1, 2, 3, 4, 5]);
        person(&connection, 11, "Ana", &[6]);
        person(&connection, 12, "Caio", &[2]);
        person(&connection, 13, "Davi", &[4, 5]);

        let cards = people_cards(&connection).unwrap();
        assert_eq!(
            cards,
            vec![
                // Hidden and missing items neither count nor cover; a photo beats a newer video.
                PersonCard { id: 10, name: "Bia".into(), count: 3, cover: Some(2) },
                // Same count: by name.
                PersonCard { id: 11, name: "Ana".into(), count: 1, cover: Some(6) },
                PersonCard { id: 12, name: "Caio".into(), count: 1, cover: Some(2) },
            ],
            "Davi has nothing visible and is left out"
        );
        assert_eq!(
            serde_json::to_string(&cards[0]).unwrap(),
            r#"{"id":10,"name":"Bia","count":3,"cover":2}"#
        );
    }
}
