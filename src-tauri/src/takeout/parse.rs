use serde_json::{Map, Value};

use crate::takeout::model::{normalized_person_name, AlbumMetadata, LocationSource, MediaMetadata};
use crate::time::iso_from_unix_seconds;

/// F06: a sidecar above this is `invalid_json`; readers never take more than this + 1.
pub const MAX_SIDECAR_BYTES: usize = 1_048_576;

const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";

pub fn parse_media_json(bytes: &[u8]) -> Result<MediaMetadata, ()> {
    let (text, value) = decode(bytes)?;
    let (taken_at_unix, taken_at) = instant_at(&value, "photoTakenTime");
    let (creation_time_unix, creation_time) = instant_at(&value, "creationTime");
    let location = [
        ("geoData", LocationSource::GeoData),
        ("geoDataExif", LocationSource::GeoDataExif),
    ]
    .into_iter()
    .find_map(|(key, source)| Some((value.get(key).and_then(read_point)?, source)));
    let ((latitude, longitude, altitude), location_source) = match location {
        Some(((latitude, longitude, altitude), source)) => {
            ((Some(latitude), Some(longitude), altitude), Some(source))
        }
        None => ((None, None, None), None),
    };
    Ok(MediaMetadata {
        taken_at_unix,
        taken_at,
        creation_time_unix,
        creation_time,
        latitude,
        longitude,
        altitude,
        location_source,
        people: read_people(&value),
        title: read_text(&value, "title"),
        description: read_text(&value, "description"),
        url: read_text(&value, "url"),
        raw_json: text.to_string(),
    })
}

pub fn parse_album_json(bytes: &[u8]) -> Result<AlbumMetadata, ()> {
    let (_, value) = decode(bytes)?;
    let (latitude, longitude, altitude) = match value.get("geoData").and_then(read_point) {
        Some((latitude, longitude, altitude)) => (Some(latitude), Some(longitude), altitude),
        None => (None, None, None),
    };
    let (date_unix, date) = instant_at(&value, "date");
    Ok(AlbumMetadata {
        title: read_text(&value, "title"),
        description: read_text(&value, "description"),
        date_unix,
        date,
        latitude,
        longitude,
        altitude,
    })
}

/// Size cap, optional BOM, UTF-8, JSON, object root. Returns the text without the BOM.
fn decode(bytes: &[u8]) -> Result<(&str, Map<String, Value>), ()> {
    if bytes.len() > MAX_SIDECAR_BYTES {
        return Err(());
    }
    let bytes = bytes.strip_prefix(UTF8_BOM).unwrap_or(bytes);
    let text = std::str::from_utf8(bytes).map_err(|_| ())?;
    match serde_json::from_str(text).map_err(|_| ())? {
        Value::Object(map) => Ok((text, map)),
        _ => Err(()),
    }
}

/// A point inside the globe that is not exactly 0/0, with an optional altitude.
fn read_point(value: &Value) -> Option<(f64, f64, Option<f64>)> {
    let latitude = value.get("latitude")?.as_f64()?;
    let longitude = value.get("longitude")?.as_f64()?;
    let inside = (-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude);
    if !inside || (latitude == 0.0 && longitude == 0.0) {
        return None;
    }
    // serde_json never yields NaN or infinity, so any number is a usable altitude.
    let altitude = value.get("altitude").and_then(Value::as_f64);
    Some((latitude, longitude, altitude))
}

/// `{key}.timestamp` → (unix seconds, ISO). Both absent for 0, junk or out of range.
fn instant_at(value: &Map<String, Value>, key: &str) -> (Option<i64>, Option<String>) {
    let Some(seconds) = value
        .get(key)
        .and_then(|node| node.get("timestamp"))
        .and_then(parse_timestamp)
        .filter(|seconds| *seconds != 0)
    else {
        return (None, None);
    };
    match iso_from_unix_seconds(seconds) {
        Some(iso) => (Some(seconds), Some(iso)),
        None => (None, None),
    }
}

fn parse_timestamp(value: &Value) -> Option<i64> {
    match value {
        Value::String(text) => parse_decimal(text),
        Value::Number(number) => number.as_i64(),
        _ => None,
    }
}

