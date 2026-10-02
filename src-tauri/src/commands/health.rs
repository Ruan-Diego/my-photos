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
pub struct DbState(pub Result<Mutex<Connection>, DbError>);

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
            let connection = connection.lock().map_err(|_| DbError::OpenFailed.message())?;
            db::schema_version(&connection)
        }
        Err(error) => Err(*error),
    };
    health_from_open(version)
}

#[cfg(test)]
mod tests {
    use super::health_from_open;
    use crate::db::DbError;

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
            "disco E: não encontrado"
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
