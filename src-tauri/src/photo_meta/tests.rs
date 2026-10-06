//! EXIF-01..06 on JPEG/PNG/GIF files generated here. Never `E:\`, no external tools.

use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

use exif::experimental::Writer;
use exif::{Field, In, Rational, Tag, Value};

use super::{read_embedded, DateTag, EmbeddedError, EmbeddedMetadata};
use crate::db::tests::Scratch;

fn ascii(tag: Tag, text: &str) -> Field {
    Field {
        tag,
        ifd_num: In::PRIMARY,
        value: Value::Ascii(vec![text.as_bytes().to_vec()]),
    }
}

fn rational(tag: Tag, values: &[(u32, u32)]) -> Field {
    Field {
        tag,
        ifd_num: In::PRIMARY,
        value: Value::Rational(
            values
                .iter()
                .map(|&(num, denom)| Rational { num, denom })
                .collect(),
        ),
    }
}

fn short(tag: Tag, value: u16) -> Field {
    Field {
        tag,
        ifd_num: In::PRIMARY,
        value: Value::Short(vec![value]),
    }
}

fn byte(tag: Tag, value: u8) -> Field {
    Field {
        tag,
        ifd_num: In::PRIMARY,
        value: Value::Byte(vec![value]),
    }
}

fn tiff(fields: &[Field]) -> Vec<u8> {
    let mut writer = Writer::new();
    for field in fields {
        writer.push_field(field);
    }
    let mut out = Cursor::new(Vec::new());
    writer.write(&mut out, false).expect("tiff");
    out.into_inner()
}

/// SOI, optional APP1 Exif, optional SOF0 with the size, EOI.
fn jpeg(exif_tiff: Option<&[u8]>, size: Option<(u16, u16)>) -> Vec<u8> {
    let mut bytes = vec![0xFF, 0xD8];
    if let Some(tiff) = exif_tiff {
        let length = (2 + 6 + tiff.len()) as u16;
        bytes.extend_from_slice(&[0xFF, 0xE1]);
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend_from_slice(b"Exif\0\0");
        bytes.extend_from_slice(tiff);
    }
    if let Some((width, height)) = size {
        bytes.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x11, 0x08]);
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&[0x03, 0x01, 0x22, 0x00, 0x02, 0x11, 0x01, 0x03, 0x11, 0x01]);
    }
    bytes.extend_from_slice(&[0xFF, 0xD9]);
    bytes
}

/// ISO-BMFF box: big-endian size, type, payload.
fn bmff(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut out = ((8 + payload.len()) as u32).to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    out
}

/// A HEIF stored as `width`x`height` with a 90° `irot`, no EXIF.
fn heif_rotated(width: u32, height: u32) -> Vec<u8> {
    let mut ispe = vec![0, 0, 0, 0];
    ispe.extend_from_slice(&width.to_be_bytes());
    ispe.extend_from_slice(&height.to_be_bytes());
    let mut ipco = bmff(b"ispe", &ispe);
    ipco.extend_from_slice(&bmff(b"irot", &[1]));
    let iprp = bmff(b"iprp", &bmff(b"ipco", &ipco));
    let mut meta = vec![0, 0, 0, 0];
    meta.extend_from_slice(&iprp);
    let mut bytes = bmff(b"ftyp", b"heic\0\0\0\0mif1heic");
    bytes.extend_from_slice(&bmff(b"meta", &meta));
    bytes
}

