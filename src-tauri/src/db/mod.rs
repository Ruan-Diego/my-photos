use std::fs;
use std::path::{Component, Path, PathBuf};

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

pub(crate) fn open_with(path: &Path, migrations: &[Migration]) -> Result<Connection, DbError> {
    let volume = volume_root(path).ok_or(DbError::PathNotAbsolute)?;
    if !volume.exists() {
        return Err(DbError::VolumeMissing);
    }
    ensure_parent(path)?;
    let connection = Connection::open(path).map_err(|_| DbError::OpenFailed)?;
    let _ = migrations;
    Ok(connection)
}

/// `E:\.myphotos\library.db` → `E:\`. Relative, drive-relative and UNC paths have none.
fn volume_root(path: &Path) -> Option<PathBuf> {
    let mut components = path.components();
    let Some(Component::Prefix(prefix)) = components.next() else {
        return None;
    };
    if !matches!(
        prefix.kind(),
        std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
    ) {
        return None;
    }
    if components.next() != Some(Component::RootDir) {
        return None;
    }
    Some(PathBuf::from(prefix.as_os_str()).join(r"\"))
}

/// Creates the database folder, hidden, only when the folder above it exists.
fn ensure_parent(path: &Path) -> Result<(), DbError> {
    let parent = path.parent().ok_or(DbError::ParentMissing)?;
    if parent.is_dir() {
        return Ok(());
    }
    let above = parent.parent().ok_or(DbError::ParentMissing)?;
    if !above.is_dir() {
        return Err(DbError::ParentMissing);
    }
    fs::create_dir(parent).map_err(|_| DbError::OpenFailed)?;
    hide(parent);
    Ok(())
}

#[cfg(windows)]
fn hide(folder: &Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN};

    let wide: Vec<u16> = folder.as_os_str().encode_wide().chain(Some(0)).collect();
    // A folder that stays visible still works, so the result is not an error.
    unsafe {
        SetFileAttributesW(wide.as_ptr(), FILE_ATTRIBUTE_HIDDEN);
    }
}

#[cfg(not(windows))]
fn hide(_folder: &Path) {}

pub fn select_one() -> Result<i64, String> {
    let connection = Connection::open_in_memory().map_err(|error| error.to_string())?;
    connection
        .query_row("SELECT 1", [], |row| row.get(0))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::{latest_version_of, open_with, select_one, DbError, Migration};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    const NO_MIGRATIONS: &[Migration] = &[];

    pub(super) struct Scratch {
        pub path: PathBuf,
    }

    impl Scratch {
        pub fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "my-photos-f04-{label}-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn is_hidden(path: &Path) -> bool {
        use std::os::windows::fs::MetadataExt;
        fs::metadata(path).unwrap().file_attributes() & 0x2 != 0
    }

    #[test]
    fn relative_path_is_rejected_without_creating_anything() {
        let relative = Path::new(r"f04-relative\.myphotos\library.db");
        let error = open_with(relative, NO_MIGRATIONS).unwrap_err();
        assert_eq!(error.as_code(), "path_not_absolute");
        assert!(!Path::new("f04-relative").exists());
    }

    #[test]
    fn missing_volume_is_rejected_without_creating_anything() {
        let letter = ('A'..='Z')
            .rev()
            .find(|letter| !Path::new(&format!(r"{letter}:\")).exists())
            .expect("a free drive letter");
        let path = PathBuf::from(format!(r"{letter}:\.myphotos\library.db"));
        let error = open_with(&path, NO_MIGRATIONS).unwrap_err();
        assert_eq!(error.as_code(), "volume_missing");
        assert!(!Path::new(&format!(r"{letter}:\")).exists());
    }

    #[test]
    fn missing_grandparent_is_rejected_without_creating_anything() {
        let scratch = Scratch::new("grandparent");
        let absent = scratch.path.join("absent");
        let path = absent.join(".myphotos").join("library.db");
        let error = open_with(&path, NO_MIGRATIONS).unwrap_err();
        assert_eq!(error.as_code(), "parent_missing");
        assert!(!absent.exists());
    }

    #[test]
    fn missing_folder_is_created_hidden_with_the_database_inside() {
        let scratch = Scratch::new("hidden");
        let folder = scratch.path.join(".myphotos");
        let path = folder.join("library.db");
        let connection = open_with(&path, NO_MIGRATIONS).expect("open");
        drop(connection);
        assert!(folder.is_dir());
        assert!(is_hidden(&folder));
        assert!(path.is_file());
    }

    #[test]
    fn existing_visible_folder_keeps_its_attributes() {
        let scratch = Scratch::new("visible");
        let folder = scratch.path.join(".myphotos");
        fs::create_dir(&folder).unwrap();
        let path = folder.join("library.db");
        drop(open_with(&path, NO_MIGRATIONS).expect("open"));
        assert!(!is_hidden(&folder));
        assert!(path.is_file());
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
