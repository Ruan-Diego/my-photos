// Reachable from F09/F10; nothing in the app calls it yet.
#![allow(dead_code)]

//! F07: metadata embedded in a photo (EXIF + image header). Reads only; never writes.

mod date;

#[cfg(test)]
mod tests;

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use exif::{Exif, In, Reader, Tag, Value};
use imagesize::ImageType;

use date::{
    gps_offset, gps_utc_seconds, offset_text, parse_exif_datetime, parse_offset, parse_subsec,
    to_utc,
};

/// Bytes enough for `imagesize::image_type` to recognise any supported format.
const SNIFF_BYTES: u64 = 256;

/// Longest text kept per tag in `raw_json`, in characters.
const RAW_VALUE_CHARS: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateTag {
    Original,
    Digitized,
}

impl DateTag {
    pub fn as_tag(self) -> &'static str {
        match self {
            Self::Original => "DateTimeOriginal",
            Self::Digitized => "DateTimeDigitized",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EmbeddedMetadata {
    /// Wall-clock capture time with no zone: `2022-08-10T11:37:00.500`.
    pub taken_at_local: Option<String>,
    /// `-03:00`, only when the zone is known (OffsetTime* or GPS).
    pub taken_at_offset: Option<String>,
    /// UTC `2022-08-10T14:37:00.500Z`, only when the zone is known.
    pub taken_at: Option<String>,
    pub taken_at_tag: Option<DateTag>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    /// As stored, before applying `orientation`.
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub orientation: Option<u16>,
    /// `{ "TagName": "readable value" }` of the primary IFD, without MakerNote.
    pub raw_json: Option<String>,
    /// An EXIF block exists but could not be read (fully or partly).
    pub exif_corrupt: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddedError {
    Unreadable,
}

impl EmbeddedError {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::Unreadable => "unreadable",
        }
    }
}

pub fn read_embedded(path: &Path) -> Result<EmbeddedMetadata, EmbeddedError> {
    let file = File::open(path).map_err(|_| EmbeddedError::Unreadable)?;
    let mut metadata = EmbeddedMetadata::default();
    let (exif, corrupt) = read_exif(file)?;
    metadata.exif_corrupt = corrupt;

    if header_size_is_stored_size(path) {
        if let Ok(size) = imagesize::size(path) {
            if let (Some(width), Some(height)) = (positive(size.width), positive(size.height)) {
                metadata.width = Some(width);
                metadata.height = Some(height);
            }
        }
    }

    if let Some(exif) = &exif {
        fill_from_exif(&mut metadata, exif);
    }
    Ok(metadata)
}

/// `(exif, corrupt)`: no EXIF → `(None, false)`; unreadable EXIF or a truncated file →
/// `(None, true)`; partly readable → `(Some, true)`. A disk error is `Unreadable`.
fn read_exif(file: File) -> Result<(Option<Exif>, bool), EmbeddedError> {
    let mut reader = BufReader::new(file);
    Ok(
        match Reader::new()
            .continue_on_error(true)
            .read_from_container(&mut reader)
        {
            Ok(exif) => (Some(exif), false),
            Err(exif::Error::Io(error)) if error.kind() != std::io::ErrorKind::UnexpectedEof => {
                return Err(EmbeddedError::Unreadable);
            }
            // No EXIF block, or a container kamadak-exif does not read (GIF): its
            // read_from_container reports that as this exact InvalidFormat.
            Err(exif::Error::NotFound(_) | exif::Error::InvalidFormat("Unknown image format")) => {
                (None, false)
            }
            Err(exif::Error::PartialResult(partial)) => {
                let (exif, _) = partial.into_inner();
                let usable = exif.fields().len() > 0;
                (usable.then_some(exif), true)
            }
            Err(_) => (None, true),
        },
    )
}

/// Decided by content, not by name: only JPEG, PNG, GIF and WebP headers hold the size as
/// stored. HEIF headers are skipped because `imagesize` applies their `irot` rotation
/// (iPhone photos would come back turned), and TIFF/RAW because the first IFD is usually
/// the preview; both use the EXIF pixel dimensions instead.
fn header_size_is_stored_size(path: &Path) -> bool {
    let mut head = Vec::new();
    let read = File::open(path).and_then(|file| file.take(SNIFF_BYTES).read_to_end(&mut head));
    if read.is_err() {
        return false;
    }
    matches!(
        imagesize::image_type(&head),
        Ok(ImageType::Jpeg | ImageType::Png | ImageType::Gif | ImageType::Webp)
    )
}

fn positive(size: usize) -> Option<u32> {
    u32::try_from(size).ok().filter(|size| *size > 0)
}

fn fill_from_exif(metadata: &mut EmbeddedMetadata, exif: &Exif) {
    fill_date(metadata, exif);
    if let Some((latitude, longitude, altitude)) = read_gps(exif) {
        metadata.latitude = Some(latitude);
        metadata.longitude = Some(longitude);
        metadata.altitude = altitude;
    }
    metadata.orientation = uint(exif, Tag::Orientation)
        .filter(|value| (1..=8).contains(value))
        .map(|value| value as u16);
    if metadata.width.is_none() || metadata.height.is_none() {
        if let (Some(width), Some(height)) = (
            uint(exif, Tag::PixelXDimension).filter(|width| *width > 0),
            uint(exif, Tag::PixelYDimension).filter(|height| *height > 0),
        ) {
            metadata.width = Some(width);
            metadata.height = Some(height);
        }
    }
    metadata.raw_json = Some(raw_json(exif));
}

fn fill_date(metadata: &mut EmbeddedMetadata, exif: &Exif) {
    let candidates = [
        (
            DateTag::Original,
            Tag::DateTimeOriginal,
            Tag::SubSecTimeOriginal,
            Tag::OffsetTimeOriginal,
        ),
        (
            DateTag::Digitized,
            Tag::DateTimeDigitized,
            Tag::SubSecTimeDigitized,
            Tag::OffsetTimeDigitized,
        ),
    ];
    let Some((date_tag, local, subsec_tag, offset_tag)) =
        candidates
            .into_iter()
            .find_map(|(date_tag, tag, subsec, offset)| {
                let local = ascii(exif, tag).and_then(parse_exif_datetime)?;
                Some((date_tag, local, subsec, offset))
            })
    else {
        return;
    };
    let millis = ascii(exif, subsec_tag).map_or(0, parse_subsec);
    metadata.taken_at_tag = Some(date_tag);
    metadata.taken_at_local = Some(local.local_text(millis));

    let offset = ascii(exif, offset_tag).and_then(parse_offset).or_else(|| {
        let date = ascii(exif, Tag::GPSDateStamp)?;
        let time = rationals(exif, Tag::GPSTimeStamp)?;
        let [hour, minute, second] = <[f64; 3]>::try_from(time).ok()?;
        gps_offset(local, gps_utc_seconds(date, hour, minute, second)?)
    });
    if let Some(offset) = offset {
        if let Some(utc) = to_utc(local, millis, offset) {
            metadata.taken_at_offset = Some(offset_text(offset));
            metadata.taken_at = Some(utc);
        }
    }
}

/// Validated point (same rules as the Takeout sidecar) and optional altitude.
fn read_gps(exif: &Exif) -> Option<(f64, f64, Option<f64>)> {
    let latitude = coordinate(exif, Tag::GPSLatitude, Tag::GPSLatitudeRef, b'N', b'S')?;
    let longitude = coordinate(exif, Tag::GPSLongitude, Tag::GPSLongitudeRef, b'E', b'W')?;
    let inside = (-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude);
    if !inside || (latitude == 0.0 && longitude == 0.0) {
        return None;
    }
    let altitude = rationals(exif, Tag::GPSAltitude)
        .and_then(|values| values.first().copied())
        .map(|meters| match uint(exif, Tag::GPSAltitudeRef) {
            Some(1) => -meters,
            _ => meters,
        });
    Some((latitude, longitude, altitude))
}

/// Degrees/minutes/seconds + a required N/S or E/W reference.
fn coordinate(exif: &Exif, tag: Tag, reference: Tag, positive: u8, negative: u8) -> Option<f64> {
    let [degrees, minutes, seconds] = <[f64; 3]>::try_from(rationals(exif, tag)?).ok()?;
    let value = degrees + minutes / 60.0 + seconds / 3600.0;
    match ascii(exif, reference)?.first()? {
        byte if *byte == positive => Some(value),
        byte if *byte == negative => Some(-value),
        _ => None,
    }
}

fn field(exif: &Exif, tag: Tag) -> Option<&exif::Field> {
    exif.get_field(tag, In::PRIMARY)
}

fn ascii(exif: &Exif, tag: Tag) -> Option<&[u8]> {
    match &field(exif, tag)?.value {
        Value::Ascii(values) => values.first().map(Vec::as_slice),
        _ => None,
    }
}

/// Unsigned rationals as numbers; any zero denominator makes the whole tag unusable.
fn rationals(exif: &Exif, tag: Tag) -> Option<Vec<f64>> {
    match &field(exif, tag)?.value {
        Value::Rational(values) => values
            .iter()
            .map(|value| (value.denom != 0).then(|| value.to_f64()))
            .collect(),
        _ => None,
    }
}

fn uint(exif: &Exif, tag: Tag) -> Option<u32> {
    field(exif, tag)?.value.get_uint(0)
}

fn raw_json(exif: &Exif) -> String {
    let mut map = serde_json::Map::new();
    for field in exif.fields() {
        if field.ifd_num != In::PRIMARY || field.tag == Tag::MakerNote {
            continue;
        }
        let text = field.display_value().with_unit(exif).to_string();
        let text: String = text.chars().take(RAW_VALUE_CHARS).collect();
        map.insert(field.tag.to_string(), serde_json::Value::String(text));
    }
    serde_json::Value::Object(map).to_string()
}