/// VP8X WebP with the canvas size and an optional EXIF chunk.
fn webp(exif_tiff: Option<&[u8]>, width: u32, height: u32) -> Vec<u8> {
    let mut chunks = b"VP8X".to_vec();
    chunks.extend_from_slice(&10u32.to_le_bytes());
    chunks.extend_from_slice(&[if exif_tiff.is_some() { 0x08 } else { 0 }, 0, 0, 0]);
    chunks.extend_from_slice(&(width - 1).to_le_bytes()[..3]);
    chunks.extend_from_slice(&(height - 1).to_le_bytes()[..3]);
    if let Some(tiff) = exif_tiff {
        chunks.extend_from_slice(b"EXIF");
        chunks.extend_from_slice(&(tiff.len() as u32).to_le_bytes());
        chunks.extend_from_slice(tiff);
        if tiff.len() % 2 == 1 {
            chunks.push(0);
        }
    }
    let mut bytes = b"RIFF".to_vec();
    bytes.extend_from_slice(&((4 + chunks.len()) as u32).to_le_bytes());
    bytes.extend_from_slice(b"WEBP");
    bytes.extend_from_slice(&chunks);
    bytes
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    let mut checked = kind.to_vec();
    checked.extend_from_slice(data);
    out.extend_from_slice(&checked);
    out.extend_from_slice(&crc32(&checked).to_be_bytes());
}

fn png(exif_tiff: Option<&[u8]>, width: u32, height: u32) -> Vec<u8> {
    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 2, 0, 0, 0]);
    png_chunk(&mut bytes, b"IHDR", &header);
    if let Some(tiff) = exif_tiff {
        png_chunk(&mut bytes, b"eXIf", tiff);
    }
    png_chunk(&mut bytes, b"IEND", &[]);
    bytes
}

struct Photos {
    scratch: Scratch,
}

