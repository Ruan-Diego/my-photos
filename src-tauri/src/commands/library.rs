//! F15: what the sidebar's disk card shows (artboard `Sidebar`).

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use tauri::State;

use super::health::DbState;
use crate::db::DbError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySummary {
    /// False when the library disk or its database could not be opened.
    pub connected: bool,
    /// `E:\`; `None` before the first scan or when disconnected.
    pub root_path: Option<String>,
    pub volume_label: Option<String>,
    /// Media the timeline shows: hidden and missing files left out.
    pub media_count: u64,
    /// UTC, from the last completed scan.
    pub last_indexed_at: Option<String>,
    /// Why the disk is not connected, in Portuguese.
    pub problem: Option<String>,
}

impl LibrarySummary {
    fn disconnected(error: DbError) -> Self {
        Self {
            connected: false,
            root_path: None,
            volume_label: None,
            media_count: 0,
            last_indexed_at: None,
            problem: Some(error.message().to_string()),
        }
    }
}

/// There is one library per disk (AD-011): the first row is it.
pub fn summary(connection: &Connection) -> rusqlite::Result<LibrarySummary> {
    let library = connection
        .query_row(
            "SELECT root_path, volume_label, last_indexed_at FROM libraries ORDER BY id LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let media_count: i64 = connection.query_row(
        "SELECT count(*) FROM media_items WHERE is_hidden = 0 AND indexing_status != 'missing_file'",
        [],
        |row| row.get(0),
    )?;
    let (root_path, volume_label, last_indexed_at) = library.unwrap_or((None, None, None));
    Ok(LibrarySummary {
        connected: true,
        root_path,
        volume_label,
        media_count: media_count.max(0) as u64,
        last_indexed_at,
        problem: None,
    })
}

#[tauri::command]
pub fn library_summary(state: State<'_, DbState>) -> LibrarySummary {
    let slot = match &state.0 {
        Ok(slot) => slot,
        Err(error) => return LibrarySummary::disconnected(*error),
    };
    let guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    match guard.as_ref() {
        Some(connection) => {
            summary(connection).unwrap_or_else(|_| LibrarySummary::disconnected(DbError::OpenFailed))
        }
        None => LibrarySummary::disconnected(DbError::OpenFailed),
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::{params, Connection};

    use super::{summary, LibrarySummary};
    use crate::db::{DbError, MIGRATIONS};

    const NOW: &str = "2026-10-15T14:02:00Z";

    fn database() -> Connection {
        let connection = Connection::open_in_memory().unwrap();
        for migration in MIGRATIONS {
            connection.execute_batch(migration.sql).unwrap();
        }
        connection
    }

    fn add_media(connection: &Connection, path: &str, hidden: bool, status: &str) {
        connection
            .execute(
                "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type,
                   is_hidden, indexing_status, created_at, updated_at)
                 VALUES (1, ?1, ?1, 'jpg', 'photo', ?2, ?3, ?4, ?4)",
                params![path, hidden, status, NOW],
            )
            .unwrap();
    }

    #[test]
    fn shell01_counts_what_the_timeline_shows() {
        let connection = database();
        connection
            .execute(
                "INSERT INTO libraries (name, root_path, volume_label, created_at, last_indexed_at)
                 VALUES ('E', 'E:\\', 'FOTOS', ?1, ?1)",
                params![NOW],
            )
            .unwrap();
        add_media(&connection, "a.jpg", false, "ok");
        add_media(&connection, "b.jpg", false, "error");
        add_media(&connection, "c.jpg", true, "ok");
        add_media(&connection, "d.jpg", false, "missing_file");

        let summary = summary(&connection).unwrap();
        assert_eq!(
            summary,
            LibrarySummary {
                connected: true,
                root_path: Some("E:\\".into()),
                volume_label: Some("FOTOS".into()),
                media_count: 2,
                last_indexed_at: Some(NOW.into()),
                problem: None,
            }
        );
        assert_eq!(
            serde_json::to_string(&summary).unwrap(),
            r#"{"connected":true,"rootPath":"E:\\","volumeLabel":"FOTOS","mediaCount":2,"lastIndexedAt":"2026-10-15T14:02:00Z","problem":null}"#
        );
    }

    #[test]
    fn shell01_a_library_never_scanned_is_connected_and_empty() {
        let summary = summary(&database()).unwrap();
        assert!(summary.connected);
        assert_eq!(summary.root_path, None);
        assert_eq!(summary.media_count, 0);
        assert_eq!(summary.last_indexed_at, None);
    }

    #[test]
    fn shell01_a_missing_disk_is_disconnected_with_its_message() {
        let summary = LibrarySummary::disconnected(DbError::VolumeMissing);
        assert!(!summary.connected);
        assert_eq!(summary.problem.as_deref(), Some("disco da biblioteca não encontrado"));
    }
}
