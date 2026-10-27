//! F14: search over the library. Words go to the FTS5 index (`media_search`, kept in
//! sync by triggers from migration v3): file and folder names, title, description,
//! people, place and note. A year, a month name or "foto"/"vídeo" typed in the box
//! become date and type filters. Results come in timeline order (newest first).

#[cfg(test)]
mod tests;

use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection};
use serde::{Deserialize, Serialize};

/// Page size when the caller does not say.
pub const DEFAULT_LIMIT: u32 = 200;
const MAX_LIMIT: u32 = 2000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Photo,
    Video,
}

impl MediaKind {
    fn as_column(self) -> &'static str {
        match self {
            Self::Photo => "photo",
            Self::Video => "video",
        }
    }
}

/// What the search box and the filter chips (F22) send.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SearchQuery {
    /// Free text: words, plus a year, a month name or a media type.
    pub text: String,
    pub media_type: Option<MediaKind>,
    /// Inclusive, ISO date or date-time in UTC (`2022-08-01`).
    pub from: Option<String>,
    /// Exclusive, same format.
    pub until: Option<String>,
    pub person_id: Option<i64>,
    /// F22: every one of these people is in the media.
    pub people: Vec<i64>,
    /// F22: under this folder (relative path, `/` separated), subfolders included.
    pub folder: Option<String>,
    /// F24: in this place (`media_locations.place_label`, or part of it), accents and
    /// case ignored: "São Paulo, SP, Brasil", "lisbon".
    pub place: Option<String>,
    pub has_location: Option<bool>,
    pub favorites_only: bool,
    pub limit: Option<u32>,
    pub offset: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub id: i64,
    pub relative_path: String,
    pub media_type: String,
    pub taken_at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    /// Matches across every page.
    pub total: u64,
    pub hits: Vec<SearchHit>,
}

/// The free text split into what goes to FTS5 and what becomes a filter.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ParsedText {
    /// FTS5 MATCH expression; `None` when no word is left.
    pub fts: Option<String>,
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub media_type: Option<MediaKind>,
}

/// Full names only: abbreviations such as "mar" (sea) or "set" are ordinary words.
const MONTHS: [&[&str]; 12] = [
    &["janeiro", "january"],
    &["fevereiro", "february"],
    &["marco", "march"],
    &["abril", "april"],
    &["maio", "may"],
    &["junho", "june"],
    &["julho", "july"],
    &["agosto", "august"],
    &["setembro", "september"],
    &["outubro", "october"],
    &["novembro", "november"],
    &["dezembro", "december"],
];

/// Ignored in the text so "praia em 2022" still finds the beach.
const STOP_WORDS: &[&str] = &[
    "a", "o", "as", "os", "e", "de", "da", "do", "das", "dos", "em", "no", "na", "nos", "nas",
    "com", "para", "por", "of", "in", "the", "and",
];

/// Lower case without accents, for matching the words above.
fn fold(word: &str) -> String {
    word.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ô' | 'õ' | 'ò' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

pub(crate) fn parse_text(text: &str) -> ParsedText {
    let mut parsed = ParsedText::default();
    let mut terms = Vec::new();
    for word in text.split_whitespace() {
        let folded = fold(word.trim_matches(|c: char| c == ',' || c == ';'));
        if folded.is_empty() || STOP_WORDS.contains(&folded.as_str()) {
            continue;
        }
        if parsed.year.is_none() && folded.len() == 4 {
            if let Ok(year @ 1800..=2199) = folded.parse::<i32>() {
                parsed.year = Some(year);
                continue;
            }
        }
        if parsed.month.is_none() {
            if let Some(index) = MONTHS.iter().position(|names| names.contains(&folded.as_str())) {
                parsed.month = Some(index as u32 + 1);
                continue;
            }
        }
        if parsed.media_type.is_none() {
            let kind = match folded.as_str() {
                "foto" | "fotos" | "photo" | "photos" | "imagem" | "imagens" => Some(MediaKind::Photo),
                "video" | "videos" => Some(MediaKind::Video),
                _ => None,
            };
            if kind.is_some() {
                parsed.media_type = kind;
                continue;
            }
        }
        // A quoted prefix term: FTS5 syntax in the box ("-", ":", "*", "OR") is just text.
        terms.push(format!("\"{}\"*", word.replace('"', "\"\"")));
    }
    if !terms.is_empty() {
        parsed.fts = Some(terms.join(" "));
    }
    parsed
}

/// `2022-08` → `2022-09`; December rolls the year.
fn next_month(year: i32, month: u32) -> (i32, u32) {
    if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    }
}

