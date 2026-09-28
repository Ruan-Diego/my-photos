mod corpus;
mod hash;
mod model;
mod volume;

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Instant;

pub use model::{BaselineReport, BenchError, HashFailure, VolumeClass};

const BYTES_PER_ITEM: u32 = 4096;
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;

pub fn run_baseline(root: &Path, item_count: u32) -> Result<BaselineReport, BenchError> {
    run_baseline_with(root, item_count, Hooks::default())
}

pub(crate) type OpenForHash = fn(&Path) -> io::Result<Box<dyn Read + Send>>;

#[derive(Clone, Copy)]
pub(crate) struct Hooks {
    pub open_for_hash: Option<OpenForHash>,
    pub fail_setup_at: Option<u32>,
    pub fail_cleanup: bool,
    // SPEC_DEVIATION: plants a reparse point after the corpus exists.
    // Reason: validate rejects a non-empty root, so the junction test cannot create the link earlier.
    pub before_walk: Option<fn(&Path)>,
}

impl Default for Hooks {
    fn default() -> Self {
        Self {
            open_for_hash: None,
            fail_setup_at: None,
            fail_cleanup: false,
            before_walk: None,
        }
    }
}

pub(crate) fn run_baseline_with(
    root: &Path,
    item_count: u32,
    hooks: Hooks,
) -> Result<BaselineReport, BenchError> {
    corpus::validate(root, item_count)?;
    let created_root = !root.exists();
    if created_root {
        fs::create_dir(root).map_err(|_| BenchError::SetupFailed)?;
    }
    if let Err(error) = write_corpus(root, item_count, hooks.fail_setup_at) {
        let _ = discard_created(root, created_root);
        return Err(error);
    }
    if let Some(hook) = hooks.before_walk {
        hook(root);
    }

    let walk_started = Instant::now();
    let files = walk_files(root);
    let walk_elapsed_ms = elapsed_ms(walk_started);

    let worker_count = available_workers();
    let hash_started = Instant::now();
    let hashed = hash_files(root, item_count, &files, worker_count, hooks.open_for_hash);
    let hash_elapsed_ms = elapsed_ms(hash_started);
    let ok = hashed.failures.is_empty() && hashed.files_hashed == item_count;

    let volume_root = volume::volume_root_of(root);
    let report = BaselineReport {
        item_count,
        bytes_per_item: BYTES_PER_ITEM,
        volume_class: volume::classify_seek_penalty(volume::query_seek_penalty(&volume_root)),
        volume_root,
        worker_count,
        files_hashed: hashed.files_hashed,
        total_bytes: u64::from(hashed.files_hashed) * u64::from(BYTES_PER_ITEM),
        failure_count: hashed.failures.len() as u32,
        failures: hashed.failures,
        content_digest: hash::content_digest(&hashed.slots),
        walk_elapsed_ms,
        hash_elapsed_ms,
        ok,
    };

    if hooks.fail_cleanup {
        return Err(BenchError::CleanupFailed);
    }
    remove_measured_root(root).map_err(|_| BenchError::CleanupFailed)?;
    Ok(report)
}

fn write_corpus(root: &Path, item_count: u32, fail_setup_at: Option<u32>) -> Result<(), BenchError> {
    for index in 0..item_count {
        if fail_setup_at == Some(index) {
            return Err(BenchError::SetupFailed);
        }
        let path = corpus_file(root, index);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|_| BenchError::SetupFailed)?;
        }
        fs::write(&path, corpus::file_bytes(index)).map_err(|_| BenchError::SetupFailed)?;
    }
    if fail_setup_at.is_some() {
        return Err(BenchError::SetupFailed);
    }
    Ok(())
}

fn corpus_file(root: &Path, index: u32) -> PathBuf {
    corpus::relative_path(index)
        .split('/')
        .fold(root.to_path_buf(), |path, part| path.join(part))
}

fn discard_created(root: &Path, created_root: bool) -> std::io::Result<()> {
    if created_root {
        remove_measured_root(root)
    } else {
        remove_children(root)
    }
}

fn remove_measured_root(root: &Path) -> std::io::Result<()> {
    if root.exists() {
        fs::remove_dir_all(root)?;
    }
    Ok(())
}

