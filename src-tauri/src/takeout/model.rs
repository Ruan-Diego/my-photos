#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    Exact,
    Heuristic,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnmatchedReason {
    Ambiguous,
}

/// Which sidecar field a location came from; the values of `media_locations.source`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationSource {
    GeoData,
    GeoDataExif,
}

impl LocationSource {
    pub fn as_column(self) -> &'static str {
        match self {
            Self::GeoData => "takeout_geoData",
            Self::GeoDataExif => "takeout_geoDataExif",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaMetadata {
    pub taken_at_unix: Option<i64>,
    /// F06: `taken_at_unix` as ISO-8601 UTC (`2022-08-10T14:37:00.000Z`).
    pub taken_at: Option<String>,
    pub creation_time_unix: Option<i64>,
    /// F06: `creation_time_unix` as ISO-8601 UTC.
    pub creation_time: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    /// F06: set exactly when `latitude`/`longitude` are.
    pub location_source: Option<LocationSource>,
    /// Display names: trimmed, inner whitespace collapsed, first of each normalized name.
    pub people: Vec<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    /// F06: the sidecar text as read, without a leading BOM (`media_metadata_raw`).
    pub raw_json: String,
}

/// `people.normalized_name`: the display name in Unicode lower case.
pub fn normalized_person_name(display: &str) -> String {
    display.to_lowercase()
}

#[derive(Debug, Clone, PartialEq)]
pub struct MediaHit {
    pub relative_path: String,
    pub confidence: Confidence,
    pub json_relative_path: Option<String>,
    pub metadata: Option<MediaMetadata>,
    pub reason: Option<UnmatchedReason>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlbumMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub date_unix: Option<i64>,
    /// F06: `date_unix` as ISO-8601 UTC.
    pub date: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AlbumHit {
    pub relative_path: String,
    pub metadata: AlbumMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProblemKind {
    InvalidJson,
    Unreadable,
    OrphanJson,
    UnreadableDirectory,
    InvalidName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Problem {
    pub relative_path: String,
    pub kind: ProblemKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveryReport {
    pub media: Vec<MediaHit>,
    pub albums: Vec<AlbumHit>,
    pub problems: Vec<Problem>,
}
