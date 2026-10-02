use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

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
    // Stable code for logs and tests; the UI shows message().
    #[allow(dead_code)]
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

/// Every published migration, in version order. Never edit one that shipped; add N+1.
pub(crate) const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("migrations/0001_initial.sql"),
}];

// Read by F05 and later; today only the schema tests use it.
#[allow(dead_code)]
pub fn latest_version() -> u32 {
    latest_version_of(MIGRATIONS)
}

/// AD-007 and AD-011: the library is E:\ and its database lives inside it.
pub const LIBRARY_DB_PATH: &str = r"E:\.myphotos\library.db";

pub fn open_library() -> Result<Connection, DbError> {
    open(Path::new(LIBRARY_DB_PATH))
}

pub fn open(path: &Path) -> Result<Connection, DbError> {
    open_with(path, MIGRATIONS)
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
    let mut connection = Connection::open(path).map_err(|_| DbError::OpenFailed)?;
    // Read before any write: journal_mode = WAL rewrites the header, and a file we
    // do not understand must stay byte-identical.
    let version = user_version(&connection)?;
    if version > latest_version_of(migrations) {
        return Err(DbError::NewerThanApp);
    }
    configure(&connection)?;
    migrate(&mut connection, path, version, migrations)?;
    Ok(connection)
}

fn migrate(
    connection: &mut Connection,
    path: &Path,
    version: u32,
    migrations: &[Migration],
) -> Result<(), DbError> {
    let mut pending: Vec<&Migration> = migrations
        .iter()
        .filter(|migration| migration.version > version)
        .collect();
    if pending.is_empty() {
        return Ok(());
    }
    pending.sort_by_key(|migration| migration.version);
    if version >= 1 {
        back_up(connection, path, version)?;
    }
    for migration in pending {
        // Dropping the transaction on any error rolls back the SQL and the version.
        let transaction = connection
            .transaction()
            .map_err(|_| DbError::MigrationFailed)?;
        transaction
            .execute_batch(migration.sql)
            .map_err(|_| DbError::MigrationFailed)?;
        transaction
            .pragma_update(None, "user_version", migration.version)
            .map_err(|_| DbError::MigrationFailed)?;
        transaction.commit().map_err(|_| DbError::MigrationFailed)?;
    }
    Ok(())
}

/// Copies the database to `<name>.bak-v<version>`, after folding the WAL into it so the
/// copy holds every committed page.
fn back_up(connection: &Connection, path: &Path, version: u32) -> Result<(), DbError> {
    let busy: i64 = connection
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| row.get(0))
        .map_err(|_| DbError::BackupFailed)?;
    if busy != 0 {
        return Err(DbError::BackupFailed);
    }
    fs::copy(path, backup_path(path, version)).map_err(|_| DbError::BackupFailed)?;
    Ok(())
}

fn backup_path(path: &Path, version: u32) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".bak-v{version}"));
    path.with_file_name(name)
}

pub fn schema_version(connection: &Connection) -> Result<u32, DbError> {
    user_version(connection)
}

fn user_version(connection: &Connection) -> Result<u32, DbError> {
    connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(|error| match error.sqlite_error_code() {
            Some(rusqlite::ErrorCode::NotADatabase) => DbError::NotSqlite,
            _ => DbError::OpenFailed,
        })
}

fn configure(connection: &Connection) -> Result<(), DbError> {
    connection
        .busy_timeout(Duration::from_millis(5000))
        .map_err(|_| DbError::OpenFailed)?;
    connection
        .pragma_update(None, "foreign_keys", "ON")
        .map_err(|_| DbError::OpenFailed)?;
    let mode: String = connection
        .pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))
        .map_err(|_| DbError::OpenFailed)?;
    if !mode.eq_ignore_ascii_case("wal") {
        return Err(DbError::OpenFailed);
    }
    // FULL because the library disk is USB and can be unplugged mid-write (AD-011).
    connection
        .pragma_update(None, "synchronous", "FULL")
        .map_err(|_| DbError::OpenFailed)
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

