//! F21: the Details panel of the viewer and the player (artboard `Viewer`): where the
//! date came from, the GPS point, the people from the Takeout JSON, the file and the
//! technical lines. Read only (AD-008); asked only when the panel is open.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tauri::State;

use super::health::DbState;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaPanel {
    pub id: i64,
    /// `takeout_json`, `exif` or `filesystem`; `None` without a date.
    pub date_source: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    /// Names from the Takeout JSON, in alphabetical order.
    pub people: Vec<String>,
    /// `E:\Takeout\Google Fotos\Viagem Europa 2022`
    pub folder: String,
    pub duration_ms: Option<i64>,
    /// File name of the paired Takeout JSON.
    pub json_filename: Option<String>,
    /// `exact`, `heuristic` or `none`.
    pub json_match: Option<String>,
    /// "Apple iPhone 12" from the EXIF Make and Model.
    pub camera: Option<String>,
    pub sha256: Option<String>,
}

/// Windows path of the folder holding `relative` under `root`.
pub(crate) fn folder_path(root: &str, relative: &str) -> String {
    let relative = relative.replace('/', "\\");
    let parent = relative.rsplit_once('\\').map(|(parent, _)| parent).unwrap_or("");
    let root = root.trim_end_matches(['\\', '/']);
    if parent.is_empty() {
        format!("{root}\\")
    } else {
        format!("{root}\\{parent}")
    }
}

/// `Make` and `Model` from the stored EXIF (`"Apple"` comes quoted); the model alone
/// when it already starts with the make ("Canon EOS 80D").
pub(crate) fn camera(exif_json: &str) -> Option<String> {
    let map: serde_json::Map<String, serde_json::Value> = serde_json::from_str(exif_json).ok()?;
    let text = |tag: &str| {
        map.get(tag)
            .and_then(|value| value.as_str())
            .map(|value| value.trim().trim_matches('"').trim().to_string())
            .filter(|value| !value.is_empty())
    };
    match (text("Make"), text("Model")) {
        (Some(make), Some(model)) if model.to_lowercase().starts_with(&make.to_lowercase()) => Some(model),
        (Some(make), Some(model)) => Some(format!("{make} {model}")),
        (make, model) => model.or(make),
    }
}