fn parse_decimal(text: &str) -> Option<i64> {
    if text.is_empty() {
        return None;
    }
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

/// Trims and collapses every run of Unicode whitespace into one space.
fn clean_name(name: &str) -> Option<String> {
    let cleaned = name.split_whitespace().collect::<Vec<_>>().join(" ");
    (!cleaned.is_empty()).then_some(cleaned)
}

fn read_people(value: &Map<String, Value>) -> Vec<String> {
    let Some(people) = value.get("people").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut seen = Vec::new();
    let mut names = Vec::new();
    for person in people {
        let Some(name) = person
            .get("name")
            .and_then(Value::as_str)
            .and_then(clean_name)
        else {
            continue;
        };
        let normalized = normalized_person_name(&name);
        if !seen.contains(&normalized) {
            seen.push(normalized);
            names.push(name);
        }
    }
    names
}

fn read_text(value: &Map<String, Value>, key: &str) -> Option<String> {
    let text = value.get(key)?.as_str()?.trim();
    (!text.is_empty()).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::{parse_album_json, parse_media_json};

    #[test]
    fn td11_taken_at_comes_from_photo_taken_time() {
        let metadata = parse_media_json(
            br#"{
                "photoTakenTime": { "timestamp": "1660142220" },
                "creationTime": { "timestamp": "1660145820" }
            }"#,
        )
        .expect("json");
        assert_eq!(metadata.taken_at_unix, Some(1660142220));
        assert_eq!(metadata.creation_time_unix, Some(1660145820));
    }

    #[test]
    fn td12_creation_time_does_not_become_taken_at() {
        let metadata = parse_media_json(br#"{ "creationTime": { "timestamp": "1660145820" } }"#)
            .expect("json");
        assert_eq!(metadata.taken_at_unix, None);
        assert_eq!(metadata.creation_time_unix, Some(1660145820));
    }

    #[test]
    fn td13_zero_coordinates_clear_altitude_too() {
        let metadata = parse_media_json(
            br#"{ "geoData": { "latitude": 0.0, "longitude": 0.0, "altitude": 0.0 } }"#,
        )
        .expect("json");
        assert_eq!(metadata.latitude, None);
        assert_eq!(metadata.longitude, None);
        assert_eq!(metadata.altitude, None);
    }

    #[test]
    fn td14_exif_coordinates_and_altitude_fill_a_zero_geo_data() {
        let metadata = parse_media_json(
            br#"{
                "geoData": { "latitude": 0.0, "longitude": 0.0, "altitude": 0.0 },
                "geoDataExif": { "latitude": -23.5505, "longitude": -46.6333, "altitude": 760.0 }
            }"#,
        )
        .expect("json");
        assert_eq!(metadata.latitude, Some(-23.5505));
        assert_eq!(metadata.longitude, Some(-46.6333));
        assert_eq!(metadata.altitude, Some(760.0));
    }

    #[test]
    fn td15_real_geo_data_wins_over_exif() {
        let metadata = parse_media_json(
            br#"{
                "geoData": { "latitude": -23.5505, "longitude": -46.6333, "altitude": 760.0 },
                "geoDataExif": { "latitude": 10.0, "longitude": 20.0, "altitude": 1.0 }
            }"#,
        )
        .expect("json");
        assert_eq!(metadata.latitude, Some(-23.5505));
        assert_eq!(metadata.longitude, Some(-46.6333));
        assert_eq!(metadata.altitude, Some(760.0));
    }

    #[test]
    fn td16_people_keep_non_empty_names_in_order() {
        let metadata = parse_media_json(
            br#"{ "people": [ { "name": "Pessoa A" }, { "name": "" }, { "name": "Pessoa B" } ] }"#,
        )
        .expect("json");
        assert_eq!(
            metadata.people,
            vec!["Pessoa A".to_string(), "Pessoa B".to_string()]
        );
    }

    #[test]
    fn td17_empty_text_is_absent_and_url_is_copied() {
        let empty =
            parse_media_json(br#"{ "title": "", "description": "", "url": "" }"#).expect("json");
        assert_eq!(empty.title, None);
        assert_eq!(empty.description, None);
        assert_eq!(empty.url, None);

        let missing = parse_media_json(b"{}").expect("json");
        assert_eq!(missing.title, None);
        assert_eq!(missing.description, None);
        assert_eq!(missing.url, None);

        let url = "https://photos.google.com/photo/x";
        let copied = parse_media_json(format!(r#"{{ "url": "{url}" }}"#).as_bytes()).expect("json");
        assert_eq!(copied.url.as_deref(), Some(url));
    }

    #[test]
    fn td18_unusable_timestamp_stays_associated_and_non_json_fails() {
        let metadata = parse_media_json(
            br#"{
                "photoTakenTime": { "timestamp": "ontem" },
                "creationTime": { "timestamp": "1660145820" }
            }"#,
        )
        .expect("association remains");
        assert_eq!(metadata.taken_at_unix, None);
        assert_eq!(metadata.creation_time_unix, Some(1660145820));
        assert!(parse_media_json(b"not json").is_err());
    }

    #[test]
    fn missing_people_is_an_empty_list() {
        let metadata = parse_media_json(b"{}").expect("json");
        assert_eq!(metadata.people, Vec::<String>::new());
    }

    #[test]
    fn unknown_field_is_ignored() {
        let metadata = parse_media_json(
            br#"{
                "photoTakenTime": { "timestamp": 1660142220 },
                "imageViews": "12"
            }"#,
        )
        .expect("json");
        assert_eq!(metadata.taken_at_unix, Some(1660142220));
        assert_eq!(metadata.creation_time_unix, None);
        assert_eq!(metadata.title, None);
        assert_eq!(metadata.description, None);
        assert_eq!(metadata.url, None);
        assert_eq!(metadata.latitude, None);
        assert_eq!(metadata.longitude, None);
        assert_eq!(metadata.altitude, None);
        assert_eq!(metadata.people, Vec::<String>::new());
    }

    #[test]
    fn td20_album_example_drops_empty_description_and_zero_coordinates() {
        let album = parse_album_json(
            br#"{
                "title": "Viagem Europa 2022",
                "description": "",
                "access": "protected",
                "date": {
                    "timestamp": "1660000000",
                    "formatted": "9 de ago. de 2022, 00:00:00 UTC"
                },
                "geoData": { "latitude": 0.0, "longitude": 0.0, "altitude": 0.0 }
            }"#,
        )
        .expect("json");
        assert_eq!(album.title.as_deref(), Some("Viagem Europa 2022"));
        assert_eq!(album.description, None);
        assert_eq!(album.date_unix, Some(1660000000));
        assert_eq!(album.latitude, None);
        assert_eq!(album.longitude, None);
        assert_eq!(album.altitude, None);
    }

    #[test]
    fn album_bytes_that_are_not_json_return_an_error() {
        assert!(parse_album_json(b"not json").is_err());
    }

    // ------------------------------------------------------------ F06

    use super::MAX_SIDECAR_BYTES;
    use crate::takeout::model::{normalized_person_name, LocationSource};

    fn media(json: &str) -> crate::takeout::model::MediaMetadata {
        parse_media_json(json.as_bytes()).expect("json")
    }

    #[test]
    fn json01_string_and_number_timestamps_become_iso() {
        for taken in [r#""1660142220""#, "1660142220"] {
            let metadata = media(&format!(
                r#"{{ "photoTakenTime": {{ "timestamp": {taken} }},
                     "creationTime": {{ "timestamp": "1660145820" }} }}"#
            ));
            assert_eq!(metadata.taken_at_unix, Some(1_660_142_220));
            assert_eq!(
                metadata.taken_at.as_deref(),
                Some("2022-08-10T14:37:00.000Z")
            );
            assert_eq!(metadata.creation_time_unix, Some(1_660_145_820));
            assert_eq!(
                metadata.creation_time.as_deref(),
                Some("2022-08-10T15:37:00.000Z")
            );
        }
    }

    #[test]
    fn json01_zero_junk_and_out_of_range_are_absent_and_negative_is_kept() {
        for timestamp in [
            r#""0""#,
            "0",
            r#""ontem""#,
            r#"" 1660142220""#,
            r#""+1""#,
            r#""-5364662401""#,
            "253402300800",
            "1.5",
            "null",
        ] {
            let metadata = media(&format!(
                r#"{{ "photoTakenTime": {{ "timestamp": {timestamp} }}, "title": "ainda lido" }}"#
            ));
            assert_eq!(
                (metadata.taken_at_unix, metadata.taken_at),
                (None, None),
                "{timestamp}"
            );
            assert_eq!(metadata.title.as_deref(), Some("ainda lido"));
        }
        let old = media(r#"{ "photoTakenTime": { "timestamp": "-86400" } }"#);
        assert_eq!(old.taken_at_unix, Some(-86_400));
        assert_eq!(old.taken_at.as_deref(), Some("1969-12-31T00:00:00.000Z"));
        let first = media(r#"{ "creationTime": { "timestamp": -5364662400 } }"#);
        assert_eq!(
            first.creation_time.as_deref(),
            Some("1800-01-01T00:00:00.000Z")
        );
        let last = media(r#"{ "creationTime": { "timestamp": "253402300799" } }"#);
        assert_eq!(
            last.creation_time.as_deref(),
            Some("9999-12-31T23:59:59.000Z")
        );
    }

    #[test]
    fn json02_geo_data_wins_and_names_its_source() {
        let metadata = media(
            r#"{ "geoData": { "latitude": -23.5505, "longitude": -46.6333, "altitude": 760.0 },
                 "geoDataExif": { "latitude": 10.0, "longitude": 20.0, "altitude": 1.0 } }"#,
        );
        assert_eq!(metadata.location_source, Some(LocationSource::GeoData));
        assert_eq!(
            (metadata.latitude, metadata.longitude),
            (Some(-23.5505), Some(-46.6333))
        );
        assert_eq!(metadata.altitude, Some(760.0));
        assert_eq!(LocationSource::GeoData.as_column(), "takeout_geoData");
        assert_eq!(
            LocationSource::GeoDataExif.as_column(),
            "takeout_geoDataExif"
        );
    }

    #[test]
    fn json02_invalid_geo_data_falls_back_to_exif() {
        for geo in [
            r#""geoData": { "latitude": 0.0, "longitude": 0.0 },"#,
            r#""geoData": { "latitude": 90.5, "longitude": 10.0 },"#,
            r#""geoData": { "latitude": -91, "longitude": 10.0 },"#,
            r#""geoData": { "latitude": 10.0, "longitude": 180.01 },"#,
            r#""geoData": { "latitude": 10.0, "longitude": -181 },"#,
            r#""geoData": { "latitude": "10", "longitude": 10.0 },"#,
            r#""geoData": { "latitude": 10.0 },"#,
            r#""geoData": "x","#,
            "",
        ] {
            let metadata = media(&format!(
                r#"{{ {geo} "geoDataExif": {{ "latitude": -23.5, "longitude": -46.6, "altitude": 760 }} }}"#
            ));
            assert_eq!(
                metadata.location_source,
                Some(LocationSource::GeoDataExif),
                "{geo}"
            );
            assert_eq!(
                (metadata.latitude, metadata.longitude),
                (Some(-23.5), Some(-46.6))
            );
            assert_eq!(metadata.altitude, Some(760.0));
        }
    }

    #[test]
    fn json02_no_valid_point_means_no_location_at_all() {
        let metadata = media(
            r#"{ "geoData": { "latitude": 0.0, "longitude": 0.0, "altitude": 5.0 },
                 "geoDataExif": { "latitude": 100.0, "longitude": 0.0, "altitude": 5.0 } }"#,
        );
        assert_eq!(
            (
                metadata.latitude,
                metadata.longitude,
                metadata.altitude,
                metadata.location_source
            ),
            (None, None, None, None)
        );
    }

    #[test]
    fn json02_range_ends_are_valid_and_altitude_needs_a_number() {
        let edge =
            media(r#"{ "geoData": { "latitude": 90, "longitude": -180, "altitude": "alto" } }"#);
        assert_eq!((edge.latitude, edge.longitude), (Some(90.0), Some(-180.0)));
        assert_eq!(edge.altitude, None);
        assert_eq!(edge.location_source, Some(LocationSource::GeoData));
        let other = media(r#"{ "geoData": { "latitude": -90, "longitude": 180 } }"#);
        assert_eq!(
            (other.latitude, other.longitude),
            (Some(-90.0), Some(180.0))
        );
    }

    #[test]
    fn json03_people_are_cleaned_and_deduplicated() {
        let metadata = media(
            "{ \"people\": [ { \"name\": \" Ana  Maria \" }, { \"name\": \"ana maria\" }, { \"name\": \"\" },
                             { \"name\": \"   \" }, { \"name\": \"Bruno\" }, { \"name\": \"BRUNO\" },
                             { \"name\": \"Caio\\u00a0\\tLima\" }, { \"other\": 1 }, { \"name\": 3 } ] }",
        );
        assert_eq!(metadata.people, ["Ana Maria", "Bruno", "Caio Lima"]);
        let normalized: Vec<_> = metadata
            .people
            .iter()
            .map(|name| normalized_person_name(name))
            .collect();
        assert_eq!(normalized, ["ana maria", "bruno", "caio lima"]);
        assert_eq!(normalized_person_name("ÉLISE"), "élise");
        let first = media(r#"{ "people": [ { "name": "BRUNO" }, { "name": "Bruno" } ] }"#);
        assert_eq!(first.people, ["BRUNO"]);
        let not_a_list = media(r#"{ "people": { "name": "Ana" } }"#);
        assert!(not_a_list.people.is_empty());
    }

    #[test]
    fn json03_text_is_trimmed_and_inner_newlines_stay() {
        let metadata = media(
            r#"{ "title": "  IMG_1.jpg ", "description": "  linha 1\nlinha 2\n ", "url": " https://x/y " }"#,
        );
        assert_eq!(metadata.title.as_deref(), Some("IMG_1.jpg"));
        assert_eq!(metadata.description.as_deref(), Some("linha 1\nlinha 2"));
        assert_eq!(metadata.url.as_deref(), Some("https://x/y"));
        let blank = media(r#"{ "title": "   ", "description": "\n\t", "url": " " }"#);
        assert_eq!(
            (blank.title, blank.description, blank.url),
            (None, None, None)
        );
    }

    #[test]
    fn json04_raw_json_is_the_text_as_read_without_the_bom() {
        let text = "{ \"title\":  \"á\" ,\n \"x\": [1, 2] }\n";
        assert_eq!(media(text).raw_json, text);
        let mut with_bom = b"\xEF\xBB\xBF".to_vec();
        with_bom.extend_from_slice(text.as_bytes());
        let metadata = parse_media_json(&with_bom).expect("BOM accepted");
        assert_eq!(metadata.raw_json, text);
        assert_eq!(metadata.title.as_deref(), Some("á"));
    }

    #[test]
    fn json04_album_gets_iso_date_valid_location_and_clean_text() {
        let album = parse_album_json(
            br#"{ "title": " Viagem ", "description": "  ",
                  "date": { "timestamp": "1660000000" },
                  "geoData": { "latitude": 48.8584, "longitude": 2.2945, "altitude": 35.0 } }"#,
        )
        .expect("json");
        assert_eq!(album.title.as_deref(), Some("Viagem"));
        assert_eq!(album.description, None);
        assert_eq!(album.date_unix, Some(1_660_000_000));
        assert_eq!(album.date.as_deref(), Some("2022-08-08T23:06:40.000Z"));
        assert_eq!(
            (album.latitude, album.longitude, album.altitude),
            (Some(48.8584), Some(2.2945), Some(35.0))
        );

        let bad = parse_album_json(
            br#"{ "date": { "timestamp": "0" }, "geoData": { "latitude": 91, "longitude": 0, "altitude": 1 },
                  "geoDataExif": { "latitude": 10, "longitude": 10 } }"#,
        )
        .expect("json");
        assert_eq!((bad.date_unix, bad.date), (None, None));
        assert_eq!(
            (bad.latitude, bad.longitude, bad.altitude),
            (None, None, None)
        );
    }

    #[test]
    fn json05_size_cap_is_inclusive() {
        let fill = |total: usize| {
            let mut bytes = b"{\"title\":\"x\",\"pad\":\"".to_vec();
            let tail = b"\"}";
            bytes.resize(total - tail.len(), b'a');
            bytes.extend_from_slice(tail);
            assert_eq!(bytes.len(), total);
            bytes
        };
        assert!(parse_media_json(&fill(MAX_SIDECAR_BYTES)).is_ok());
        assert!(parse_media_json(&fill(MAX_SIDECAR_BYTES + 1)).is_err());
        assert!(parse_album_json(&fill(MAX_SIDECAR_BYTES)).is_ok());
        assert!(parse_album_json(&fill(MAX_SIDECAR_BYTES + 1)).is_err());
        assert_eq!(MAX_SIDECAR_BYTES, 1_048_576);
    }

    #[test]
    fn json05_root_must_be_an_object_and_text_utf8() {
        for bad in [
            &b"[]"[..],
            b"\"x\"",
            b"3",
            b"null",
            b"true",
            b"",
            b"\xEF\xBB\xBF",
            b"{\"t\":\"\xFF\"}",
        ] {
            assert!(parse_media_json(bad).is_err(), "{bad:?}");
            assert!(parse_album_json(bad).is_err(), "{bad:?}");
        }
        assert!(parse_media_json(b"\xEF\xBB\xBF{}").is_ok());
        assert!(parse_album_json(b"\xEF\xBB\xBF{}").is_ok());
    }
}