impl Photos {
    fn new(label: &str) -> Self {
        Self {
            scratch: Scratch::new(&format!("f07-{label}")),
        }
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.scratch.path.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    fn read(&self, name: &str, bytes: &[u8]) -> EmbeddedMetadata {
        read_embedded(&self.write(name, bytes)).expect("readable")
    }

    fn jpeg_with(&self, fields: &[Field]) -> EmbeddedMetadata {
        self.read("foto.jpg", &jpeg(Some(&tiff(fields)), Some((640, 480))))
    }
}

const ORIGINAL: &str = "2022:08:10 11:37:00";

fn gps_time(fields: &mut Vec<Field>, date: &str, (hour, minute, second): (u32, u32, u32)) {
    fields.push(ascii(Tag::GPSDateStamp, date));
    fields.push(rational(
        Tag::GPSTimeStamp,
        &[(hour, 1), (minute, 1), (second, 1)],
    ));
}

// ---------------------------------------------------------------- EXIF-01, EXIF-02

#[test]
fn exif01_original_date_with_subsec_and_offset_becomes_utc() {
    let photos = Photos::new("date-offset");
    let metadata = photos.jpeg_with(&[
        ascii(Tag::DateTimeOriginal, ORIGINAL),
        ascii(Tag::SubSecTimeOriginal, "5"),
        ascii(Tag::OffsetTimeOriginal, "-03:00"),
    ]);
    assert_eq!(
        metadata.taken_at_local.as_deref(),
        Some("2022-08-10T11:37:00.500")
    );
    assert_eq!(metadata.taken_at_offset.as_deref(), Some("-03:00"));
    assert_eq!(
        metadata.taken_at.as_deref(),
        Some("2022-08-10T14:37:00.500Z")
    );
    assert_eq!(metadata.taken_at_tag, Some(DateTag::Original));
    assert_eq!(DateTag::Original.as_tag(), "DateTimeOriginal");
    assert_eq!(DateTag::Digitized.as_tag(), "DateTimeDigitized");
    assert!(!metadata.exif_corrupt);
}

#[test]
fn exif02_offset_comes_from_gps_when_offset_time_is_missing() {
    let photos = Photos::new("date-gps");
    let mut fields = vec![ascii(Tag::DateTimeOriginal, ORIGINAL)];
    gps_time(&mut fields, "2022:08:10", (14, 35, 10));
    let metadata = photos.jpeg_with(&fields);
    assert_eq!(metadata.taken_at_offset.as_deref(), Some("-03:00"));
    assert_eq!(
        metadata.taken_at.as_deref(),
        Some("2022-08-10T14:37:00.000Z")
    );

    // GPS 14:30 away from the wall clock: zone unknown.
    let mut far = vec![ascii(Tag::DateTimeOriginal, "2022:08:11 05:10:00")];
    gps_time(&mut far, "2022:08:10", (14, 35, 10));
    let metadata = photos.jpeg_with(&far);
    assert_eq!(
        metadata.taken_at_local.as_deref(),
        Some("2022-08-11T05:10:00.000")
    );
    assert_eq!((metadata.taken_at_offset, metadata.taken_at), (None, None));
}

#[test]
fn exif01_without_any_zone_only_the_local_time_is_set() {
    let photos = Photos::new("date-local");
    let metadata = photos.jpeg_with(&[ascii(Tag::DateTimeOriginal, ORIGINAL)]);
    assert_eq!(
        metadata.taken_at_local.as_deref(),
        Some("2022-08-10T11:37:00.000")
    );
    assert_eq!((metadata.taken_at_offset, metadata.taken_at), (None, None));
    assert_eq!(metadata.taken_at_tag, Some(DateTag::Original));
}

#[test]
fn exif01_digitized_is_the_fallback_and_datetime_is_never_used() {
    let photos = Photos::new("date-digitized");
    for original in [None, Some("0000:00:00 00:00:00"), Some("lixo")] {
        let mut fields = vec![
            ascii(Tag::DateTimeDigitized, "2021:12:31 22:00:00"),
            ascii(Tag::SubSecTimeDigitized, "25"),
            ascii(Tag::OffsetTimeDigitized, "+01:00"),
            ascii(Tag::SubSecTimeOriginal, "9"),
            ascii(Tag::OffsetTimeOriginal, "-05:00"),
        ];
        if let Some(original) = original {
            fields.push(ascii(Tag::DateTimeOriginal, original));
        }
        let metadata = photos.jpeg_with(&fields);
        assert_eq!(
            metadata.taken_at_tag,
            Some(DateTag::Digitized),
            "{original:?}"
        );
        assert_eq!(
            metadata.taken_at_local.as_deref(),
            Some("2021-12-31T22:00:00.250")
        );
        assert_eq!(metadata.taken_at_offset.as_deref(), Some("+01:00"));
        assert_eq!(
            metadata.taken_at.as_deref(),
            Some("2021-12-31T21:00:00.250Z")
        );
    }
    let only_datetime = photos.jpeg_with(&[ascii(Tag::DateTime, ORIGINAL)]);
    assert_eq!(
        (only_datetime.taken_at_local, only_datetime.taken_at_tag),
        (None, None)
    );
}

#[test]
fn exif01_invalid_dates_are_absent_and_dash_dates_are_accepted() {
    let photos = Photos::new("date-formats");
    for bad in [
        "0000:00:00 00:00:00",
        "2022:02:30 10:00:00",
        "2022:08:10 24:00:00",
        "1799:12:31 10:00:00",
        "ontem",
    ] {
        let metadata = photos.jpeg_with(&[ascii(Tag::DateTimeOriginal, bad)]);
        assert_eq!(metadata.taken_at_local, None, "{bad}");
        assert_eq!(metadata.taken_at_tag, None, "{bad}");
    }
    let dashed = photos.jpeg_with(&[ascii(Tag::DateTimeOriginal, "2022-08-10 11:37:00")]);
    assert_eq!(
        dashed.taken_at_local.as_deref(),
        Some("2022-08-10T11:37:00.000")
    );
}

#[test]
fn exif02_invalid_offset_time_falls_back_to_gps() {
    let photos = Photos::new("date-bad-offset");
    for offset in ["+15:00", "3:00", "  "] {
        let mut fields = vec![
            ascii(Tag::DateTimeOriginal, ORIGINAL),
            ascii(Tag::OffsetTimeOriginal, offset),
        ];
        gps_time(&mut fields, "2022:08:10", (14, 35, 10));
        let metadata = photos.jpeg_with(&fields);
        assert_eq!(
            metadata.taken_at_offset.as_deref(),
            Some("-03:00"),
            "{offset:?}"
        );
        assert_eq!(
            metadata.taken_at.as_deref(),
            Some("2022-08-10T14:37:00.000Z")
        );
    }
}

#[test]
fn edge_long_subsec_and_day_crossing() {
    let photos = Photos::new("date-edges");
    let metadata = photos.jpeg_with(&[
        ascii(Tag::DateTimeOriginal, "2022:01:01 01:00:00"),
        ascii(Tag::SubSecTimeOriginal, "123456"),
        ascii(Tag::OffsetTimeOriginal, "+03:00"),
    ]);
    assert_eq!(
        metadata.taken_at_local.as_deref(),
        Some("2022-01-01T01:00:00.123")
    );
    assert_eq!(
        metadata.taken_at.as_deref(),
        Some("2021-12-31T22:00:00.123Z")
    );
    let junk = photos.jpeg_with(&[
        ascii(Tag::DateTimeOriginal, ORIGINAL),
        ascii(Tag::SubSecTimeOriginal, "abc"),
    ]);
    assert_eq!(
        junk.taken_at_local.as_deref(),
        Some("2022-08-10T11:37:00.000")
    );
}

// ---------------------------------------------------------------- EXIF-03

fn sao_paulo() -> Vec<Field> {
    vec![
        rational(Tag::GPSLatitude, &[(23, 1), (33, 1), (1818, 100)]),
        ascii(Tag::GPSLatitudeRef, "S"),
        rational(Tag::GPSLongitude, &[(46, 1), (37, 1), (5988, 100)]),
        ascii(Tag::GPSLongitudeRef, "W"),
    ]
}

#[test]
fn exif03_degrees_minutes_seconds_with_references() {
    let photos = Photos::new("gps");
    let metadata = photos.jpeg_with(&sao_paulo());
    assert!((metadata.latitude.unwrap() - -23.555_05).abs() < 1e-6);
    assert!((metadata.longitude.unwrap() - -46.633_3).abs() < 1e-6);
    assert_eq!(metadata.altitude, None);

    let mut north_east = sao_paulo();
    north_east[1] = ascii(Tag::GPSLatitudeRef, "N");
    north_east[3] = ascii(Tag::GPSLongitudeRef, "E");
    let metadata = photos.jpeg_with(&north_east);
    assert!((metadata.latitude.unwrap() - 23.555_05).abs() < 1e-6);
    assert!((metadata.longitude.unwrap() - 46.633_3).abs() < 1e-6);
}

#[test]
fn exif03_altitude_sign_follows_its_reference() {
    let photos = Photos::new("gps-altitude");
    for (reference, expected) in [(Some(1), -760.0), (Some(0), 760.0), (None, 760.0)] {
        let mut fields = sao_paulo();
        fields.push(rational(Tag::GPSAltitude, &[(760, 1)]));
        if let Some(reference) = reference {
            fields.push(byte(Tag::GPSAltitudeRef, reference));
        }
        assert_eq!(
            photos.jpeg_with(&fields).altitude,
            Some(expected),
            "{reference:?}"
        );
    }
    let mut broken = sao_paulo();
    broken.push(rational(Tag::GPSAltitude, &[(760, 0)]));
    let metadata = photos.jpeg_with(&broken);
    assert!(metadata.latitude.is_some());
    assert_eq!(metadata.altitude, None);
}

#[test]
fn exif03_unusable_points_mean_no_location() {
    let photos = Photos::new("gps-bad");
    let cases: Vec<(&str, Vec<Field>)> = vec![
        (
            "no latitude ref",
            sao_paulo()
                .into_iter()
                .filter(|f| f.tag != Tag::GPSLatitudeRef)
                .collect(),
        ),
        (
            "no longitude ref",
            sao_paulo()
                .into_iter()
                .filter(|f| f.tag != Tag::GPSLongitudeRef)
                .collect(),
        ),
        ("bad ref", {
            let mut fields = sao_paulo();
            fields[1] = ascii(Tag::GPSLatitudeRef, "X");
            fields
        }),
        ("zero denominator", {
            let mut fields = sao_paulo();
            fields[0] = rational(Tag::GPSLatitude, &[(23, 1), (33, 0), (1818, 100)]);
            fields
        }),
        ("latitude over 90", {
            let mut fields = sao_paulo();
            fields[0] = rational(Tag::GPSLatitude, &[(90, 1), (0, 1), (1, 1)]);
            fields
        }),
        ("longitude over 180", {
            let mut fields = sao_paulo();
            fields[2] = rational(Tag::GPSLongitude, &[(180, 1), (0, 1), (1, 1)]);
            fields
        }),
        (
            "zero zero",
            vec![
                rational(Tag::GPSLatitude, &[(0, 1), (0, 1), (0, 1)]),
                ascii(Tag::GPSLatitudeRef, "N"),
                rational(Tag::GPSLongitude, &[(0, 1), (0, 1), (0, 1)]),
                ascii(Tag::GPSLongitudeRef, "E"),
                rational(Tag::GPSAltitude, &[(5, 1)]),
            ],
        ),
        ("two values", {
            let mut fields = sao_paulo();
            fields[0] = rational(Tag::GPSLatitude, &[(23, 1), (33, 1)]);
            fields
        }),
    ];
    for (label, mut fields) in cases {
        fields.push(rational(Tag::GPSAltitude, &[(760, 1)]));
        let metadata = photos.jpeg_with(&fields);
        assert_eq!(
            (metadata.latitude, metadata.longitude, metadata.altitude),
            (None, None, None),
            "{label}"
        );
    }
}

#[test]
fn edge_gps_range_ends_are_valid() {
    let photos = Photos::new("gps-edge");
    let metadata = photos.jpeg_with(&[
        rational(Tag::GPSLatitude, &[(90, 1), (0, 1), (0, 1)]),
        ascii(Tag::GPSLatitudeRef, "S"),
        rational(Tag::GPSLongitude, &[(180, 1), (0, 1), (0, 1)]),
        ascii(Tag::GPSLongitudeRef, "E"),
    ]);
    assert_eq!(
        (metadata.latitude, metadata.longitude),
        (Some(-90.0), Some(180.0))
    );
    let equator = photos.jpeg_with(&[
        rational(Tag::GPSLatitude, &[(0, 1), (0, 1), (0, 1)]),
        ascii(Tag::GPSLatitudeRef, "N"),
        rational(Tag::GPSLongitude, &[(10, 1), (0, 1), (0, 1)]),
        ascii(Tag::GPSLongitudeRef, "E"),
    ]);
    assert_eq!(
        (equator.latitude, equator.longitude),
        (Some(0.0), Some(10.0))
    );
}

// ---------------------------------------------------------------- EXIF-04

fn pixel_dimensions(width: u16, height: u16) -> Vec<Field> {
    vec![
        short(Tag::PixelXDimension, width),
        short(Tag::PixelYDimension, height),
    ]
}

#[test]
fn exif04_header_size_wins_over_exif_dimensions() {
    let photos = Photos::new("size-header");
    let exif = tiff(&pixel_dimensions(100, 50));
    let from_jpeg = photos.read("a.jpg", &jpeg(Some(&exif), Some((4032, 3024))));
    assert_eq!(
        (from_jpeg.width, from_jpeg.height),
        (Some(4032), Some(3024))
    );
    let from_png = photos.read("b.png", &png(Some(&exif), 4032, 3024));
    assert_eq!((from_png.width, from_png.height), (Some(4032), Some(3024)));
    assert!(from_png.raw_json.is_some());
    // Chosen by content: upper-case, .jpeg, and a JPEG under a RAW name all use the header.
    for name in ["C.JPG", "d.jpeg", "e.dng"] {
        let metadata = photos.read(name, &jpeg(Some(&exif), Some((4032, 3024))));
        assert_eq!(
            (metadata.width, metadata.height),
            (Some(4032), Some(3024)),
            "{name}"
        );
    }
    let from_webp = photos.read("f.webp", &webp(Some(&exif), 4032, 3024));
    assert_eq!(
        (from_webp.width, from_webp.height),
        (Some(4032), Some(3024))
    );
    assert!(from_webp
        .raw_json
        .as_deref()
        .unwrap()
        .contains("PixelXDimension"));
}

#[test]
fn exif04_orientation_is_one_to_eight() {
    let photos = Photos::new("orientation");
    for value in 1..=8 {
        let metadata = photos.jpeg_with(&[short(Tag::Orientation, value)]);
        assert_eq!(metadata.orientation, Some(value));
    }
    for value in [0, 9] {
        assert_eq!(
            photos
                .jpeg_with(&[short(Tag::Orientation, value)])
                .orientation,
            None
        );
    }
    assert_eq!(photos.jpeg_with(&[ascii(Tag::Make, "X")]).orientation, None);
}

// ---------------------------------------------------------------- EXIF-05, EXIF-06

#[test]
fn exif05_photos_without_exif_still_have_dimensions() {
    let photos = Photos::new("no-exif");
    let gif = photos.read("a.gif", b"GIF89a\x0a\x00\x05\x00\x00\x00\x00;");
    let plain_png = photos.read("b.png", &png(None, 32, 16));
    let plain_jpeg = photos.read("c.jpg", &jpeg(None, Some((640, 480))));
    for (metadata, size) in [
        (gif, (10, 5)),
        (plain_png, (32, 16)),
        (plain_jpeg, (640, 480)),
    ] {
        assert_eq!(
            metadata,
            EmbeddedMetadata {
                width: Some(size.0),
                height: Some(size.1),
                ..EmbeddedMetadata::default()
            }
        );
    }
}

#[test]
fn exif05_garbage_exif_is_flagged_and_the_size_is_kept() {
    let photos = Photos::new("corrupt");
    for garbage in [
        &b"XXXXXXXXXXXXXXXX"[..],
        b"II*\0\xff\xff\xff\xff",
        b"MM\0*\0\0\0\x08\xff\xff",
    ] {
        let metadata = photos.read("a.jpg", &jpeg(Some(garbage), Some((640, 480))));
        assert!(metadata.exif_corrupt, "{garbage:?}");
        assert_eq!((metadata.width, metadata.height), (Some(640), Some(480)));
        assert_eq!(metadata.taken_at_local, None);
        assert_eq!(metadata.raw_json, None);
    }
}

#[test]
fn exif05_missing_file_is_unreadable() {
    let photos = Photos::new("missing");
    let error = read_embedded(&photos.scratch.path.join("absent.jpg")).unwrap_err();
    assert_eq!(error, EmbeddedError::Unreadable);
    assert_eq!(error.as_code(), "unreadable");
}

#[test]
fn exif05_raw_json_is_readable_text_without_maker_note() {
    let photos = Photos::new("raw-json");
    let mut fields = sao_paulo();
    fields.push(ascii(Tag::DateTimeOriginal, ORIGINAL));
    fields.push(ascii(Tag::ImageDescription, &"x".repeat(3000)));
    fields.push(Field {
        tag: Tag::MakerNote,
        ifd_num: In::PRIMARY,
        value: Value::Undefined(vec![1, 2, 3, 4], 0),
    });
    let metadata = photos.jpeg_with(&fields);
    let json: serde_json::Value =
        serde_json::from_str(metadata.raw_json.as_deref().unwrap()).unwrap();
    let object = json.as_object().unwrap();
    assert!(object["DateTimeOriginal"]
        .as_str()
        .unwrap()
        .contains("2022"));
    assert!(object["GPSLatitude"].is_string());
    assert!(object["GPSLatitudeRef"].is_string());
    assert!(!object.contains_key("MakerNote"));
    assert!(object
        .values()
        .all(|value| value.as_str().unwrap().chars().count() <= 1024));
    assert_eq!(
        object["ImageDescription"].as_str().unwrap().chars().count(),
        1024
    );
}

#[test]
fn exif06_reading_never_changes_the_file() {
    let photos = Photos::new("readonly");
    let mut fields = sao_paulo();
    fields.push(ascii(Tag::DateTimeOriginal, ORIGINAL));
    let path = photos.write("a.jpg", &jpeg(Some(&tiff(&fields)), Some((640, 480))));
    let before = (
        fs::read(&path).unwrap(),
        fs::metadata(&path).unwrap().modified().unwrap(),
    );
    read_embedded(&path).unwrap();
    read_embedded(&path).unwrap();
    let after = (
        fs::read(&path).unwrap(),
        fs::metadata(&path).unwrap().modified().unwrap(),
    );
    assert_eq!(before, after);
    assert_eq!(fs::read_dir(&photos.scratch.path).unwrap().count(), 1);
}

// ---------------------------------------------------------------- Verifier iteration 1

#[test]
fn exif04_zero_sizes_are_absent() {
    let photos = Photos::new("size-zero");
    let header = photos.read("a.png", &png(None, 0, 16));
    assert_eq!((header.width, header.height), (None, None));
    for (width, height) in [(0, 3024), (4032, 0)] {
        let exif = photos.read(
            "b.jpg",
            &jpeg(Some(&tiff(&pixel_dimensions(width, height))), None),
        );
        assert_eq!((exif.width, exif.height), (None, None), "{width}x{height}");
    }
}

#[test]
fn exif05_a_disk_error_after_opening_is_unreadable() {
    // Opening a directory succeeds on Unix and reading it fails; on Windows the open fails.
    let photos = Photos::new("dir");
    let error = read_embedded(&photos.scratch.path).unwrap_err();
    assert_eq!(error, EmbeddedError::Unreadable);
}

#[test]
fn exif05_truncated_photo_is_flagged_corrupt() {
    let photos = Photos::new("truncated");
    let mut bytes = jpeg(None, Some((640, 480)));
    bytes.truncate(bytes.len() - 2);
    let metadata = photos.read("a.jpg", &bytes);
    assert!(metadata.exif_corrupt);
    assert_eq!(metadata.raw_json, None);
}

#[test]
fn exif05_partly_readable_exif_keeps_what_was_read() {
    let photos = Photos::new("partial");
    let fields = [
        ascii(Tag::Make, "Fabricante Quebrado"),
        ascii(Tag::Model, "Modelo Bom"),
        ascii(Tag::DateTimeOriginal, ORIGINAL),
    ];
    let mut exif = tiff(&fields);
    // Big-endian TIFF: IFD0 at offset 8 → entry count, then 12-byte entries. Point the
    // Make value (stored by offset) far outside the block.
    assert_eq!(&exif[..4], b"MM\0*");
    let count = u16::from_be_bytes([exif[8], exif[9]]) as usize;
    let make = (0..count)
        .map(|index| 10 + index * 12)
        .find(|&entry| exif[entry..entry + 2] == [0x01, 0x0F])
        .expect("Make entry");
    exif[make + 8..make + 12].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0x00]);

    let metadata = photos.read("a.jpg", &jpeg(Some(&exif), Some((640, 480))));
    assert!(metadata.exif_corrupt);
    assert_eq!(
        metadata.taken_at_local.as_deref(),
        Some("2022-08-10T11:37:00.000")
    );
    let raw = metadata.raw_json.expect("partial fields kept");
    assert!(raw.contains("Modelo Bom"));
    assert!(!raw.contains("Fabricante"));
}

