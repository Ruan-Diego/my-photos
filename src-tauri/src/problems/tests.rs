use rusqlite::{params, Connection};

use super::{integrity, problems, report, report_csv, Problem, Status};
use crate::db::MIGRATIONS;

const PART1: &str = "takeout-20260921T183943Z-1-001/Takeout/Google Fotos";
const PART2: &str = "takeout-20260921T183943Z-2-001/Takeout/Google Fotos";

fn database() -> Connection {
    let connection = Connection::open_in_memory().unwrap();
    for migration in MIGRATIONS {
        connection.execute_batch(migration.sql).unwrap();
    }
    connection
        .execute("INSERT INTO libraries (name, root_path, created_at) VALUES ('E', 'E:\\', 't')", [])
        .unwrap();
    connection
}

fn media(connection: &Connection, path: &str, kind: &str, source: &str, json: Option<&str>, confidence: &str, status: &str) -> i64 {
    connection
        .execute(
            "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, taken_at_source,
               json_relative_path, json_match_confidence, indexing_status, created_at, updated_at)
             VALUES (1, ?1, ?1, 'jpg', ?2, ?3, ?4, ?5, ?6, 't', 't')",
            params![path, kind, source, json, confidence, status],
        )
        .unwrap();
    connection.last_insert_rowid()
}

fn run(connection: &Connection, status: &str, started: &str, finished: &str, errors: &[(&str, &str)]) {
    connection
        .execute(
            "INSERT INTO indexing_runs (library_id, started_at, finished_at, run_type, status) VALUES (1, ?1, ?2, 'full', ?3)",
            params![started, finished, status],
        )
        .unwrap();
    let id = connection.last_insert_rowid();
    for (path, kind) in errors {
        connection
            .execute(
                "INSERT INTO indexing_errors (run_id, relative_path, error_type, created_at) VALUES (?1, ?2, ?3, 't')",
                params![id, path, kind],
            )
            .unwrap();
    }
}

fn row<'a>(rows: &'a [Problem], path: &str) -> Vec<(&'a str, Status)> {
    rows.iter().filter(|row| row.path == path).map(|row| (row.reason.as_str(), row.status)).collect()
}

#[test]
fn prb01_problems_come_from_the_last_run_and_from_the_media() {
    let connection = database();
    // An older run's JSON problem was fixed since; the last run's ones are listed.
    run(&connection, "completed", "2026-10-01T10:00:00Z", "2026-10-01T10:05:00Z", &[("velho.json", "invalid_json"), ("a.jpg", "corrupt_media")]);
    run(
        &connection,
        "completed",
        "2026-10-26T10:00:00Z",
        "2026-10-26T10:06:41Z",
        &[("foto.json", "invalid_json"), ("lonely.json", "orphan_json"), ("bloqueada", "unreadable_directory"), ("x.zip", "unknown_extension")],
    );
    run(&connection, "running", "2026-10-27T10:00:00Z", "", &[("novo.json", "invalid_json")]);
    media(&connection, "a.jpg", "photo", "takeout_json", None, "exact", "ok");
    let heic = media(&connection, "b.heic", "photo", "exif", None, "none", "ok");
    let video = media(&connection, "VID.mp4", "video", "takeout_json", None, "exact", "ok");
    media(&connection, "sem-data.jpg", "photo", "filesystem", None, "none", "ok");
    media(&connection, "cortado.jpg", "photo", "takeout_json", Some("cortado..json"), "heuristic", "ok");
    media(&connection, "ruim.jpg", "photo", "filesystem", None, "none", "error");
    media(&connection, "sumiu.jpg", "photo", "filesystem", None, "none", "missing_file");
    for (id, reason) in [(heic, "unsupported_format"), (video, "corrupt_media")] {
        connection
            .execute(
                "INSERT INTO thumbnail_failures (media_id, source_fingerprint, reason, created_at) VALUES (?1, 'f', ?2, 't')",
                params![id, reason],
            )
            .unwrap();
    }

    let rows = problems(&connection).unwrap();
    assert_eq!(row(&rows, "foto.json"), [("JSON corrompido", Status::Pending)]);
    assert!(row(&rows, "velho.json").is_empty(), "older runs do not count");
    assert!(row(&rows, "novo.json").is_empty(), "the running one neither");
    assert_eq!(row(&rows, "lonely.json"), [("JSON sem mídia correspondente", Status::Ignored)]);
    assert_eq!(row(&rows, "bloqueada"), [("Pasta ilegível", Status::Pending)]);
    assert_eq!(row(&rows, "x.zip"), [("Extensão desconhecida", Status::Ignored)]);
    assert_eq!(row(&rows, "a.jpg"), [("EXIF corrompido", Status::Pending)], "corrupt EXIF from any run");
    assert_eq!(row(&rows, "b.heic"), [("Formato sem miniatura (.HEIC)", Status::Pending)]);
    assert_eq!(row(&rows, "VID.mp4"), [("Falha ao gerar a capa do vídeo", Status::Pending)]);
    assert_eq!(row(&rows, "sem-data.jpg"), [("Sem data no JSON nem no EXIF", Status::Pending)]);
    assert_eq!(row(&rows, "cortado.jpg"), [("Nome cortado pelo Takeout", Status::Resolved)]);
    assert_eq!(
        row(&rows, "ruim.jpg"),
        [("Arquivo ilegível", Status::Pending), ("Sem data no JSON nem no EXIF", Status::Pending)]
    );
    assert!(row(&rows, "sumiu.jpg").is_empty());
    let statuses: Vec<Status> = rows.iter().map(|row| row.status).collect();
    let mut sorted = statuses.clone();
    sorted.sort_by_key(|status| *status as u8);
    assert_eq!(statuses, sorted, "pending, then resolved, then ignored");
}

#[test]
fn prb02_report_counts_dates_parts_and_duration() {
    let connection = database();
    run(&connection, "completed", "2026-10-26T10:00:00Z", "2026-10-26T10:06:41Z", &[]);
    run(&connection, "cancelled", "2026-10-27T10:00:00Z", "2026-10-27T10:00:05Z", &[]);
    media(&connection, &format!("{PART1}/a.jpg"), "photo", "takeout_json", Some(&format!("{PART2}/a.jpg.json")), "exact", "ok");
    media(&connection, &format!("{PART1}/b.jpg"), "photo", "takeout_json", Some(&format!("{PART1}/b.jpg.json")), "exact", "ok");
    media(&connection, "Ruan/c.jpg", "photo", "exif", None, "none", "ok");
    media(&connection, "Ruan/d.jpg", "photo", "filesystem", None, "none", "ok");
    media(&connection, "Ruan/e.jpg", "photo", "filesystem", None, "none", "missing_file");
    let found = report(&connection).unwrap();
    assert_eq!(
        (found.media, found.date_from_json, found.date_from_exif, found.date_from_file, found.json_in_other_part),
        (4, 2, 1, 1, 1)
    );
    assert_eq!(found.last_duration_seconds, Some(401), "6 min 41 s, from the last completed run");
    assert_eq!(integrity(&connection).unwrap(), None);
}

#[test]
fn prb03_csv_quotes_and_names_the_status() {
    let rows = vec![Problem {
        path: "Fotos, 2022/a \"b\".json".into(),
        reason: "JSON corrompido".into(),
        action: "Nada".into(),
        status: Status::Ignored,
    }];
    assert_eq!(
        report_csv(&rows),
        "\u{feff}Caminho,Motivo,O que o app fez,Status\r\n\"Fotos, 2022/a \"\"b\"\".json\",JSON corrompido,Nada,Ignorado\r\n"
    );
}
