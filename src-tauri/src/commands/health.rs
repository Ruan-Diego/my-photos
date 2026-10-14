use std::sync::Mutex;

use rusqlite::Connection;
use serde::Serialize;
use tauri::State;

use crate::db::{self, DbError};

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppHealth {
    pub app_name: String,
    pub schema_version: u32,
}

/// The library database opened at startup, or why it could not be opened.
pub struct DbState(pub Result<Mutex<Option<Connection>>, DbError>);

impl DbState {
    /// Checkpoints and closes the connection on app exit. Idempotent: the connection is
    /// taken out of the mutex, so a second call (or a failed open) does nothing.
    pub fn shutdown(&self) {
        if let Ok(slot) = &self.0 {
            let mut guard = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(connection) = guard.take() {
                db::close(connection);
            }
        }
    }
}

pub fn health_from_open(result: Result<u32, DbError>) -> Result<AppHealth, String> {
    match result {
        Ok(schema_version) => Ok(AppHealth {
            app_name: "My Photos".to_string(),
            schema_version,
        }),
        Err(error) => Err(error.message().to_string()),
    }
}

#[tauri::command]
pub fn app_health(state: State<'_, DbState>) -> Result<AppHealth, String> {
    let version = match &state.0 {
        Ok(connection) => {
            let guard = connection.lock().map_err(|_| DbError::OpenFailed.message())?;
            match guard.as_ref() {
                Some(connection) => db::schema_version(connection),
                None => Err(DbError::OpenFailed),
            }
        }
        Err(error) => Err(*error),
    };
    health_from_open(version)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use rusqlite::Connection;

    use super::{health_from_open, DbState};
    use crate::db::DbError;

    fn wal_connection(dir: &std::path::Path) -> (Connection, std::path::PathBuf) {
        let path = dir.join("library.db");
        let connection = Connection::open(&path).unwrap();
        connection.pragma_update(None, "journal_mode", "WAL").unwrap();
        // Keep SQLite from checkpointing on its own so the data stays in the -wal file.
        connection.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
        connection
            .execute_batch("CREATE TABLE t(x); INSERT INTO t VALUES (1), (2), (3);")
            .unwrap();
        (connection, path)
    }

    #[test]
    fn shutdown_folds_the_wal_into_the_database_file() {
        let dir = crate::db::tests::Scratch::new("shutdown");
        let (connection, path) = wal_connection(&dir.path);
        let wal = dir.path.join("library.db-wal");
        assert!(std::fs::metadata(&wal).unwrap().len() > 0, "data starts in the WAL");

        let state = DbState(Ok(Mutex::new(Some(connection))));
        state.shutdown();

        assert!(
            std::fs::metadata(&wal).map(|m| m.len()).unwrap_or(0) == 0,
            "WAL is truncated or removed"
        );
        // Read the main file alone: without the WAL, the rows must already be in it.
        let alone = Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let rows: i64 = alone.query_row("SELECT count(*) FROM t", [], |r| r.get(0)).unwrap();
        assert_eq!(rows, 3);
    }

    #[test]
    fn shutdown_twice_and_after_a_failed_open_do_nothing() {
        let dir = crate::db::tests::Scratch::new("shutdown-twice");
        let (connection, _) = wal_connection(&dir.path);
        let state = DbState(Ok(Mutex::new(Some(connection))));
        state.shutdown();
        state.shutdown();
        DbState(Err(DbError::VolumeMissing)).shutdown();
    }

    #[test]
    fn open_database_reports_my_photos_and_the_schema_version() {
        let health = health_from_open(Ok(1)).expect("health");
        assert_eq!(health.app_name, "My Photos");
        assert_eq!(health.schema_version, 1);
        assert_eq!(
            serde_json::to_string(&health).expect("json"),
            r#"{"appName":"My Photos","schemaVersion":1}"#
        );
    }

    #[test]
    fn every_open_error_becomes_its_portuguese_message() {
        assert_eq!(
            health_from_open(Err(DbError::VolumeMissing)).unwrap_err(),
            "disco da biblioteca não encontrado"
        );
        for error in [
            DbError::PathNotAbsolute,
            DbError::VolumeMissing,
            DbError::ParentMissing,
            DbError::OpenFailed,
            DbError::NotSqlite,
            DbError::NewerThanApp,
            DbError::BackupFailed,
            DbError::MigrationFailed,
        ] {
            assert_eq!(health_from_open(Err(error)).unwrap_err(), error.message());
        }
    }
}
