//! IDX-01..06 on a temp tree and a temp v1 database. Never `E:\`. EXIF is faked by file
//! name so the tests do not depend on image encoders.

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use super::{index_with, IndexSummary, Progress, Sources, Stage};
use crate::db::tests::Scratch;
use crate::db::MIGRATIONS;
use crate::photo_meta::{EmbeddedError, EmbeddedMetadata};
use crate::scan::{absolute_path, RealFs};
use crate::takeout::discover;

const T1: &str = "2026-10-14T10:00:00.000Z";
const T2: &str = "2026-10-14T11:00:00.000Z";
const T3: &str = "2026-10-14T12:00:00.000Z";

const PHOTOS: &str = "Takeout/Google Fotos/Fotos de 2022";

struct Library {
    _scratch: Scratch,
    root: PathBuf,
    connection: Connection,
    exif_reads: RefCell<Vec<String>>,
}

impl Library {
    fn new(label: &str) -> Self {
        let scratch = Scratch::new(&format!("f10-{label}"));
        let root = scratch.path.join("root");
        fs::create_dir_all(&root).unwrap();
        let connection = Connection::open(scratch.path.join("library.db")).unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        for migration in MIGRATIONS {
            connection.execute_batch(migration.sql).unwrap();
        }
        Self {
            _scratch: scratch,
            root,
            connection,
            exif_reads: RefCell::new(Vec::new()),
        }
    }

