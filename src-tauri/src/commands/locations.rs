//! F24: the places screen (artboard `Locations`): items with and without coordinates,
//! and the named places (F24 `places`) with their count, dates, center and cover.
//! Read only (AD-008).

use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

use super::health::DbState;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    /// `media_locations.place_label`: "São Paulo, SP, Brasil".
    pub label: String,
    pub count: u64,
    /// Mean of the items' coordinates: where the pin goes.
    pub latitude: f64,
    pub longitude: f64,
    /// Oldest and newest `taken_at` (UTC), when any item has a date.
    pub first: Option<String>,
    pub last: Option<String>,
    /// The newest photo there (a video only when there is no photo).
    pub cover: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Locations {
    /// Items the timeline shows that have coordinates.
    pub located: u64,
    pub unlocated: u64,
    /// Named places, most items first.
    pub places: Vec<Place>,
}

const VISIBLE: &str = "m.is_hidden = 0 AND m.indexing_status != 'missing_file'";

pub fn locations(connection: &Connection) -> rusqlite::Result<Locations> {
    let (located, total): (i64, i64) = connection.query_row(
        &format!(
            "SELECT coalesce(sum(l.latitude IS NOT NULL AND l.longitude IS NOT NULL), 0), count(*)
             FROM media_items m LEFT JOIN media_locations l ON l.media_id = m.id
             WHERE {VISIBLE}"
        ),
        [],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let places = connection
        .prepare(&format!(
            "SELECT l.place_label, count(*), avg(l.latitude), avg(l.longitude), min(m.taken_at), max(m.taken_at),
                    (SELECT c.id FROM media_locations cl JOIN media_items c ON c.id = cl.media_id
                      WHERE cl.place_label = l.place_label AND c.is_hidden = 0 AND c.indexing_status != 'missing_file'
                      ORDER BY c.media_type = 'photo' DESC, c.taken_at IS NULL, c.taken_at DESC, c.id DESC
                      LIMIT 1)
             FROM media_items m JOIN media_locations l ON l.media_id = m.id
             WHERE {VISIBLE} AND l.place_label IS NOT NULL AND l.latitude IS NOT NULL AND l.longitude IS NOT NULL
             GROUP BY l.place_label
             ORDER BY count(*) DESC, l.place_label"
        ))?
        .query_map([], |row| {
            Ok(Place {
                label: row.get(0)?,
                count: row.get::<_, i64>(1)?.max(0) as u64,
                latitude: row.get(2)?,
                longitude: row.get(3)?,
                first: row.get(4)?,
                last: row.get(5)?,
                cover: row.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Locations {
        located: located.max(0) as u64,
        unlocated: (total - located).max(0) as u64,
        places,
    })
}

#[tauri::command]
pub fn locations_overview(state: State<'_, DbState>) -> Result<Locations, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    locations(connection).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::locations;
    use crate::db::MIGRATIONS;

    fn media(connection: &Connection, id: i64, kind: &str, taken_at: Option<&str>, hidden: bool) {
        connection
            .execute(
                "INSERT INTO media_items (id, library_id, relative_path, filename, extension, media_type,
                   taken_at, is_hidden, created_at, updated_at)
                 VALUES (?1, 1, ?2, ?2, 'jpg', ?3, ?4, ?5, 't', 't')",
                params![id, format!("m{id}.jpg"), kind, taken_at, hidden],
            )
            .unwrap();
    }

    fn located(connection: &Connection, id: i64, latitude: f64, longitude: f64, place: Option<&str>) {
        connection
            .execute(
                "INSERT INTO media_locations (media_id, latitude, longitude, source, place_label) VALUES (?1, ?2, ?3, 'exif', ?4)",
                params![id, latitude, longitude, place],
            )
            .unwrap();
    }

    #[test]
    fn loc01_counts_places_with_dates_center_and_cover() {
        let connection = Connection::open_in_memory().unwrap();
        for migration in MIGRATIONS {
            connection.execute_batch(migration.sql).unwrap();
        }
        connection
            .execute("INSERT INTO libraries (name, root_path, created_at) VALUES ('E', 'E:\\', 't')", [])
            .unwrap();
        const LISBOA: &str = "Lisbon, Portugal";
        const SAO_PAULO: &str = "São Paulo, SP, Brasil";
        media(&connection, 1, "photo", Some("2022-08-10T10:00:00Z"), false);
        media(&connection, 2, "video", Some("2022-08-12T10:00:00Z"), false);
        media(&connection, 3, "photo", Some("2022-08-11T10:00:00Z"), false);
        media(&connection, 4, "photo", Some("2023-01-01T10:00:00Z"), true);
        media(&connection, 5, "photo", None, false);
        media(&connection, 6, "photo", Some("2015-03-01T10:00:00Z"), false);
        media(&connection, 7, "photo", Some("2024-03-01T10:00:00Z"), false);
        media(&connection, 8, "photo", None, false);
        located(&connection, 1, 38.70, -9.14, Some(LISBOA));
        located(&connection, 2, 38.72, -9.12, Some(LISBOA));
        located(&connection, 3, 38.71, -9.13, Some(LISBOA));
        located(&connection, 4, 38.71, -9.13, Some(LISBOA));
        located(&connection, 5, -20.0, -25.0, None);
        located(&connection, 6, -23.55, -46.63, Some(SAO_PAULO));
        located(&connection, 7, -23.55, -46.63, Some(SAO_PAULO));

        let found = locations(&connection).unwrap();
        // 8 visible: 6 with coordinates (the one at sea too), 1 without; the hidden one is out.
        assert_eq!((found.located, found.unlocated), (6, 1));
        assert_eq!(found.places.len(), 2, "the unnamed place at sea is not listed");
        let lisboa = &found.places[0];
        assert_eq!((lisboa.label.as_str(), lisboa.count), (LISBOA, 3));
        assert!((lisboa.latitude - 38.71).abs() < 1e-9 && (lisboa.longitude + 9.13).abs() < 1e-9);
        assert_eq!(lisboa.first.as_deref(), Some("2022-08-10T10:00:00Z"));
        assert_eq!(lisboa.last.as_deref(), Some("2022-08-12T10:00:00Z"));
        assert_eq!(lisboa.cover, Some(3), "newest photo, not the newer video nor the hidden one");
        let sao_paulo = &found.places[1];
        assert_eq!((sao_paulo.count, sao_paulo.cover), (2, Some(7)));
        assert_eq!(
            serde_json::to_value(sao_paulo).unwrap()["first"],
            serde_json::json!("2015-03-01T10:00:00Z")
        );
    }
}