#[cfg(test)]
mod schema_tests;

#[cfg(test)]
pub(crate) mod tests {
    use super::{latest_version_of, open_with, DbError, Migration};
    use std::fs;
    use std::path::{Path, PathBuf};
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

    pub(crate) struct Scratch {
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

    const ONE: &[Migration] = &[Migration {
        version: 1,
        sql: "CREATE TABLE t (a);",
    }];

    fn sidecars_absent(path: &Path) -> bool {
        let wal = PathBuf::from(format!("{}-wal", path.display()));
        let shm = PathBuf::from(format!("{}-shm", path.display()));
        !wal.exists() && !shm.exists()
    }

    #[test]
    fn file_that_is_not_sqlite_is_rejected_and_left_identical() {
        let scratch = Scratch::new("garbage");
        let path = scratch.path.join("library.db");
        let bytes = b"isto nao e um banco sqlite, e tem mais de cem bytes para cobrir o cabecalho inteiro do formato".repeat(2);
        fs::write(&path, &bytes).unwrap();
        let error = open_with(&path, ONE).unwrap_err();
        assert_eq!(error.as_code(), "db_not_sqlite");
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(sidecars_absent(&path));
    }

    #[test]
    fn database_newer_than_the_app_is_rejected_and_left_identical() {
        let scratch = Scratch::new("newer");
        let path = scratch.path.join("library.db");
        {
            let connection = rusqlite::Connection::open(&path).unwrap();
            connection.execute_batch("CREATE TABLE x (a); PRAGMA user_version = 99;").unwrap();
        }
        let bytes = fs::read(&path).unwrap();
        let error = open_with(&path, ONE).unwrap_err();
        assert_eq!(error.as_code(), "db_newer_than_app");
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(sidecars_absent(&path));
    }

    #[test]
    fn opened_connection_has_the_spec_pragmas() {
        let scratch = Scratch::new("pragmas");
        let connection = open_with(&scratch.path.join("library.db"), ONE).expect("open");
        let pragma = |name: &str| -> String {
            connection
                .pragma_query_value(None, name, |row| row.get::<_, rusqlite::types::Value>(0))
                .map(|value| match value {
                    rusqlite::types::Value::Integer(number) => number.to_string(),
                    rusqlite::types::Value::Text(text) => text,
                    other => format!("{other:?}"),
                })
                .unwrap()
        };
        assert_eq!(pragma("foreign_keys"), "1");
        assert_eq!(pragma("journal_mode"), "wal");
        assert_eq!(pragma("synchronous"), "2");
        assert_eq!(pragma("busy_timeout"), "5000");
    }

    fn plain_version(path: &Path) -> u32 {
        rusqlite::Connection::open(path)
            .unwrap()
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    fn has_table(path: &Path, table: &str) -> bool {
        rusqlite::Connection::open(path)
            .unwrap()
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get::<_, i64>(0),
            )
            .unwrap()
            == 1
    }

    fn schema_of(connection: &rusqlite::Connection) -> Vec<String> {
        let mut statement = connection
            .prepare("SELECT coalesce(sql, name) FROM sqlite_schema ORDER BY name")
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    }