#[test]
fn edge_north_pole_and_antimeridian_west_are_valid() {
    let photos = Photos::new("gps-edge-2");
    let metadata = photos.jpeg_with(&[
        rational(Tag::GPSLatitude, &[(90, 1), (0, 1), (0, 1)]),
        ascii(Tag::GPSLatitudeRef, "N"),
        rational(Tag::GPSLongitude, &[(180, 1), (0, 1), (0, 1)]),
        ascii(Tag::GPSLongitudeRef, "W"),
    ]);
    assert_eq!(
        (metadata.latitude, metadata.longitude),
        (Some(90.0), Some(-180.0))
    );
}

#[test]
fn exif05_raw_json_ignores_thumbnail_fields() {
    let photos = Photos::new("raw-thumb");
    let metadata = photos.jpeg_with(&[
        ascii(Tag::Make, "Principal"),
        Field {
            tag: Tag::Make,
            ifd_num: In::THUMBNAIL,
            value: Value::Ascii(vec![b"Miniatura".to_vec()]),
        },
        Field {
            tag: Tag::Software,
            ifd_num: In::THUMBNAIL,
            value: Value::Ascii(vec![b"SoMiniatura".to_vec()]),
        },
    ]);
    let raw = metadata.raw_json.unwrap();
    assert!(raw.contains("Principal"));
    assert!(!raw.contains("Miniatura"));
}

