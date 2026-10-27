//! F32: how long each screen's query takes on a synthetic 300k-item library.
//!
//! `cargo test --lib perf -- --ignored --nocapture` prints one line per query. It is
//! ignored by default: building the library takes a while and the times are for people.

use std::path::Path;
use std::time::Instant;

use rusqlite::{params, Connection};

use crate::db::MIGRATIONS;

pub const ITEMS: i64 = 300_000;
const FOLDERS: i64 = 600;
const PEOPLE: i64 = 400;
const PLACES: [&str; 8] = [
    "São Paulo, SP, Brasil",
    "Rio de Janeiro, RJ, Brasil",
    "Lisbon, Portugal",
    "Porto, Portugal",
    "Campinas, SP, Brasil",
    "Florianópolis, SC, Brasil",
    "Paris, França",
    "Osasco, SP, Brasil",
];

/// A file-backed library shaped like a big Takeout: 600 folders, 400 people, a third
/// of the items with a place, one in twelve a video, one in fifty a favorite.
pub fn build(path: &Path, items: i64) -> Connection {
    let _ = std::fs::remove_file(path);
    let mut connection = Connection::open(path).unwrap();
    connection.pragma_update(None, "journal_mode", "WAL").unwrap();
    connection.pragma_update(None, "foreign_keys", "ON").unwrap();
    for migration in MIGRATIONS {
        connection.execute_batch(migration.sql).unwrap();
    }
    let transaction = connection.transaction().unwrap();
    transaction
        .execute("INSERT INTO libraries (name, root_path, created_at) VALUES ('E', 'E:\\', 't')", [])
        .unwrap();
    for folder in 0..FOLDERS {
        transaction
            .execute(
                "INSERT INTO folders (library_id, relative_path, created_at) VALUES (1, ?1, 't')",
                [format!("takeout-{}/Takeout/Google Fotos/Album {folder:03}", folder % 6)],
            )
            .unwrap();
    }
    for export in 0..6 {
        for path in [format!("takeout-{export}"), format!("takeout-{export}/Takeout"), format!("takeout-{export}/Takeout/Google Fotos")] {
            transaction
                .execute("INSERT OR IGNORE INTO folders (library_id, relative_path, created_at) VALUES (1, ?1, 't')", [path])
                .unwrap();
        }
    }
    for person in 0..PEOPLE {
        transaction
            .execute(
                "INSERT INTO people (library_id, name, normalized_name) VALUES (1, ?1, ?2)",
                params![format!("Pessoa {person}"), format!("pessoa {person}")],
            )
            .unwrap();
    }
    {
        let mut media = transaction
            .prepare(
                "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, size_bytes,
                   file_hash_sha256, perceptual_hash, width, height, taken_at, taken_at_source, has_takeout_json,
                   json_relative_path, json_match_confidence, is_favorite, created_at, updated_at)
                 VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, 4032, 3024, ?8, 'takeout_json', 1, ?9, 'exact', ?10, 't', 't')",
            )
            .unwrap();
        let mut place = transaction
            .prepare("INSERT INTO media_locations (media_id, latitude, longitude, source, place_label) VALUES (?1, ?2, ?3, 'takeout_geoData', ?4)")
            .unwrap();
        let mut tag = transaction.prepare("INSERT INTO media_people (media_id, person_id) VALUES (?1, ?2)").unwrap();
        let start = 1_700_000_000i64;
        for index in 0..items {
            let folder = format!("takeout-{}/Takeout/Google Fotos/Album {:03}", (index % FOLDERS) % 6, index % FOLDERS);
            let video = index % 12 == 5;
            let extension = if video { "mp4" } else { "jpg" };
            let filename = format!("IMG_{index:07}.{extension}");
            let path = format!("{folder}/{filename}");
            let taken = chrono_like(start - index * 3_000);
            // One in 40 shares its bytes with the item before it: identical copies.
            let sha = format!("{:064x}", if index % 40 == 1 { index - 1 } else { index });
            let phash = (!video).then(|| format!("{:016x}", (index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)));
            media
                .execute(params![
                    path,
                    filename,
                    extension,
                    if video { "video" } else { "photo" },
                    3_000_000 + index % 1000,
                    sha,
                    phash,
                    taken,
                    format!("{path}.supplemental-metadata.json"),
                    (index % 50 == 0) as i64
                ])
                .unwrap();
            let id = transaction.last_insert_rowid();
            if index % 3 == 0 {
                let label = PLACES[(index as usize / 3) % PLACES.len()];
                place.execute(params![id, -23.5 + (index % 100) as f64 / 100.0, -46.6, label]).unwrap();
            }
            if index % 4 == 0 {
                tag.execute(params![id, 1 + (index * 7) % PEOPLE]).unwrap();
            }
        }
    }
    transaction.commit().unwrap();
    connection
}

/// `2023-11-14T22:13:20Z` from Unix seconds, without a date crate.
fn chrono_like(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    // Civil-from-days (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// Runs `work` three times and returns the best time in ms.
pub fn best<T>(work: impl Fn() -> T) -> u128 {
    (0..3)
        .map(|_| {
            let start = Instant::now();
            std::hint::black_box(work());
            start.elapsed().as_millis()
        })
        .min()
        .unwrap()
}

#[cfg(test)]
mod tests;