    fn backups_in(folder: &Path) -> Vec<String> {
        fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".bak-v"))
            .collect()
    }

    const TWO: &[Migration] = &[
        Migration {
            version: 1,
            sql: "CREATE TABLE t (a);",
        },
        Migration {
            version: 2,
            sql: "CREATE TABLE u (b);",
        },
    ];

    const BROKEN_TWO: &[Migration] = &[
        Migration {
            version: 1,
            sql: "CREATE TABLE t (a);",
        },
        Migration {
            version: 2,
            sql: "CREATE TABLE u (b); INSERT INTO nope VALUES (1);",
        },
    ];

    #[test]
    fn new_file_reaches_the_latest_version_without_a_backup() {
        let scratch = Scratch::new("new");
        let path = scratch.path.join("library.db");
        drop(open_with(&path, TWO).expect("open"));
        assert_eq!(plain_version(&path), 2);
        assert!(has_table(&path, "t") && has_table(&path, "u"));
        assert!(backups_in(&scratch.path).is_empty());
    }

    #[test]
    fn reopening_runs_nothing_and_keeps_the_schema() {
        let scratch = Scratch::new("reopen");
        let path = scratch.path.join("library.db");
        let before = schema_of(&open_with(&path, TWO).expect("first"));
        let again = open_with(&path, TWO).expect("second");
        assert_eq!(schema_of(&again), before);
        drop(again);
        assert_eq!(plain_version(&path), 2);
        assert!(backups_in(&scratch.path).is_empty());
    }

    #[test]
    fn existing_database_is_copied_before_it_is_migrated() {
        let scratch = Scratch::new("backup");
        let path = scratch.path.join("library.db");
        drop(open_with(&path, ONE).expect("v1"));
        let backup = scratch.path.join("library.db.bak-v1");
        fs::write(&backup, b"stale backup").unwrap();

        drop(open_with(&path, TWO).expect("v2"));

        assert_eq!(plain_version(&path), 2);
        assert_eq!(backups_in(&scratch.path), vec!["library.db.bak-v1".to_string()]);
        assert_eq!(plain_version(&backup), 1);
        assert!(has_table(&backup, "t"));
        assert!(!has_table(&backup, "u"));
    }

    #[test]
    fn backup_holds_rows_that_were_still_in_the_wal() {
        let scratch = Scratch::new("wal-backup");
        let path = scratch.path.join("library.db");
        // Kept open so closing it does not fold the WAL back into the main file.
        let writer = open_with(&path, ONE).expect("v1");
        writer.execute("INSERT INTO t VALUES ('in-wal')", []).unwrap();
        let wal = PathBuf::from(format!("{}-wal", path.display()));
        assert!(fs::metadata(&wal).unwrap().len() > 0, "row must sit in the WAL");

        drop(open_with(&path, TWO).expect("v2"));
        drop(writer);

        let backup = scratch.path.join("library.db.bak-v1");
        let value: String = rusqlite::Connection::open(&backup)
            .unwrap()
            .query_row("SELECT a FROM t", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "in-wal");
    }

    #[test]
    fn each_migration_commits_on_its_own() {
        let scratch = Scratch::new("own-transaction");
        let path = scratch.path.join("library.db");
        // Fresh file: version 1 must stay committed when version 2 fails.
        let error = open_with(&path, BROKEN_TWO).unwrap_err();
        assert_eq!(error.as_code(), "migration_failed");
        assert_eq!(plain_version(&path), 1);
        assert!(has_table(&path, "t"));
        assert!(!has_table(&path, "u"));
    }

    #[test]
    fn failed_migration_rolls_back_and_keeps_the_previous_version() {
        let scratch = Scratch::new("broken");
        let path = scratch.path.join("library.db");
        drop(open_with(&path, ONE).expect("v1"));
        let error = open_with(&path, BROKEN_TWO).unwrap_err();
        assert_eq!(error.as_code(), "migration_failed");
        assert_eq!(plain_version(&path), 1);
        assert!(has_table(&path, "t"));
        assert!(!has_table(&path, "u"));
    }

    #[test]
    fn pending_migrations_run_in_ascending_version_order() {
        let scratch = Scratch::new("order");
        let path = scratch.path.join("library.db");
        // Listed out of order; version 2 only works after version 1.
        let unordered = [
            Migration {
                version: 2,
                sql: "INSERT INTO t VALUES (2);",
            },
            Migration {
                version: 1,
                sql: "CREATE TABLE t (a);",
            },
        ];
        let connection = open_with(&path, &unordered).expect("open");
        let value: i64 = connection
            .query_row("SELECT a FROM t", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, 2);
        drop(connection);
        assert_eq!(plain_version(&path), 2);
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
}
