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

#[derive(Debug, Clone, PartialEq)]
pub struct MediaMetadata {
    pub taken_at_unix: Option<i64>,
    pub creation_time_unix: Option<i64>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub altitude: Option<f64>,
    pub people: Vec<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
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