pub fn media_panel(connection: &Connection, media_id: i64) -> rusqlite::Result<Option<MediaPanel>> {
    let Some(mut panel) = connection
        .query_row(
            "SELECT m.id, m.taken_at_source, loc.latitude, loc.longitude, l.root_path, m.relative_path,
                    m.duration_ms, m.json_relative_path, m.json_match_confidence, raw.exif_json,
                    m.file_hash_sha256
             FROM media_items m
             JOIN libraries l ON l.id = m.library_id
             LEFT JOIN media_locations loc ON loc.media_id = m.id
             LEFT JOIN media_metadata_raw raw ON raw.media_id = m.id
             WHERE m.id = ?1",
            [media_id],
            |row| {
                let root: String = row.get(4)?;
                let relative: String = row.get(5)?;
                let json: Option<String> = row.get(7)?;
                let exif: Option<String> = row.get(9)?;
                Ok(MediaPanel {
                    id: row.get(0)?,
                    date_source: row.get(1)?,
                    latitude: row.get(2)?,
                    longitude: row.get(3)?,
                    people: Vec::new(),
                    folder: folder_path(&root, &relative),
                    duration_ms: row.get(6)?,
                    json_filename: json.map(|path| path.rsplit(['/', '\\']).next().unwrap_or(&path).to_string()),
                    json_match: row.get(8)?,
                    camera: exif.as_deref().and_then(camera),
                    sha256: row.get(10)?,
                })
            },
        )
        .optional()?
    else {
        return Ok(None);
    };
    let mut statement = connection.prepare(
        "SELECT p.name FROM media_people mp JOIN people p ON p.id = mp.person_id
         WHERE mp.media_id = ?1 ORDER BY p.normalized_name",
    )?;
    panel.people = statement
        .query_map([media_id], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(Some(panel))
}

#[tauri::command]
pub fn media_panel_details(state: State<'_, DbState>, id: i64) -> Result<Option<MediaPanel>, String> {
    let slot = state.0.as_ref().map_err(|error| error.message().to_string())?;
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let connection = guard.as_ref().ok_or("biblioteca fechada")?;
    media_panel(connection, id).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::{camera, folder_path, media_panel, MediaPanel};
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
            .execute(
                "INSERT INTO media_items (id, library_id, relative_path, filename, extension, media_type,
                   taken_at, taken_at_source, json_relative_path, json_match_confidence, file_hash_sha256,
                   created_at, updated_at)
                 VALUES (1, 1, 'Takeout/Google Fotos/Viagem/IMG_1.jpg', 'IMG_1.jpg', 'jpg', 'photo',
                   '2022-08-10T13:37:00Z', 'takeout_json', 'Takeout/Google Fotos/Viagem/IMG_1.jpg.supplemental-metadata.json',
                   'exact', 'ab12', 't', 't')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO media_items (id, library_id, relative_path, filename, extension, media_type,
                   duration_ms, created_at, updated_at)
                 VALUES (2, 1, 'VID_2.mp4', 'VID_2.mp4', 'mp4', 'video', 138000, 't', 't')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO media_locations (media_id, latitude, longitude, source) VALUES (1, 38.7139, -9.1394, 'takeout_geoData')",
                [],
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO media_metadata_raw (media_id, exif_json) VALUES (1, ?1)",
                [r#"{"Make":"\"Apple\"","Model":"\"iPhone 12\""}"#],
            )
            .unwrap();
        for (id, name, normalized) in [(1, "Zé", "ze"), (2, "Ana", "ana")] {
            connection
                .execute(
                    "INSERT INTO people (id, library_id, name, normalized_name) VALUES (?1, 1, ?2, ?3)",
                    params![id, name, normalized],
                )
                .unwrap();
            connection
                .execute("INSERT INTO media_people (media_id, person_id) VALUES (1, ?1)", [id])
                .unwrap();
        }
        connection
    }

    #[test]
    fn det01_the_panel_reads_date_source_place_people_and_file() {
        let connection = database();
        assert_eq!(
            media_panel(&connection, 1).unwrap(),
            Some(MediaPanel {
                id: 1,
                date_source: Some("takeout_json".into()),
                latitude: Some(38.7139),
                longitude: Some(-9.1394),
                people: vec!["Ana".into(), "Zé".into()],
                folder: r"E:\Takeout\Google Fotos\Viagem".into(),
                duration_ms: None,
                json_filename: Some("IMG_1.jpg.supplemental-metadata.json".into()),
                json_match: Some("exact".into()),
                camera: Some("Apple iPhone 12".into()),
                sha256: Some("ab12".into()),
            })
        );
        let video = media_panel(&connection, 2).unwrap().unwrap();
        assert_eq!((video.date_source, video.latitude, video.people.len()), (None, None, 0));
        assert_eq!((video.folder.as_str(), video.duration_ms), (r"E:\", Some(138000)));
        assert_eq!(media_panel(&connection, 99).unwrap(), None);
    }

    #[test]
    fn det02_camera_and_folder_read_like_the_canvas() {
        assert_eq!(camera(r#"{"Make":"\"Canon\"","Model":"\"Canon EOS 80D\""}"#).as_deref(), Some("Canon EOS 80D"));
        assert_eq!(camera(r#"{"Model":"\"SM-G991B\""}"#).as_deref(), Some("SM-G991B"));
        assert_eq!(camera(r#"{"Make":"\"\""}"#), None);
        assert_eq!(camera("not json"), None);
        assert_eq!(folder_path(r"F:\", r"Ruan\2020\a.jpg"), r"F:\Ruan\2020");
    }
}
