//! F28: the problems screen (artboard `Problems`): what the indexing could not
//! recognize, the numbers of the last indexing and a health check of what the app
//! depends on. Read only, except the report file under `.myphotos` (AD-011).

#[cfg(test)]
mod tests;

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::index::export_part;
use crate::thumbs::{cache_bytes, cache_limit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// The app worked around it, but something may be missing or wrong.
    Pending,
    /// The app found what it needed another way.
    Resolved,
    /// Nothing to show; listed only so the user knows.
    Ignored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    /// Relative to the library root, `/`-separated.
    pub path: String,
    pub reason: String,
    /// What the app did about it.
    pub action: String,
    pub status: Status,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub media: u64,
    pub date_from_json: u64,
    pub date_from_exif: u64,
    pub date_from_file: u64,
    pub json_in_other_part: u64,
    /// Seconds the last completed indexing took.
    pub last_duration_seconds: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub database_path: Option<String>,
    pub root_path: Option<String>,
    pub thumbnail_bytes: u64,
    pub thumbnail_limit_bytes: u64,
    pub ffmpeg: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub problems: Vec<Problem>,
    pub report: Report,
    pub diagnostics: Diagnostics,
}

const VISIBLE: &str = "is_hidden = 0 AND indexing_status != 'missing_file'";

fn problem(path: String, reason: &str, action: &str, status: Status) -> Problem {
    Problem { path, reason: reason.into(), action: action.into(), status }
}