    fn write(&self, relative: &str, bytes: &[u8]) {
        let path = absolute_path(&self.root, relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn index(&mut self, now: &str) -> (IndexSummary, Vec<Progress>) {
        let mut seen = Vec::new();
        let reads = &self.exif_reads;
        let read_exif = |path: &Path| -> Result<EmbeddedMetadata, EmbeddedError> {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            reads.borrow_mut().push(name.clone());
            fake_exif(&name)
        };
        let sources = Sources {
            fs: &RealFs,
            discover: &discover,
            read_exif: &read_exif,
        };
        let summary = index_with(&mut self.connection, &self.root, &sources, now, &mut |p| {
            seen.push(p)
        })
        .expect("index");
        (summary, seen)
    }

    fn row(&self, relative: &str) -> Row {
        self.connection
            .query_row(
                "SELECT taken_at, taken_at_source, taken_at_timezone, has_takeout_json, json_relative_path,
                        json_match_confidence, width, height, orientation, title, description
                 FROM media_items WHERE relative_path = ?1",
                [relative],
                |row| {
                    Ok(Row {
                        taken_at: row.get(0)?,
                        source: row.get(1)?,
                        timezone: row.get(2)?,
                        has_json: row.get(3)?,
                        json: row.get(4)?,
                        confidence: row.get(5)?,
                        width: row.get(6)?,
                        height: row.get(7)?,
                        orientation: row.get(8)?,
                        title: row.get(9)?,
                        description: row.get(10)?,
                    })
                },
            )
            .unwrap()
    }

    fn strings(&self, sql: &str) -> Vec<String> {
        let mut statement = self.connection.prepare(sql).unwrap();
        let rows = statement.query_map([], |row| row.get(0)).unwrap();
        rows.map(Result::unwrap).collect()
    }

    fn errors(&self, run_id: i64) -> Vec<String> {
        self.strings(&format!(
            "SELECT error_type||' '||relative_path FROM indexing_errors WHERE run_id = {run_id} ORDER BY relative_path, error_type"
        ))
    }

    fn reads(&self) -> Vec<String> {
        std::mem::take(&mut *self.exif_reads.borrow_mut())
    }
}

#[derive(Debug, PartialEq)]
struct Row {
    taken_at: Option<String>,
    source: Option<String>,
    timezone: Option<String>,
    has_json: bool,
    json: Option<String>,
    confidence: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    orientation: Option<u16>,
    title: Option<String>,
    description: Option<String>,
}

fn fake_exif(name: &str) -> Result<EmbeddedMetadata, EmbeddedError> {
    match name {
        "IMG_local.jpg" => Ok(EmbeddedMetadata {
            taken_at_local: Some("2021-12-31T22:30:00.000".to_string()),
            latitude: Some(-22.9),
            longitude: Some(-43.2),
            width: Some(4000),
            height: Some(3000),
            orientation: Some(6),
            raw_json: Some(r#"{"Model":"X"}"#.to_string()),
            ..EmbeddedMetadata::default()
        }),
        "broken.jpg" => Ok(EmbeddedMetadata {
            exif_corrupt: true,
            width: Some(10),
            height: Some(20),
            ..EmbeddedMetadata::default()
        }),
        "gone.jpg" => Err(EmbeddedError::Unreadable),
        _ => Ok(EmbeddedMetadata {
            width: Some(800),
            height: Some(600),
            ..EmbeddedMetadata::default()
        }),
    }
}

fn sidecar(taken: i64, people: &[&str]) -> Vec<u8> {
    let people: Vec<String> = people
        .iter()
        .map(|name| format!(r#"{{ "name": "{name}" }}"#))
        .collect();
    format!(
        r#"{{ "title": "foto", "description": "praia",
             "photoTakenTime": {{ "timestamp": "{taken}" }},
             "creationTime": {{ "timestamp": "1700000000" }},
             "geoData": {{ "latitude": -23.5505, "longitude": -46.6333, "altitude": 760.0 }},
             "people": [{}] }}"#,
        people.join(",")
    )
    .into_bytes()
}

fn takeout_library(label: &str) -> Library {
    let library = Library::new(label);
    library.write(&format!("{PHOTOS}/a.jpg"), b"a");
    library.write(
        &format!("{PHOTOS}/a.jpg.supplemental-metadata.json"),
        &sidecar(1660142220, &["Ana", "Bruno"]),
    );
    library.write(&format!("{PHOTOS}/b.jpg"), b"b");
    library.write(&format!("{PHOTOS}/b.jpg.json"), &sidecar(1660142280, &["ana "]));
    library.write("Ruan/Photos-1-001/IMG_local.jpg", b"c");
    library.write("Ruan/Clipes/video.mp4", b"d");
    library
}

// ---------------------------------------------------------------- IDX-01

#[test]
fn idx01_first_run_writes_every_source_to_the_database() {
    let mut library = takeout_library("first");
    let (summary, progress) = library.index(T1);

    assert_eq!(summary.scan.media, 4);
    assert_eq!(summary.scan.hashed, 4);
    assert_eq!(summary.resolved, 4);
    assert_eq!(
        (summary.date_from_json, summary.date_from_exif, summary.date_from_filesystem, summary.no_date),
        (2, 1, 1, 0)
    );
    assert_eq!(summary.date_offset_assumed, 1);
    assert_eq!(summary.with_json, 2);
    assert_eq!(summary.with_location, 3);

    let a = library.row(&format!("{PHOTOS}/a.jpg"));
    assert_eq!(
        a,
        Row {
            taken_at: Some("2022-08-10T14:37:00.000Z".to_string()),
            source: Some("takeout_json".to_string()),
            timezone: None,
            has_json: true,
            json: Some(format!("{PHOTOS}/a.jpg.supplemental-metadata.json")),
            confidence: Some("exact".to_string()),
            width: Some(800),
            height: Some(600),
            orientation: None,
            title: Some("foto".to_string()),
            description: Some("praia".to_string()),
        }
    );

    let local = library.row("Ruan/Photos-1-001/IMG_local.jpg");
    assert_eq!(local.taken_at.as_deref(), Some("2022-01-01T01:30:00.000Z"));
    assert_eq!(local.source.as_deref(), Some("exif"));
    assert_eq!(local.timezone, None);
    assert!(!local.has_json);
    assert_eq!(local.json, None);
    assert_eq!(local.confidence.as_deref(), Some("none"));
    assert_eq!((local.width, local.height, local.orientation), (Some(4000), Some(3000), Some(6)));

    let video = library.row("Ruan/Clipes/video.mp4");
    assert_eq!(video.source.as_deref(), Some("filesystem"));
    assert!(video.taken_at.is_some());
    assert_eq!((video.width, video.height), (None, None));

    // Only photos are handed to the EXIF reader.
    let mut reads = library.reads();
    reads.sort();
    assert_eq!(reads, ["IMG_local.jpg", "a.jpg", "b.jpg"]);

    assert_eq!(
        library.strings(
            "SELECT m.filename||' '||l.source||' '||l.latitude FROM media_locations l
             JOIN media_items m ON m.id = l.media_id ORDER BY m.filename"
        ),
        ["IMG_local.jpg exif -22.9", "a.jpg takeout_geoData -23.5505", "b.jpg takeout_geoData -23.5505"]
    );
    assert_eq!(
        library.strings(
            "SELECT m.filename||' '||(r.takeout_json IS NOT NULL)||' '||coalesce(r.exif_json, '-')
             FROM media_metadata_raw r JOIN media_items m ON m.id = r.media_id ORDER BY m.filename"
        ),
        ["IMG_local.jpg 0 {\"Model\":\"X\"}", "a.jpg 1 -", "b.jpg 1 -"]
    );

    // "ana " and "Ana" are one person; the first spelling seen names them.
    assert_eq!(
        library.strings("SELECT name FROM people ORDER BY normalized_name"),
        ["Ana", "Bruno"]
    );
    assert_eq!(
        library.strings(
            "SELECT m.filename||' '||p.name FROM media_people mp JOIN media_items m ON m.id = mp.media_id
             JOIN people p ON p.id = mp.person_id ORDER BY m.filename, p.name"
        ),
        ["a.jpg Ana", "a.jpg Bruno", "b.jpg Ana"]
    );

    let run: (String, String, i64, i64) = library
        .connection
        .query_row(
            "SELECT run_type, status, files_scanned, files_indexed FROM indexing_runs WHERE id = ?1",
            [summary.scan.run_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(run, ("full".to_string(), "completed".to_string(), 4, 4));

    let stages: Vec<Stage> = progress.iter().map(|p| p.stage).collect();
    assert_eq!(
        stages,
        [Stage::Scanning, Stage::Matching, Stage::Resolving, Stage::Resolving]
    );
    assert_eq!(progress.last().map(|p| (p.done, p.total)), Some((4, 4)));
}

// ---------------------------------------------------------------- IDX-02

#[test]
fn idx02_second_run_without_changes_reads_nothing() {
    let mut library = takeout_library("again");
    library.index(T1);
    library.reads();
    let before = library.strings("SELECT relative_path||' '||updated_at FROM media_items ORDER BY relative_path");

    let (summary, _) = library.index(T2);
    assert_eq!(summary.scan.run_type, "incremental");
    assert_eq!(summary.scan.unchanged, 4);
    assert_eq!(summary.resolved, 0);
    assert!(library.reads().is_empty());
    assert_eq!(
        library.strings("SELECT relative_path||' '||updated_at FROM media_items ORDER BY relative_path"),
        before
    );
}

#[test]
fn idx02_a_sidecar_that_appears_later_re_resolves_only_its_media() {
    let mut library = takeout_library("late-json");
    library.index(T1);
    library.reads();

    library.write("Ruan/Photos-1-001/IMG_local.jpg.json", &sidecar(1500000000, &[]));
    let (summary, _) = library.index(T2);
    assert_eq!(summary.resolved, 1);
    assert_eq!(library.reads(), ["IMG_local.jpg"]);
    let local = library.row("Ruan/Photos-1-001/IMG_local.jpg");
    assert_eq!(local.source.as_deref(), Some("takeout_json"));
    assert_eq!(local.taken_at.as_deref(), Some("2017-07-14T02:40:00.000Z"));
    assert_eq!(local.confidence.as_deref(), Some("exact"));
    // Dimensions still come from the file.
    assert_eq!(local.width, Some(4000));

    // And when it goes away the media falls back to its EXIF date.
    fs::remove_file(absolute_path(&library.root, "Ruan/Photos-1-001/IMG_local.jpg.json")).unwrap();
    let (summary, _) = library.index(T3);
    assert_eq!(summary.resolved, 1);
    let local = library.row("Ruan/Photos-1-001/IMG_local.jpg");
    assert_eq!(local.source.as_deref(), Some("exif"));
    assert!(!local.has_json);
    assert_eq!(
        library.strings(
            "SELECT CAST(count(*) AS TEXT) FROM media_metadata_raw r JOIN media_items m ON m.id = r.media_id
             WHERE m.filename = 'IMG_local.jpg' AND r.takeout_json IS NOT NULL"
        ),
        ["0"]
    );
}

#[test]
fn idx02_a_changed_file_is_read_again() {
    let mut library = takeout_library("changed");
    library.index(T1);
    library.reads();

    library.write("Ruan/Photos-1-001/IMG_local.jpg", b"longer content");
    let (summary, _) = library.index(T2);
    assert_eq!(summary.scan.modified, 1);
    assert_eq!(summary.resolved, 1);
    assert_eq!(library.reads(), ["IMG_local.jpg"]);
}

// ---------------------------------------------------------------- IDX-03

#[test]
fn idx03_a_rows_people_location_and_raw_json_are_replaced_not_appended() {
    let mut library = takeout_library("replace");
    library.index(T1);
    library.write(&format!("{PHOTOS}/a.jpg"), b"edited a");
    library.write(
        &format!("{PHOTOS}/a.jpg.supplemental-metadata.json"),
        &sidecar(1660142220, &["Carla"]),
    );
    library.index(T2);
    assert_eq!(
        library.strings(
            "SELECT p.name FROM media_people mp JOIN media_items m ON m.id = mp.media_id
             JOIN people p ON p.id = mp.person_id WHERE m.filename = 'a.jpg'"
        ),
        ["Carla"]
    );
    assert_eq!(
        library.strings("SELECT CAST(count(*) AS TEXT) FROM media_locations"),
        ["3"]
    );
    assert_eq!(
        library.strings("SELECT CAST(count(*) AS TEXT) FROM media_metadata_raw"),
        ["3"]
    );
}

// ---------------------------------------------------------------- IDX-04

#[test]
fn idx04_sidecar_problems_are_recorded_but_not_inside_the_app_folder() {
    let mut library = takeout_library("problems");
    library.write(&format!("{PHOTOS}/lonely.jpg.supplemental-metadata.json"), b"{}");
    library.write(&format!("{PHOTOS}/c.jpg"), b"c");
    library.write(&format!("{PHOTOS}/c.jpg.json"), b"{ not json");
    library.write(".myphotos/trash/x.jpg.json", b"{}");

    let (summary, _) = library.index(T1);
    assert_eq!(
        library.errors(summary.scan.run_id),
        [
            format!("invalid_json {PHOTOS}/c.jpg.json"),
            format!("orphan_json {PHOTOS}/lonely.jpg.supplemental-metadata.json"),
        ]
    );
    // The media of an invalid sidecar is still indexed, as having no JSON.
    let c = library.row(&format!("{PHOTOS}/c.jpg"));
    assert!(!c.has_json);
    assert_eq!(c.confidence.as_deref(), Some("none"));
    assert_eq!(c.source.as_deref(), Some("filesystem"));
}

// ---------------------------------------------------------------- IDX-05

#[test]
fn idx05_corrupt_or_unreadable_exif_never_stops_the_run() {
    let mut library = Library::new("corrupt");
    library.write("broken.jpg", b"x");
    library.write("gone.jpg", b"y");
    library.write("fine.jpg", b"z");

    let (summary, _) = library.index(T1);
    assert_eq!(summary.resolved, 3);
    assert_eq!(library.errors(summary.scan.run_id), ["corrupt_media broken.jpg"]);
    let broken = library.row("broken.jpg");
    assert_eq!(broken.source.as_deref(), Some("filesystem"));
    assert_eq!((broken.width, broken.height), (Some(10), Some(20)));
    let gone = library.row("gone.jpg");
    assert_eq!(gone.source.as_deref(), Some("filesystem"));
    assert_eq!(gone.width, None);
}

// ---------------------------------------------------------------- IDX-06

#[test]
fn idx06_missing_media_is_left_alone() {
    let mut library = takeout_library("missing");
    library.index(T1);
    fs::remove_file(absolute_path(&library.root, "Ruan/Clipes/video.mp4")).unwrap();
    let (summary, _) = library.index(T2);
    assert_eq!(summary.scan.missing, 1);
    assert_eq!(summary.resolved, 0);
    let video = library.row("Ruan/Clipes/video.mp4");
    assert_eq!(video.source.as_deref(), Some("filesystem"));
}