/// The WHERE clause and its values. `with_type` off leaves the type out, for the
/// photo and video counts next to the type boxes (F22).
fn where_clause(query: &SearchQuery, with_type: bool) -> (String, Vec<Value>) {
    let parsed = parse_text(&query.text);
    let mut conditions = vec![
        "m.is_hidden = 0".to_string(),
        "m.indexing_status != 'missing_file'".to_string(),
    ];
    let mut values: Vec<Value> = Vec::new();
    let mut bind = |conditions: &mut Vec<String>, sql: &str, value: Value| {
        values.push(value);
        conditions.push(sql.replace('?', &format!("?{}", values.len())));
    };

    if let Some(fts) = &parsed.fts {
        bind(
            &mut conditions,
            "m.id IN (SELECT rowid FROM media_search WHERE media_search MATCH ?)",
            Value::Text(fts.clone()),
        );
    }
    if let Some(kind) = query.media_type.or(parsed.media_type).filter(|_| with_type) {
        bind(&mut conditions, "m.media_type = ?", Value::Text(kind.as_column().into()));
    }
    match (parsed.year, parsed.month) {
        (Some(year), Some(month)) => {
            let (next_year, next) = next_month(year, month);
            bind(&mut conditions, "m.taken_at >= ?", Value::Text(format!("{year:04}-{month:02}")));
            bind(&mut conditions, "m.taken_at < ?", Value::Text(format!("{next_year:04}-{next:02}")));
        }
        (Some(year), None) => {
            bind(&mut conditions, "m.taken_at >= ?", Value::Text(format!("{year:04}")));
            bind(&mut conditions, "m.taken_at < ?", Value::Text(format!("{:04}", year + 1)));
        }
        (None, Some(month)) => {
            bind(&mut conditions, "substr(m.taken_at, 6, 2) = ?", Value::Text(format!("{month:02}")));
        }
        (None, None) => {}
    }
    if let Some(from) = &query.from {
        bind(&mut conditions, "m.taken_at >= ?", Value::Text(from.clone()));
    }
    if let Some(until) = &query.until {
        bind(&mut conditions, "m.taken_at < ?", Value::Text(until.clone()));
    }
    if let Some(person) = query.person_id {
        bind(
            &mut conditions,
            "m.id IN (SELECT media_id FROM media_people WHERE person_id = ?)",
            Value::Integer(person),
        );
    }
    match query.has_location {
        Some(true) => conditions.push(
            "EXISTS (SELECT 1 FROM media_locations l WHERE l.media_id = m.id AND l.latitude IS NOT NULL)".into(),
        ),
        Some(false) => conditions.push(
            "NOT EXISTS (SELECT 1 FROM media_locations l WHERE l.media_id = m.id AND l.latitude IS NOT NULL)".into(),
        ),
        None => {}
    }
    for person in &query.people {
        bind(
            &mut conditions,
            "m.id IN (SELECT media_id FROM media_people WHERE person_id = ?)",
            Value::Integer(*person),
        );
    }
    if let Some(folder) = query.folder.as_deref().map(|folder| folder.trim_matches('/')).filter(|folder| !folder.is_empty()) {
        // Prefix compare, not LIKE: folder names carry `_` and `%`.
        bind(
            &mut conditions,
            "lower(substr(m.relative_path, 1, length(?))) = lower(?)",
            Value::Text(format!("{folder}/")),
        );
    }
    if let Some(place) = query.place.as_deref().map(str::trim).filter(|place| !place.is_empty()) {
        // One phrase in the place column: "Porto, Portugal" is not "Porto Alegre, RS, Brasil".
        bind(
            &mut conditions,
            "m.id IN (SELECT rowid FROM media_search WHERE media_search MATCH ?)",
            Value::Text(format!("place : \"{}\"", place.replace('"', "\"\""))),
        );
    }
    if query.favorites_only {
        conditions.push("m.is_favorite = 1".into());
    }
    (conditions.join(" AND "), values)
}