/// What the scan and the Takeout matcher found (only the last finished run: they look
/// at every file each time), plus what stays wrong with indexed media.
pub fn problems(connection: &Connection) -> rusqlite::Result<Vec<Problem>> {
    let mut found = Vec::new();
    let last_run: Option<i64> = connection
        .query_row("SELECT max(id) FROM indexing_runs WHERE status != 'running'", [], |row| row.get(0))
        .optional()?
        .flatten();
    if let Some(run) = last_run {
        let mut statement = connection.prepare(
            "SELECT DISTINCT relative_path, error_type FROM indexing_errors
             WHERE run_id = ?1 AND error_type NOT IN ('corrupt_media', 'unreadable')",
        )?;
        let rows = statement.query_map([run], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
        for row in rows {
            let (path, kind) = row?;
            found.push(match kind.as_str() {
                "invalid_json" => problem(path, "JSON corrompido", "A mídia usou a data do EXIF ou do arquivo", Status::Pending),
                "orphan_json" => problem(path, "JSON sem mídia correspondente", "Nada a mostrar", Status::Ignored),
                "unreadable_directory" => problem(path, "Pasta ilegível", "A pasta foi pulada", Status::Pending),
                "invalid_name" => problem(path, "Nome inválido no Windows", "Ficou fora da biblioteca", Status::Pending),
                _ => problem(path, "Extensão desconhecida", "Ficou fora da biblioteca", Status::Ignored),
            });
        }
    }

    let media = |sql: &str| -> rusqlite::Result<Vec<(String, String, String)>> {
        connection
            .prepare(sql)?
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect()
    };
    for (path, _, _) in media(&format!(
        "SELECT relative_path, '', '' FROM media_items WHERE {VISIBLE} AND indexing_status = 'error'"
    ))? {
        found.push(problem(path, "Arquivo ilegível", "Aparece na biblioteca, sem conferir o conteúdo", Status::Pending));
    }
    // EXIF corrupt: recorded when the item was last read, so any run counts.
    for (path, _, _) in media(&format!(
        "SELECT m.relative_path, '', '' FROM media_items m
         WHERE m.is_hidden = 0 AND m.indexing_status != 'missing_file' AND m.relative_path IN (SELECT relative_path FROM indexing_errors WHERE error_type = 'corrupt_media')"
    ))? {
        found.push(problem(path, "EXIF corrompido", "Usou a data do JSON ou do arquivo", Status::Pending));
    }
    for (path, reason, kind) in media(&format!(
        "SELECT m.relative_path, f.reason, m.media_type FROM thumbnail_failures f JOIN media_items m ON m.id = f.media_id
         WHERE m.is_hidden = 0 AND m.indexing_status != 'missing_file'"
    ))? {
        let extension = path.rsplit_once('.').map(|(_, extension)| extension.to_uppercase()).unwrap_or_default();
        found.push(match (reason.as_str(), kind.as_str()) {
            (_, "video") => problem(path, "Falha ao gerar a capa do vídeo", "Mostra um ícone no lugar da miniatura", Status::Pending),
            ("unsupported_format", _) => problem(
                path,
                &format!("Formato sem miniatura (.{extension})"),
                "Mostra um ícone no lugar da miniatura",
                Status::Pending,
            ),
            _ => problem(path, "Imagem corrompida", "Mostra um ícone no lugar da miniatura", Status::Pending),
        });
    }
    for (path, _, _) in media(&format!(
        "SELECT relative_path, '', '' FROM media_items WHERE {VISIBLE} AND taken_at_source = 'filesystem'"
    ))? {
        found.push(problem(path, "Sem data no JSON nem no EXIF", "Usou a data do arquivo", Status::Pending));
    }
    for (path, _, _) in media(&format!(
        "SELECT relative_path, '', '' FROM media_items WHERE {VISIBLE} AND json_match_confidence = 'heuristic'"
    ))? {
        found.push(problem(path, "Nome cortado pelo Takeout", "Pareado por heurística", Status::Resolved));
    }

    // Pending first; then by path. One row per (path, reason).
    let mut seen = HashSet::new();
    found.retain(|row| seen.insert((row.path.to_lowercase(), row.reason.clone())));
    found.sort_by(|a, b| {
        let rank = |status: Status| status as u8;
        rank(a.status).cmp(&rank(b.status)).then_with(|| a.path.to_lowercase().cmp(&b.path.to_lowercase()))
    });
    Ok(found)
}

pub fn report(connection: &Connection) -> rusqlite::Result<Report> {
    let (media, json, exif, file): (i64, i64, i64, i64) = connection.query_row(
        &format!(
            "SELECT count(*), coalesce(sum(taken_at_source = 'takeout_json'), 0),
                    coalesce(sum(taken_at_source = 'exif'), 0), coalesce(sum(taken_at_source = 'filesystem'), 0)
             FROM media_items WHERE {VISIBLE}"
        ),
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;
    let mut other_part = 0;
    let mut statement = connection.prepare(&format!(
        "SELECT relative_path, json_relative_path FROM media_items WHERE {VISIBLE} AND json_relative_path IS NOT NULL"
    ))?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?;
    for row in rows {
        let (media_path, json_path) = row?;
        let media_part = export_part(&media_path).map(str::to_ascii_lowercase);
        let json_part = export_part(&json_path).map(str::to_ascii_lowercase);
        if media_part.is_some() && json_part.is_some() && media_part != json_part {
            other_part += 1;
        }
    }
    let last_duration_seconds: Option<f64> = connection
        .query_row(
            "SELECT (julianday(finished_at) - julianday(started_at)) * 86400 FROM indexing_runs
             WHERE status = 'completed' AND finished_at IS NOT NULL ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?
        .flatten();
    let count = |value: i64| value.max(0) as u64;
    Ok(Report {
        media: count(media),
        date_from_json: count(json),
        date_from_exif: count(exif),
        date_from_file: count(file),
        json_in_other_part: other_part,
        last_duration_seconds: last_duration_seconds.map(|seconds| seconds.max(0.0).round() as u64),
    })
}

pub fn diagnostics(connection: &Connection, ffmpeg: bool) -> rusqlite::Result<Diagnostics> {
    let root_path: Option<String> = connection
        .query_row("SELECT root_path FROM libraries ORDER BY id LIMIT 1", [], |row| row.get(0))
        .optional()?;
    Ok(Diagnostics {
        database_path: connection.path().filter(|path| !path.is_empty()).map(str::to_string),
        root_path,
        thumbnail_bytes: cache_bytes(connection)?,
        thumbnail_limit_bytes: cache_limit(connection)?,
        ffmpeg,
    })
}

pub fn overview(connection: &Connection, ffmpeg: bool) -> rusqlite::Result<Overview> {
    Ok(Overview {
        problems: problems(connection)?,
        report: report(connection)?,
        diagnostics: diagnostics(connection, ffmpeg)?,
    })
}

/// `PRAGMA integrity_check`: `None` when the database is sound, else its first complaint.
pub fn integrity(connection: &Connection) -> rusqlite::Result<Option<String>> {
    let first: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    Ok((first != "ok").then_some(first))
}

fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', ';']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// The problems as CSV (UTF-8 with BOM, so Excel reads the accents).
pub fn report_csv(problems: &[Problem]) -> String {
    let mut csv = String::from("\u{feff}Caminho,Motivo,O que o app fez,Status\r\n");
    for row in problems {
        let status = match row.status {
            Status::Pending => "Pendente",
            Status::Resolved => "Resolvido",
            Status::Ignored => "Ignorado",
        };
        csv.push_str(&format!(
            "{},{},{},{status}\r\n",
            csv_field(&row.path),
            csv_field(&row.reason),
            csv_field(&row.action)
        ));
    }
    csv
}
