//! VOL-01..04. The volume lookups are faked; only `real_volume_has_a_serial` touches
//! Windows, and only the system drive.

use rusqlite::Connection;

use super::*;
use crate::db::MIGRATIONS;

struct Fake {
    roots: Vec<&'static str>,
    with_db: Vec<&'static str>,
}

impl Volumes for Fake {
    fn roots(&self) -> Vec<String> {
        self.roots.iter().map(|root| root.to_string()).collect()
    }

    fn has_library_db(&self, root: &str) -> bool {
        self.with_db.iter().any(|with| with.eq_ignore_ascii_case(root))
    }
}

fn locate(roots: &[&'static str], with_db: &[&'static str]) -> Result<String, LocateError> {
    locate_with(&Fake {
        roots: roots.to_vec(),
        with_db: with_db.to_vec(),
    })
}

fn database() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    for migration in MIGRATIONS {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
}

fn library(connection: &Connection, root: &str, serial: Option<&str>) -> i64 {
    connection
        .execute(
            "INSERT INTO libraries (name, root_path, volume_id, status, created_at)
             VALUES ('x', ?1, ?2, 'active', '2026-10-14T00:00:00.000Z')",
            rusqlite::params![root, serial],
        )
        .unwrap();
    connection.last_insert_rowid()
}

fn roots(connection: &Connection) -> Vec<String> {
    let mut statement = connection
        .prepare("SELECT root_path||' '||coalesce(volume_id,'-')||' '||coalesce(volume_label,'-') FROM libraries ORDER BY id")
        .unwrap();
    let rows = statement.query_map([], |row| row.get(0)).unwrap();
    rows.map(Result::unwrap).collect()
}

fn disk(serial: &str) -> VolumeInfo {
    VolumeInfo {
        serial: serial.to_string(),
        label: Some("Fotos".to_string()),
    }
}

// ---------------------------------------------------------------- VOL-01

#[test]
fn vol01_e_wins_when_it_has_the_library() {
    assert_eq!(locate(&[r"C:\", r"E:\", r"F:\"], &[r"E:\", r"F:\"]), Ok(r"E:\".to_string()));
}

#[test]
fn vol01_the_library_is_found_on_another_letter() {
    assert_eq!(locate(&[r"C:\", r"F:\"], &[r"F:\"]), Ok(r"F:\".to_string()));
    assert_eq!(locate(&[r"C:\", r"E:\", r"G:\"], &[r"G:\"]), Ok(r"G:\".to_string()));
}

#[test]
fn vol01_first_run_starts_on_e_and_nothing_else_is_guessed() {
    assert_eq!(locate(&[r"C:\", r"E:\"], &[]), Ok(r"E:\".to_string()));
    assert_eq!(locate(&[r"C:\", r"F:\"], &[]), Err(LocateError::NotFound));
    assert_eq!(locate(&[], &[]), Err(LocateError::NotFound));
}

#[test]
fn vol01_two_libraries_without_e_is_ambiguous() {
    assert_eq!(locate(&[r"F:\", r"G:\"], &[r"F:\", r"G:\"]), Err(LocateError::Ambiguous));
    assert_eq!(LocateError::NotFound.message(), "disco da biblioteca não encontrado");
    assert_eq!(LocateError::Ambiguous.as_code(), "library_disk_ambiguous");
}

// ---------------------------------------------------------------- VOL-02

#[test]
fn vol02_serial_and_drive_mask_are_formatted_like_windows() {
    assert_eq!(format_serial(0x1A2B_3C4D), "1A2B-3C4D");
    assert_eq!(format_serial(0x0000_00FF), "0000-00FF");
    assert_eq!(roots_from_mask(0b10100), [r"C:\", r"E:\"]);
    assert_eq!(roots_from_mask(1 << 25), [r"Z:\"]);
    assert!(roots_from_mask(0).is_empty());
}

#[cfg(windows)]
#[test]
fn vol02_real_volume_has_a_serial() {
    let system = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
    let info = volume_info(Path::new(&format!(r"{system}\"))).expect("system drive volume");
    assert_eq!(info.serial.len(), 9);
    assert_eq!(info.serial.as_bytes()[4], b'-');
}

#[cfg(not(windows))]
#[test]
fn vol02_no_volume_off_windows() {
    assert_eq!(volume_info(Path::new("/")), None);
}

// ---------------------------------------------------------------- VOL-03

#[test]
fn vol03_a_moved_disk_is_relinked_to_its_new_letter() {
    let connection = database();
    library(&connection, r"E:\", Some("AAAA-BBBB"));
    assert!(relink(&connection, r"F:\", Some(&disk("AAAA-BBBB"))).unwrap());
    assert_eq!(roots(&connection), [r"F:\ AAAA-BBBB -"]);
    // Already there: nothing to do.
    assert!(!relink(&connection, r"F:\", Some(&disk("AAAA-BBBB"))).unwrap());
}

#[test]
fn vol03_another_disk_or_an_unknown_volume_is_never_relinked() {
    let connection = database();
    library(&connection, r"E:\", Some("AAAA-BBBB"));
    assert!(!relink(&connection, r"F:\", Some(&disk("CCCC-DDDD"))).unwrap());
    assert!(!relink(&connection, r"F:\", None).unwrap());
    assert_eq!(roots(&connection), [r"E:\ AAAA-BBBB -"]);
}

#[test]
fn vol03_a_root_already_used_by_another_library_is_not_taken_over() {
    let connection = database();
    library(&connection, r"E:\", Some("AAAA-BBBB"));
    library(&connection, r"F:\", None);
    assert!(!relink(&connection, r"F:\", Some(&disk("AAAA-BBBB"))).unwrap());
    assert_eq!(roots(&connection), [r"E:\ AAAA-BBBB -", r"F:\ - -"]);
}

// ---------------------------------------------------------------- VOL-04

#[test]
fn vol04_the_run_records_serial_and_label() {
    let connection = database();
    let id = library(&connection, r"E:\", None);
    record_volume(&connection, id, Some(&disk("AAAA-BBBB"))).unwrap();
    assert_eq!(roots(&connection), [r"E:\ AAAA-BBBB Fotos"]);
    // Unknown volume (not Windows, or unreadable) keeps what was there.
    record_volume(&connection, id, None).unwrap();
    assert_eq!(roots(&connection), [r"E:\ AAAA-BBBB Fotos"]);
}
