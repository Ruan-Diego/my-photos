// Reachable from F10; nothing in the app calls it yet.
#![allow(dead_code)]

//! F09: picks each media field from the Takeout JSON (F06), the embedded EXIF (F07)
//! or the filesystem (F05), in that order, and says where it came from. Pure: no I/O.

#[cfg(test)]
mod tests;

use crate::photo_meta::EmbeddedMetadata;
use crate::takeout::MediaMetadata;
use crate::time::{days_from_civil, iso_from_unix_millis};

/// Brasília, minutes east of UTC. Used for an EXIF time with no zone (AD-012).
pub const DEFAULT_FALLBACK_OFFSET_MINUTES: i32 = -180;

/// The values of `media_items.taken_at_source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateSource {
    TakeoutJson,
    Exif,
    Filesystem,
}

impl DateSource {
    pub fn as_column(self) -> &'static str {
        match self {
            Self::TakeoutJson => "takeout_json",
            Self::Exif => "exif",
            Self::Filesystem => "filesystem",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Location {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: Option<f64>,
    /// The value of `media_locations.source`.
    pub source: &'static str,
}

/// What F10 writes to `media_items` and `media_locations` for one file.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ResolvedMetadata {
    /// UTC `2022-08-10T14:37:00.000Z`.
    pub taken_at: Option<String>,
    pub taken_at_source: Option<DateSource>,
    /// `media_items.taken_at_timezone`: `-03:00` only when the file itself said so.
    pub taken_at_timezone: Option<String>,
    /// The EXIF time had no zone and `fallback_offset_minutes` was applied.
    pub taken_at_offset_assumed: bool,
    pub location: Option<Location>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub orientation: Option<u16>,
    pub title: Option<String>,
    pub description: Option<String>,
}

/// Filesystem times as F05 stores them (`fs_created_at`, `fs_modified_at`).
#[derive(Debug, Clone, Copy, Default)]
pub struct FsTimes<'a> {
    pub created_at: Option<&'a str>,
    pub modified_at: Option<&'a str>,
}

pub fn resolve(
    json: Option<&MediaMetadata>,
    exif: Option<&EmbeddedMetadata>,
    fs: FsTimes<'_>,
    fallback_offset_minutes: i32,
) -> ResolvedMetadata {
    let mut resolved = ResolvedMetadata::default();

    if let Some(taken_at) = json.and_then(|json| json.taken_at.clone()) {
        resolved.taken_at = Some(taken_at);
        resolved.taken_at_source = Some(DateSource::TakeoutJson);
    } else if let Some(exif) = exif.filter(|exif| exif.taken_at_local.is_some()) {
        if let Some(taken_at) = exif.taken_at.clone() {
            resolved.taken_at = Some(taken_at);
            resolved.taken_at_timezone = exif.taken_at_offset.clone();
        } else {
            let local = exif.taken_at_local.as_deref().unwrap_or_default();
            resolved.taken_at = local_to_utc(local, fallback_offset_minutes);
            resolved.taken_at_offset_assumed = resolved.taken_at.is_some();
        }
        if resolved.taken_at.is_some() {
            resolved.taken_at_source = Some(DateSource::Exif);
        }
    }
    if resolved.taken_at.is_none() {
        // A copy resets the creation time but keeps the modification time, so the
        // earlier of the two is the closest to the capture.
        let earliest = [fs.created_at, fs.modified_at]
            .into_iter()
            .flatten()
            .filter(|time| !time.is_empty())
            .min();
        if let Some(earliest) = earliest {
            resolved.taken_at = Some(earliest.to_string());
            resolved.taken_at_source = Some(DateSource::Filesystem);
        }
    }

    resolved.location = json
        .and_then(json_location)
        .or_else(|| exif.and_then(exif_location));

    if let Some(exif) = exif {
        resolved.width = exif.width;
        resolved.height = exif.height;
        resolved.orientation = exif.orientation;
    }
    if let Some(json) = json {
        resolved.title = json.title.clone();
        resolved.description = json.description.clone();
    }
    resolved
}

fn json_location(json: &MediaMetadata) -> Option<Location> {
    Some(Location {
        latitude: json.latitude?,
        longitude: json.longitude?,
        altitude: json.altitude,
        source: json.location_source?.as_column(),
    })
}

fn exif_location(exif: &EmbeddedMetadata) -> Option<Location> {
    Some(Location {
        latitude: exif.latitude?,
        longitude: exif.longitude?,
        altitude: exif.altitude,
        source: "exif",
    })
}

/// `2022-08-10T11:37:00.500` at `offset_minutes` east of UTC → `2022-08-10T14:37:00.500Z`.
fn local_to_utc(local: &str, offset_minutes: i32) -> Option<String> {
    let bytes = local.as_bytes();
    if bytes.len() != 23
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'.'
    {
        return None;
    }
    let number = |range: std::ops::Range<usize>| -> Option<i64> {
        let part = &local[range];
        part.bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| part.parse().ok())
            .flatten()
    };
    let (year, month, day) = (number(0..4)?, number(5..7)?, number(8..10)?);
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let millis = number(20..23)?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let days = days_from_civil(year, month as u32, day as u32);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second
        - i64::from(offset_minutes) * 60;
    iso_from_unix_millis(seconds, millis as u32)
}