/// Media of every library in the database (there is one per disk, AD-011), hidden and
/// missing files left out.
pub fn search(connection: &Connection, query: &SearchQuery) -> rusqlite::Result<SearchPage> {
    let (filter, values) = where_clause(query, true);

    let total: i64 = connection.query_row(
        &format!("SELECT count(*) FROM media_items m WHERE {filter}"),
        params_from_iter(values.iter()),
        |row| row.get(0),
    )?;

    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let mut page_values = values.clone();
    page_values.push(Value::Integer(i64::from(limit)));
    page_values.push(Value::Integer(i64::from(query.offset)));
    let sql = format!(
        "SELECT m.id, m.relative_path, m.media_type, m.taken_at FROM media_items m
         WHERE {filter}
         ORDER BY m.taken_at IS NULL, m.taken_at DESC, m.id DESC
         LIMIT ?{} OFFSET ?{}",
        values.len() + 1,
        values.len() + 2
    );
    let hits = connection
        .prepare(&sql)?
        .query_map(params_from_iter(page_values.iter()), |row| {
            Ok(SearchHit {
                id: row.get(0)?,
                relative_path: row.get(1)?,
                media_type: row.get(2)?,
                taken_at: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(SearchPage {
        total: total.max(0) as u64,
        hits,
    })
}

/// F22: every match at once, as ids in timeline order; the screen already has the rest
/// of each item from the `timeline` command. With the photo and video counts.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchIds {
    pub ids: Vec<i64>,
    /// Matches of each type with every other filter applied.
    pub photos: u64,
    pub videos: u64,
}

pub fn search_ids(connection: &Connection, query: &SearchQuery) -> rusqlite::Result<SearchIds> {
    let (filter, values) = where_clause(query, true);
    let ids = connection
        .prepare(&format!(
            "SELECT m.id FROM media_items m WHERE {filter}
             ORDER BY m.taken_at IS NULL, m.taken_at DESC, m.id DESC"
        ))?
        .query_map(params_from_iter(values.iter()), |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<i64>>>()?;
    let (filter, values) = where_clause(query, false);
    let (photos, videos): (i64, i64) = connection.query_row(
        &format!(
            "SELECT coalesce(sum(m.media_type = 'photo'), 0), coalesce(sum(m.media_type = 'video'), 0)
             FROM media_items m WHERE {filter}"
        ),
        params_from_iter(values.iter()),
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    Ok(SearchIds {
        ids,
        photos: photos.max(0) as u64,
        videos: videos.max(0) as u64,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonOption {
    pub id: i64,
    pub name: String,
    pub count: u64,
}

/// What the filter column offers: people (most photos first) and folders with media.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchOptions {
    pub people: Vec<PersonOption>,
    /// Relative paths, `/` separated, in name order.
    pub folders: Vec<String>,
}

pub fn search_options(connection: &Connection) -> rusqlite::Result<SearchOptions> {
    let people = connection
        .prepare(
            "SELECT p.id, p.name, count(*) FROM people p
             JOIN media_people mp ON mp.person_id = p.id
             JOIN media_items m ON m.id = mp.media_id
             WHERE m.is_hidden = 0 AND m.indexing_status != 'missing_file'
             GROUP BY p.id ORDER BY count(*) DESC, p.normalized_name",
        )?
        .query_map([], |row| {
            Ok(PersonOption {
                id: row.get(0)?,
                name: row.get(1)?,
                count: row.get::<_, i64>(2)?.max(0) as u64,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let folders = connection
        .prepare(
            "SELECT f.relative_path FROM folders f
             WHERE f.relative_path != '' AND EXISTS (
               SELECT 1 FROM media_items m
               WHERE m.library_id = f.library_id AND m.indexing_status != 'missing_file'
                 AND lower(substr(m.relative_path, 1, length(f.relative_path) + 1)) = lower(f.relative_path || '/'))
             ORDER BY f.relative_path COLLATE NOCASE",
        )?
        .query_map([], |row| row.get(0))?
        .collect::<rusqlite::Result<Vec<String>>>()?;
    Ok(SearchOptions { people, folders })
}

/// Rebuilds the index from the tables (after a restore or if it ever drifts).
pub fn rebuild(connection: &Connection) -> rusqlite::Result<()> {
    connection.execute_batch(
        "DELETE FROM media_search;
         INSERT INTO media_search (rowid, path, title, description, people, place, note)
         SELECT m.id, m.relative_path, m.title, m.description,
                (SELECT group_concat(p.name, ' ') FROM media_people mp JOIN people p ON p.id = mp.person_id
                  WHERE mp.media_id = m.id),
                (SELECT l.place_label FROM media_locations l WHERE l.media_id = m.id),
                (SELECT n.note FROM user_notes n WHERE n.media_id = m.id)
         FROM media_items m;
         INSERT INTO media_search (media_search) VALUES ('optimize');",
    )
}