#[test]
fn exif02_gps_offset_beyond_minus_fourteen_hours_is_unknown() {
    let photos = Photos::new("gps-west");
    // Local 00:05 on the 10th vs UTC 14:35 on the 10th: −14:30.
    let mut fields = vec![ascii(Tag::DateTimeOriginal, "2022:08:10 00:05:00")];
    gps_time(&mut fields, "2022:08:10", (14, 35, 0));
    let metadata = photos.jpeg_with(&fields);
    assert_eq!((metadata.taken_at_offset, metadata.taken_at), (None, None));
}

#[test]
fn exif02_gps_offset_rounds_to_the_nearest_quarter_hour() {
    let photos = Photos::new("gps-round");
    // Local 17:33:10 vs UTC 14:35:10: +2:58 → +03:00 (down would give +02:45).
    let mut fields = vec![ascii(Tag::DateTimeOriginal, "2022:08:10 17:33:10")];
    gps_time(&mut fields, "2022:08:10", (14, 35, 10));
    let metadata = photos.jpeg_with(&fields);
    assert_eq!(metadata.taken_at_offset.as_deref(), Some("+03:00"));
    assert_eq!(
        metadata.taken_at.as_deref(),
        Some("2022-08-10T14:33:10.000Z")
    );
}

#[test]
fn exif04_tiff_and_raw_content_use_exif_pixel_dimensions() {
    let photos = Photos::new("size-tiff");
    let mut fields = pixel_dimensions(6000, 4000);
    fields.push(short(Tag::ImageWidth, 160));
    fields.push(short(Tag::ImageLength, 120));
    let raw = tiff(&fields);
    for name in ["a.CR2", "b.nef", "c.arw", "d.dng", "e.tif", "f.jpg"] {
        let path = photos.write(name, &raw);
        // The header (first IFD) says 160x120: that is the preview, not the photo.
        let header = imagesize::size(&path).unwrap();
        assert_eq!((header.width, header.height), (160, 120));
        let metadata = read_embedded(&path).unwrap();
        assert_eq!(
            (metadata.width, metadata.height),
            (Some(6000), Some(4000)),
            "{name}"
        );
    }
    let no_pixels = photos.read(
        "g.dng",
        &tiff(&[short(Tag::ImageWidth, 160), short(Tag::ImageLength, 120)]),
    );
    assert_eq!((no_pixels.width, no_pixels.height), (None, None));
}

#[test]
fn exif04_heif_header_is_never_used_whatever_the_name() {
    let photos = Photos::new("size-heif");
    for name in ["a.heic", "b.HEIF", "c.jpg"] {
        let path = photos.write(name, &heif_rotated(4032, 3024));
        // imagesize applies irot and would report the photo turned.
        let header = imagesize::size(&path).unwrap();
        assert_eq!((header.width, header.height), (3024, 4032));
        let metadata = read_embedded(&path).unwrap();
        assert_eq!((metadata.width, metadata.height), (None, None), "{name}");
    }
    // A JPEG body under a HEIC name is a JPEG.
    let jpeg_named_heic = photos.read("d.heic", &jpeg(None, Some((640, 480))));
    assert_eq!(
        (jpeg_named_heic.width, jpeg_named_heic.height),
        (Some(640), Some(480))
    );
}
