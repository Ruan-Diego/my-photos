use serde_json::Value;

use crate::takeout::model::{AlbumMetadata, MediaMetadata};

pub fn parse_media_json(bytes: &[u8]) -> Result<MediaMetadata, ()> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| ())?;
    let (latitude, longitude, altitude) = read_media_location(&value);
    Ok(MediaMetadata {
        taken_at_unix: timestamp_at(&value, "photoTakenTime"),
        creation_time_unix: timestamp_at(&value, "creationTime"),
        latitude,
        longitude,
        altitude,
        people: read_people(&value),
        title: read_text(&value, "title"),
        description: read_text(&value, "description"),
        url: read_text(&value, "url"),
    })
}

pub fn parse_album_json(bytes: &[u8]) -> Result<AlbumMetadata, ()> {
    let value: Value = serde_json::from_slice(bytes).map_err(|_| ())?;
    let (latitude, longitude, altitude) = value
        .get("geoData")
        .and_then(read_point)
        .unwrap_or((None, None, None));
    Ok(AlbumMetadata {
        title: read_text(&value, "title"),
        description: read_text(&value, "description"),
        date_unix: value
            .get("date")
            .and_then(|date| date.get("timestamp"))
            .and_then(parse_timestamp),
        latitude,
        longitude,
        altitude,
    })
}

fn read_media_location(value: &Value) -> (Option<f64>, Option<f64>, Option<f64>) {
    if let Some(point) = value.get("geoData").and_then(read_point) {
        return point;
    }
    value
        .get("geoDataExif")
        .and_then(read_point)
        .unwrap_or((None, None, None))
}

fn read_point(value: &Value) -> Option<(Option<f64>, Option<f64>, Option<f64>)> {
    let latitude = value.get("latitude")?.as_f64()?;
    let longitude = value.get("longitude")?.as_f64()?;
    if latitude == 0.0 && longitude == 0.0 {
        return None;
    }
    let altitude = value.get("altitude").and_then(Value::as_f64);
    Some((Some(latitude), Some(longitude), altitude))
}

fn timestamp_at(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(|node| parse_timestamp(node.get("timestamp")?))
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

fn read_people(value: &Value) -> Vec<String> {
    let Some(people) = value.get("people").and_then(Value::as_array) else {
        return Vec::new();
    };
    people
        .iter()
        .filter_map(|person| {
            let name = person.get("name")?.as_str()?;
            if name.is_empty() {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect()
}

fn read_text(value: &Value, key: &str) -> Option<String> {
    let text = value.get(key)?.as_str()?;
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
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
        let metadata = parse_media_json(
            br#"{ "creationTime": { "timestamp": "1660145820" } }"#,
        )
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
        assert_eq!(metadata.people, vec!["Pessoa A".to_string(), "Pessoa B".to_string()]);
    }

    #[test]
    fn td17_empty_text_is_absent_and_url_is_copied() {
        let empty = parse_media_json(
            br#"{ "title": "", "description": "", "url": "" }"#,
        )
        .expect("json");
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
}
