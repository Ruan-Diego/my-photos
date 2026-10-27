//! F24: names for coordinates, offline. The nearest town of 5,000 people or more
//! (GeoNames, CC BY 4.0, embedded from `data/places.tsv`) within `MAX_KM` names a
//! photo's place: "São Paulo, SP, Brasil", "Lisbon, Portugal". The label goes to
//! `media_locations.place_label`, which the search index (F14) reads by trigger.

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::OnceLock;

use rusqlite::{params, Connection};

const PLACES: &str = include_str!("../../data/places.tsv");
const COUNTRIES: &str = include_str!("../../data/countries.tsv");

/// Farther than this from any town, a photo has coordinates but no place name.
pub const MAX_KM: f64 = 50.0;
/// A bigger town this much farther than the nearest one still wins: a photo in
/// Times Square is in New York City, not in the Hoboken across the river.
const BIG_TOWN_SLACK_KM: f64 = 4.0;

/// GeoNames admin1 code → state abbreviation, as people write it in Brazil.
const BR_STATES: [(&str, &str); 27] = [
    ("01", "AC"), ("02", "AL"), ("03", "AP"), ("04", "AM"), ("05", "BA"), ("06", "CE"),
    ("07", "DF"), ("08", "ES"), ("11", "MS"), ("13", "MA"), ("14", "MT"), ("15", "MG"),
    ("16", "PA"), ("17", "PB"), ("18", "PR"), ("20", "PI"), ("21", "RJ"), ("22", "RN"),
    ("23", "RS"), ("24", "RO"), ("25", "RR"), ("26", "SC"), ("27", "SP"), ("28", "SE"),
    ("29", "GO"), ("30", "PE"), ("31", "TO"),
];

#[derive(Debug, Clone, PartialEq)]
pub struct Town {
    pub name: &'static str,
    pub country: &'static str,
    pub admin: &'static str,
    pub latitude: f64,
    pub longitude: f64,
}

struct Gazetteer {
    towns: Vec<Town>,
    /// 1° × 1° cells → indexes into `towns`.
    cells: HashMap<(i32, i32), Vec<usize>>,
    countries: HashMap<&'static str, &'static str>,
}

fn cell(latitude: f64, longitude: f64) -> (i32, i32) {
    (latitude.floor() as i32, longitude.floor() as i32)
}

fn gazetteer() -> &'static Gazetteer {
    static GAZETTEER: OnceLock<Gazetteer> = OnceLock::new();
    GAZETTEER.get_or_init(|| {
        let towns: Vec<Town> = PLACES
            .lines()
            .filter(|line| !line.starts_with('#') && !line.is_empty())
            .filter_map(|line| {
                let mut fields = line.split('\t');
                let name = fields.next()?;
                let country = fields.next()?;
                let admin = fields.next()?;
                let latitude = fields.next()?.parse().ok()?;
                let longitude = fields.next()?.parse().ok()?;
                Some(Town { name, country, admin, latitude, longitude })
            })
            .collect();
        let mut cells: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (index, town) in towns.iter().enumerate() {
            cells.entry(cell(town.latitude, town.longitude)).or_default().push(index);
        }
        let countries = COUNTRIES
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .collect();
        Gazetteer { towns, cells, countries }
    })
}

/// Great-circle distance in km.
pub fn distance_km(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (lat1, lon1) = (a.0.to_radians(), a.1.to_radians());
    let (lat2, lon2) = (b.0.to_radians(), b.1.to_radians());
    let h = ((lat2 - lat1) / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * ((lon2 - lon1) / 2.0).sin().powi(2);
    2.0 * 6371.0 * h.sqrt().min(1.0).asin()
}

/// The town of a photo: the biggest of those about as near as the nearest one, within
/// `MAX_KM`; `None` at sea or far from any town.
pub fn nearest(latitude: f64, longitude: f64) -> Option<&'static Town> {
    if !latitude.is_finite() || !longitude.is_finite() || latitude.abs() > 90.0 || longitude.abs() > 180.0 {
        return None;
    }
    let gazetteer = gazetteer();
    let (row, column) = cell(latitude, longitude);
    // 50 km is under one cell in latitude; near the poles a degree of longitude is
    // shorter, so look two cells to each side there.
    let reach = if latitude.abs() > 60.0 { 2 } else { 1 };
    let mut near: Vec<(f64, usize)> = Vec::new();
    for dr in -1..=1 {
        for dc in -reach..=reach {
            let wrapped = (column + dc + 180).rem_euclid(360) - 180;
            for &index in gazetteer.cells.get(&(row + dr, wrapped)).into_iter().flatten() {
                let town = &gazetteer.towns[index];
                let km = distance_km((latitude, longitude), (town.latitude, town.longitude));
                if km <= MAX_KM {
                    near.push((km, index));
                }
            }
        }
    }
    let closest = near.iter().map(|(km, _)| *km).fold(f64::INFINITY, f64::min);
    // `towns` is sorted by population, biggest first.
    near.iter()
        .filter(|(km, _)| *km <= closest + BIG_TOWN_SLACK_KM)
        .map(|(_, index)| *index)
        .min()
        .map(|index| &gazetteer.towns[index])
}

/// "São Paulo, SP, Brasil", "Lisbon, OH, Estados Unidos", "Lisbon, Portugal".
pub fn label_of(town: &Town) -> String {
    let country = gazetteer().countries.get(town.country).copied().unwrap_or(town.country);
    let state = match town.country {
        "BR" => BR_STATES.iter().find(|(code, _)| *code == town.admin).map(|(_, uf)| *uf),
        "US" if !town.admin.is_empty() => Some(town.admin),
        _ => None,
    };
    match state {
        Some(state) => format!("{}, {state}, {country}", town.name),
        None => format!("{}, {country}", town.name),
    }
}

pub fn label(latitude: f64, longitude: f64) -> Option<String> {
    nearest(latitude, longitude).map(label_of)
}

/// Names every located media that has no place yet (F10 rows from before F24, or
/// added by an older app). Returns how many got a name. The search index follows by
/// trigger.
pub fn fill_place_labels(connection: &mut Connection) -> rusqlite::Result<usize> {
    let pending: Vec<(i64, f64, f64)> = connection
        .prepare(
            "SELECT media_id, latitude, longitude FROM media_locations
             WHERE place_label IS NULL AND latitude IS NOT NULL AND longitude IS NOT NULL",
        )?
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    let transaction = connection.transaction()?;
    let mut named = 0;
    {
        let mut update = transaction.prepare("UPDATE media_locations SET place_label = ?2 WHERE media_id = ?1")?;
        for (media_id, latitude, longitude) in pending {
            if let Some(place) = label(latitude, longitude) {
                update.execute(params![media_id, place])?;
                named += 1;
            }
        }
    }
    transaction.commit()?;
    Ok(named)
}