fn remove_children(dir: &Path) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let meta = fs::symlink_metadata(&path)?;
        if meta.is_dir() {
            if is_reparse(&meta) {
                fs::remove_dir_all(&path)?;
            } else {
                remove_children(&path)?;
                fs::remove_dir(&path)?;
            }
        } else {
            fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                if !is_reparse(&meta) {
                    dirs.push(path);
                }
            } else if meta.is_file() {
                files.push(path);
            }
        }
    }
    files
}

fn is_reparse(meta: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        let _ = FILE_ATTRIBUTE_REPARSE_POINT;
        meta.file_type().is_symlink()
    }
}

struct Hashed {
    files_hashed: u32,
    slots: Vec<Option<[u8; 32]>>,
    failures: Vec<HashFailure>,
}

fn hash_files(
    root: &Path,
    item_count: u32,
    files: &[PathBuf],
    worker_count: u32,
    open_for_hash: Option<OpenForHash>,
) -> Hashed {
    let results = Mutex::new((0..files.len()).map(|_| None).collect::<Vec<_>>());
    let next = AtomicUsize::new(0);
    let threads = if files.is_empty() {
        0
    } else {
        (worker_count as usize).clamp(1, files.len())
    };
    thread::scope(|scope| {
        for _ in 0..threads {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::Relaxed);
                if index >= files.len() {
                    break;
                }
                let outcome = hash_file(root, item_count, &files[index], open_for_hash);
                results.lock().expect("hash results")[index] = Some(outcome);
            });
        }
    });

    let mut hashed = Hashed {
        files_hashed: 0,
        slots: vec![None; item_count as usize],
        failures: Vec::new(),
    };
    for outcome in results.into_inner().expect("hash results").into_iter().flatten() {
        if let Some(digest) = outcome.digest {
            hashed.files_hashed += 1;
            if let Some(index) = outcome.corpus_index {
                hashed.slots[index as usize] = Some(digest);
            }
        } else {
            hashed.failures.push(HashFailure {
                relative_path: outcome.relative_path,
                reason: "unreadable".to_string(),
            });
        }
    }
    hashed
}

struct HashOutcome {
    corpus_index: Option<u32>,
    relative_path: String,
    digest: Option<[u8; 32]>,
}

fn hash_file(
    root: &Path,
    item_count: u32,
    path: &Path,
    open_for_hash: Option<OpenForHash>,
) -> HashOutcome {
    let opened = match open_for_hash {
        Some(open) => open(path),
        None => File::open(path).map(|file| Box::new(file) as Box<dyn Read + Send>),
    };
    let digest = opened.and_then(hash::hash_reader).ok();
    HashOutcome {
        corpus_index: corpus_index(root, path, item_count),
        relative_path: relative_of(root, path),
        digest,
    }
}

fn corpus_index(root: &Path, path: &Path, item_count: u32) -> Option<u32> {
    let name = path.file_name()?.to_str()?;
    let number = name.strip_prefix("item-")?.strip_suffix(".bin")?;
    if number.len() != 7 {
        return None;
    }
    let index = number.parse::<u32>().ok()?;
    if index >= item_count || relative_of(root, path) != corpus::relative_path(index) {
        return None;
    }
    Some(index)
}

