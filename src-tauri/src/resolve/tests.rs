use super::*;
use crate::takeout::LocationSource;

fn json(taken_at: Option<&str>) -> MediaMetadata {
    MediaMetadata {
        taken_at_unix: None,
        taken_at: taken_at.map(str::to_string),
        creation_time_unix: None,
        creation_time: Some("2023-01-01T00:00:00.000Z".to_string()),
        latitude: None,
        longitude: None,
        altitude: None,
        location_source: None,
        people: Vec::new(),
        title: None,
        description: None,
        url: None,
        raw_json: "{}".to_string(),
    }
}

fn exif_with_zone() -> EmbeddedMetadata {
    EmbeddedMetadata {
        taken_at_local: Some("2022-08-10T11:37:00.500".to_string()),
        taken_at_offset: Some("-03:00".to_string()),
        taken_at: Some("2022-08-10T14:37:00.500Z".to_string()),
        ..EmbeddedMetadata::default()
    }
}

fn exif_local_only(local: &str) -> EmbeddedMetadata {
    EmbeddedMetadata {
        taken_at_local: Some(local.to_string()),
        ..EmbeddedMetadata::default()
    }
}

const FS: FsTimes<'static> = FsTimes {
    created_at: Some("2026-09-21T18:00:00.000Z"),
    modified_at: Some("2019-05-04T10:00:00.000Z"),
};

fn run(json: Option<&MediaMetadata>, exif: Option<&EmbeddedMetadata>, fs: FsTimes) -> ResolvedMetadata {
    resolve(json, exif, fs, DEFAULT_FALLBACK_OFFSET_MINUTES)
}

#[test]
fn res01_json_date_wins_over_exif_and_filesystem() {
    let json = json(Some("2022-08-10T14:00:00.000Z"));
    let resolved = run(Some(&json), Some(&exif_with_zone()), FS);
    assert_eq!(resolved.taken_at.as_deref(), Some("2022-08-10T14:00:00.000Z"));
    assert_eq!(resolved.taken_at_source, Some(DateSource::TakeoutJson));
    assert_eq!(resolved.taken_at_timezone, None);
    assert!(!resolved.taken_at_offset_assumed);
}

#[test]
fn res01_json_without_a_date_falls_to_exif_never_to_its_creation_time() {
    let json = json(None);
    let resolved = run(Some(&json), Some(&exif_with_zone()), FS);
    assert_eq!(resolved.taken_at.as_deref(), Some("2022-08-10T14:37:00.500Z"));
    assert_eq!(resolved.taken_at_source, Some(DateSource::Exif));
    assert_eq!(resolved.taken_at_timezone.as_deref(), Some("-03:00"));

    let resolved = run(Some(&json), None, FsTimes::default());
    assert_eq!(resolved.taken_at, None);
    assert_eq!(resolved.taken_at_source, None);
}

#[test]
fn res02_exif_without_zone_uses_the_fallback_offset() {
    let exif = exif_local_only("2022-08-10T11:37:00.500");
    let resolved = run(None, Some(&exif), FS);
    assert_eq!(resolved.taken_at.as_deref(), Some("2022-08-10T14:37:00.500Z"));
    assert_eq!(resolved.taken_at_source, Some(DateSource::Exif));
    assert_eq!(resolved.taken_at_timezone, None);
    assert!(resolved.taken_at_offset_assumed);

    let resolved = resolve(None, Some(&exif), FS, 0);
    assert_eq!(resolved.taken_at.as_deref(), Some("2022-08-10T11:37:00.500Z"));
    let resolved = resolve(None, Some(&exif), FS, 330);
    assert_eq!(resolved.taken_at.as_deref(), Some("2022-08-10T06:07:00.500Z"));
}

