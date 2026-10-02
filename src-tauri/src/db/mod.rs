use rusqlite::Connection;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DbError {
    PathNotAbsolute,
    VolumeMissing,
    ParentMissing,
    OpenFailed,
    NotSqlite,
    NewerThanApp,
    BackupFailed,
    MigrationFailed,
}

impl DbError {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::PathNotAbsolute => "path_not_absolute",
            Self::VolumeMissing => "volume_missing",
            Self::ParentMissing => "parent_missing",
            Self::OpenFailed => "open_failed",
            Self::NotSqlite => "db_not_sqlite",
            Self::NewerThanApp => "db_newer_than_app",
            Self::BackupFailed => "backup_failed",
            Self::MigrationFailed => "migration_failed",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::PathNotAbsolute => "caminho do banco não é absoluto",
            // The only production path is LIBRARY_DB_PATH on E:.
            Self::VolumeMissing => "disco E: não encontrado",
            Self::ParentMissing => "pasta do banco não encontrada",
            Self::OpenFailed => "não foi possível abrir o banco",
            Self::NotSqlite => "o arquivo do banco não é um banco SQLite",
            Self::NewerThanApp => "o banco é de uma versão mais nova do My Photos",
            Self::BackupFailed => "não foi possível copiar o banco antes de atualizar",
            Self::MigrationFailed => "falha ao atualizar o banco",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Migration {
    pub version: u32,
    pub sql: &'static str,
}

pub(crate) fn latest_version_of(migrations: &[Migration]) -> u32 {
    migrations
        .iter()
        .map(|migration| migration.version)
        .max()
        .unwrap_or(0)
}

pub fn select_one() -> Result<i64, String> {
    let connection = Connection::open_in_memory().map_err(|error| error.to_string())?;
    connection
        .query_row("SELECT 1", [], |row| row.get(0))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{latest_version_of, select_one, DbError, Migration};

    #[test]
    fn every_error_has_the_design_code_and_message() {
        let expected = [
            (DbError::PathNotAbsolute, "path_not_absolute", "caminho do banco não é absoluto"),
            (DbError::VolumeMissing, "volume_missing", "disco E: não encontrado"),
            (DbError::ParentMissing, "parent_missing", "pasta do banco não encontrada"),
            (DbError::OpenFailed, "open_failed", "não foi possível abrir o banco"),
            (DbError::NotSqlite, "db_not_sqlite", "o arquivo do banco não é um banco SQLite"),
            (
                DbError::NewerThanApp,
                "db_newer_than_app",
                "o banco é de uma versão mais nova do My Photos",
            ),
            (
                DbError::BackupFailed,
                "backup_failed",
                "não foi possível copiar o banco antes de atualizar",
            ),
            (DbError::MigrationFailed, "migration_failed", "falha ao atualizar o banco"),
        ];
        for (error, code, message) in expected {
            assert_eq!(error.as_code(), code);
            assert_eq!(error.message(), message);
        }
    }

    #[test]
    fn latest_version_is_the_highest_listed_or_zero() {
        assert_eq!(latest_version_of(&[]), 0);
        let migrations = [
            Migration { version: 1, sql: "" },
            Migration { version: 2, sql: "" },
        ];
        assert_eq!(latest_version_of(&migrations), 2);
    }
    use std::env;
    use std::fs;
    use std::path::Path;

    #[test]
    fn select_one_returns_1() {
        let directory = env::current_dir().expect("cwd");
        let before = database_file_names(&directory);

        assert_eq!(select_one().expect("SELECT 1"), 1);
        assert_eq!(database_file_names(&directory), before);
    }

    #[test]
    fn second_call_returns_1_without_creating_a_db_file() {
        let directory = env::current_dir().expect("cwd");
        let before = database_file_names(&directory);

        assert_eq!(select_one().expect("first SELECT 1"), 1);
        assert_eq!(select_one().expect("second SELECT 1"), 1);
        assert_eq!(database_file_names(&directory), before);
    }

    fn database_file_names(directory: &Path) -> Vec<String> {
        let mut names = fs::read_dir(directory)
            .unwrap_or_else(|error| panic!("read {directory:?}: {error}"))
            .map(|entry| {
                entry
                    .unwrap_or_else(|error| panic!("dir entry: {error}"))
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .filter(|name| name.ends_with(".db"))
            .collect::<Vec<_>>();
        names.sort();
        names
    }
}