fn relative_of(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn available_workers() -> u32 {
    thread::available_parallelism()
        .map(|count| count.get() as u32)
        .unwrap_or(1)
        .max(1)
}

fn elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::io::{self, Read};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{run_baseline, run_baseline_with, Hooks};

    const DIGEST_64: &str = "c067a4e6cde3d7306e1fef52a92ebee10c1e562368ad06488bb1398610067415";

    #[test]
    fn absent_root_of_64_is_created_measured_and_removed() {
        let scratch = Scratch::new("created");
        let root = scratch.path.join("corpus");
        let report = run_baseline(&root, 64).expect("baseline");
        assert_success(&report);
        assert!(!root.exists());
        let json = serde_json::to_value(&report).unwrap();
        assert!(json["walkElapsedMs"].as_u64().is_some());
        assert!(json["hashElapsedMs"].as_u64().is_some());
    }

    #[test]
    fn pre_existing_empty_root_is_removed_after_the_same_measurement() {
        let scratch = Scratch::new("empty");
        let root = scratch.path.join("corpus");
        fs::create_dir(&root).unwrap();
        let report = run_baseline(&root, 64).expect("baseline");
        assert_success(&report);
        assert!(!root.exists());
    }

    #[test]
    fn second_run_returns_the_same_content_digest() {
        let scratch = Scratch::new("repeat");
        let first = run_baseline(&scratch.path.join("one"), 64)
            .expect("first")
            .content_digest;
        let second = run_baseline(&scratch.path.join("two"), 64)
            .expect("second")
            .content_digest;
        assert_eq!(first, DIGEST_64);
        assert_eq!(second, DIGEST_64);
    }

    #[test]
    fn report_records_workers_and_the_volume_class() {
        let scratch = Scratch::new("volume");
        let root = scratch.path.join("corpus");
        let report = run_baseline(&root, 64).expect("baseline");
        let expected_root = super::volume::volume_root_of(&root);
        let expected_class = super::volume::classify_seek_penalty(super::volume::query_seek_penalty(
            &expected_root,
        ));
        let expected_workers = std::thread::available_parallelism()
            .map(|count| count.get() as u32)
            .unwrap_or(1)
            .max(1);
        assert_eq!(report.worker_count, expected_workers);
        assert_eq!(report.volume_class, expected_class);
        assert_eq!(report.volume_root, expected_root);
        assert!(report.volume_root.ends_with(":\\"), "{}", report.volume_root);
    }

    #[test]
    fn marker_outside_the_root_is_unchanged() {
        let scratch = Scratch::new("marker");
        let marker = scratch.path.join("marker.bin");
        fs::write(&marker, b"keep-outside").unwrap();
        let root = scratch.path.join("corpus");
        run_baseline(&root, 64).expect("baseline");
        assert_eq!(fs::read(&marker).unwrap(), b"keep-outside");
        assert!(!root.exists());
    }

    #[test]
    fn junction_pointing_outside_is_not_hashed_or_deleted() {
        let scratch = Scratch::new("junction");
        let root = scratch.path.join("corpus");
        let report = run_baseline_with(
            &root,
            64,
            Hooks {
                before_walk: Some(plant_junction),
                ..Hooks::default()
            },
        )
        .expect("baseline");
        assert_success(&report);
        let sentinel = scratch.path.join("outside").join("sentinel.bin");
        assert_eq!(fs::read(&sentinel).unwrap(), b"outside");
        assert!(!root.exists());
    }

    #[test]
    fn report_json_uses_camel_case_names() {
        let scratch = Scratch::new("json");
        let report = run_baseline(&scratch.path.join("corpus"), 64).expect("baseline");
        let json = serde_json::to_string(&report).unwrap();
        for key in [
            "itemCount",
            "bytesPerItem",
            "volumeClass",
            "volumeRoot",
            "workerCount",
            "filesHashed",
            "totalBytes",
            "failureCount",
            "failures",
            "contentDigest",
            "walkElapsedMs",
            "hashElapsedMs",
            "ok",
        ] {
            assert!(json.contains(&format!("\"{key}\"")), "{key} missing");
        }
        assert!(!json.contains("item_count"));
        assert!(!json.contains("walk_elapsed_ms"));
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["itemCount"], 64);
        assert_eq!(value["bytesPerItem"], 4096);
        assert_eq!(value["filesHashed"], 64);
        assert_eq!(value["totalBytes"], 262144);
        assert_eq!(value["failureCount"], 0);
        assert_eq!(value["failures"], serde_json::json!([]));
        assert_eq!(value["contentDigest"], DIGEST_64);
        assert_eq!(value["ok"], true);
        assert!(matches!(
            value["volumeClass"].as_str(),
            Some("hdd" | "ssd" | "unknown")
        ));
    }

    #[test]
    fn one_unreadable_file_stays_in_failures_and_the_root_is_removed() {
        let scratch = Scratch::new("one-bad");
        let root = scratch.path.join("corpus");
        let report = run_baseline_with(
            &root,
            64,
            Hooks {
                open_for_hash: Some(open_except_first),
                ..Hooks::default()
            },
        )
        .expect("baseline");
        assert!(!report.ok);
        assert_eq!(report.files_hashed, 63);
        assert_eq!(report.failure_count, 1);
        assert_eq!(report.failures[0].reason, "unreadable");
        assert_eq!(
            report.failures[0].relative_path,
            "f03/2000/01/item-0000000.bin"
        );
        assert!(!root.exists());
    }

    #[test]
    fn every_unreadable_file_leaves_an_empty_digest() {
        let scratch = Scratch::new("all-bad");
        let root = scratch.path.join("corpus");
        let report = run_baseline_with(
            &root,
            64,
            Hooks {
                open_for_hash: Some(open_nothing),
                ..Hooks::default()
            },
        )
        .expect("baseline");
        assert!(!report.ok);
        assert_eq!(report.files_hashed, 0);
        assert_eq!(report.content_digest, "");
        assert_eq!(report.failure_count, 64);
        assert!(report.failures.iter().all(|failure| {
            failure.reason == "unreadable" && failure.relative_path.contains('/')
        }));
        assert!(!root.exists());
    }

    #[test]
    fn failed_setup_removes_what_the_call_created() {
        let scratch = Scratch::new("setup");
        let marker = scratch.path.join("marker.bin");
        fs::write(&marker, b"parent").unwrap();
        let root = scratch.path.join("corpus");
        let error = run_baseline_with(
            &root,
            64,
            Hooks {
                fail_setup_at: Some(2),
                ..Hooks::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.as_code(), "setup_failed");
        assert!(!root.exists());
        assert_eq!(fs::read(&marker).unwrap(), b"parent");
    }

    #[test]
    fn failed_cleanup_leaves_the_root() {
        let scratch = Scratch::new("cleanup");
        let root = scratch.path.join("corpus");
        let error = run_baseline_with(
            &root,
            64,
            Hooks {
                fail_cleanup: true,
                ..Hooks::default()
            },
        )
        .unwrap_err();
        assert_eq!(error.as_code(), "cleanup_failed");
        assert!(root.join("f03").is_dir());
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn non_empty_root_is_rejected_without_changing_the_marker() {
        let scratch = Scratch::new("occupied");
        let root = scratch.path.join("corpus");
        fs::create_dir(&root).unwrap();
        let marker = root.join("marker.bin");
        fs::write(&marker, b"keep").unwrap();
        let error = run_baseline(&root, 64).unwrap_err();
        assert_eq!(error.as_code(), "root_not_empty");
        assert_eq!(fs::read(&marker).unwrap(), b"keep");
    }

    fn assert_success(report: &super::BaselineReport) {
        assert!(report.ok);
        assert_eq!(report.item_count, 64);
        assert_eq!(report.files_hashed, 64);
        assert_eq!(report.total_bytes, 262_144);
        assert_eq!(report.bytes_per_item, 4096);
        assert_eq!(report.failure_count, 0);
        assert!(report.failures.is_empty());
        assert_eq!(report.content_digest, DIGEST_64);
    }

    fn open_except_first(path: &Path) -> io::Result<Box<dyn Read + Send>> {
        if path.file_name().and_then(|name| name.to_str()) == Some("item-0000000.bin") {
            return Err(io::Error::other("unreadable"));
        }
        Ok(Box::new(File::open(path)?))
    }

    fn open_nothing(_path: &Path) -> io::Result<Box<dyn Read + Send>> {
        Err(io::Error::other("unreadable"))
    }

    fn plant_junction(root: &Path) {
        let outside = root.parent().unwrap().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("sentinel.bin"), b"outside").unwrap();
        let link = root.join("leak");
        let link_arg = link.to_string_lossy().into_owned();
        let outside_arg = outside.to_string_lossy().into_owned();
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J", &link_arg, &outside_arg])
            .status()
            .expect("mklink");
        assert!(status.success(), "mklink {status}");
    }

    struct Scratch {
        path: PathBuf,
    }

    impl Scratch {
        fn new(label: &str) -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "my-photos-f03-{label}-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self { path }
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