#[test]
fn res02_fallback_offset_crosses_midnight_and_year_end() {
    let exif = exif_local_only("2021-12-31T22:30:00.000");
    let resolved = run(None, Some(&exif), FS);
    assert_eq!(resolved.taken_at.as_deref(), Some("2022-01-01T01:30:00.000Z"));
    let exif = exif_local_only("2024-02-29T01:00:00.000");
    let resolved = resolve(None, Some(&exif), FS, 120);
    assert_eq!(resolved.taken_at.as_deref(), Some("2024-02-28T23:00:00.000Z"));
}

#[test]
fn res02_unreadable_local_time_falls_to_the_filesystem() {
    for local in ["", "2022-08-10 11:37:00.500", "2022-13-10T11:37:00.000", "2022-08-10T11:37:00"] {
        let exif = exif_local_only(local);
        let resolved = run(None, Some(&exif), FS);
        assert_eq!(resolved.taken_at_source, Some(DateSource::Filesystem), "{local:?}");
        assert!(!resolved.taken_at_offset_assumed, "{local:?}");
    }
}

#[test]
fn res03_filesystem_uses_the_earlier_time() {
    let resolved = run(None, Some(&EmbeddedMetadata::default()), FS);
    assert_eq!(resolved.taken_at.as_deref(), Some("2019-05-04T10:00:00.000Z"));
    assert_eq!(resolved.taken_at_source, Some(DateSource::Filesystem));
    assert_eq!(resolved.taken_at_timezone, None);

    let only_created = FsTimes { created_at: Some("2020-01-01T00:00:00.000Z"), modified_at: None };
    let resolved = run(None, None, only_created);
    assert_eq!(resolved.taken_at.as_deref(), Some("2020-01-01T00:00:00.000Z"));

    let resolved = run(None, None, FsTimes::default());
    assert_eq!(resolved, ResolvedMetadata::default());
}

#[test]
fn res04_json_location_wins_and_keeps_its_source() {
    let mut json = json(None);
    json.latitude = Some(-23.5);
    json.longitude = Some(-46.6);
    json.altitude = Some(760.0);
    json.location_source = Some(LocationSource::GeoDataExif);
    let mut exif = exif_with_zone();
    exif.latitude = Some(1.0);
    exif.longitude = Some(2.0);
    let location = run(Some(&json), Some(&exif), FS).location.expect("location");
    assert_eq!(
        location,
        Location { latitude: -23.5, longitude: -46.6, altitude: Some(760.0), source: "takeout_geoDataExif" }
    );
}

#[test]
fn res04_exif_location_fills_in_when_json_has_none() {
    let json = json(Some("2022-08-10T14:00:00.000Z"));
    let mut exif = exif_with_zone();
    exif.latitude = Some(-22.9);
    exif.longitude = Some(-43.2);
    let location = run(Some(&json), Some(&exif), FS).location.expect("location");
    assert_eq!(
        location,
        Location { latitude: -22.9, longitude: -43.2, altitude: None, source: "exif" }
    );
    assert_eq!(run(Some(&json), None, FS).location, None);
}

#[test]
fn res05_dimensions_come_from_the_file_and_text_from_the_json() {
    let mut json = json(None);
    json.title = Some("IMG_1.jpg".to_string());
    json.description = Some("praia".to_string());
    let exif = EmbeddedMetadata {
        width: Some(4000),
        height: Some(3000),
        orientation: Some(6),
        ..EmbeddedMetadata::default()
    };
    let resolved = run(Some(&json), Some(&exif), FS);
    assert_eq!((resolved.width, resolved.height, resolved.orientation), (Some(4000), Some(3000), Some(6)));
    assert_eq!(resolved.title.as_deref(), Some("IMG_1.jpg"));
    assert_eq!(resolved.description.as_deref(), Some("praia"));

    let video = run(Some(&json), None, FS);
    assert_eq!((video.width, video.height, video.orientation), (None, None, None));
}

#[test]
fn date_source_columns_match_the_schema() {
    assert_eq!(DateSource::TakeoutJson.as_column(), "takeout_json");
    assert_eq!(DateSource::Exif.as_column(), "exif");
    assert_eq!(DateSource::Filesystem.as_column(), "filesystem");
}
