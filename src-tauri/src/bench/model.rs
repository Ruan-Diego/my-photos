use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum VolumeClass {
    Hdd,
    Ssd,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheMode {
    Warm,
    Cold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashFailure {
    pub relative_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineReport {
    pub item_count: u32,
    pub bytes_per_item: u32,
    pub volume_class: VolumeClass,
    pub volume_root: String,
    pub cache_mode: CacheMode,
    pub worker_count: u32,
    pub files_hashed: u32,
    pub total_bytes: u64,
    pub failure_count: u32,
    pub failures: Vec<HashFailure>,
    pub content_digest: String,
    pub walk_elapsed_ms: u64,
    pub hash_elapsed_ms: u64,
    pub ok: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchError {
    CountOutOfRange,
    RootNotAbsolute,
    RootIsVolume,
    ParentMissing,
    RootNotEmpty,
    SetupFailed,
    CleanupFailed,
}

impl BenchError {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::CountOutOfRange => "count_out_of_range",
            Self::RootNotAbsolute => "root_not_absolute",
            Self::RootIsVolume => "root_is_volume",
            Self::ParentMissing => "parent_missing",
            Self::RootNotEmpty => "root_not_empty",
            Self::SetupFailed => "setup_failed",
            Self::CleanupFailed => "cleanup_failed",
        }
    }
}
