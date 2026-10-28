//! THUMB-01..07 on a temp tree and a temp database. Never `E:\`. Images are made with
//! the same `image` crate the service decodes with.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use image::{ImageFormat, Rgb, RgbImage, Rgba, RgbaImage};
use rusqlite::{params, Connection};

use super::*;
use crate::db::tests::Scratch;
use crate::db::MIGRATIONS;

const NOW: &str = "2026-10-14T12:00:00.000Z";

struct Library {
    _scratch: Scratch,
    root: PathBuf,
    connection: Connection,
}

impl Library {
    fn new(label: &str) -> Self {
        let scratch = Scratch::new(&format!("f12-{label}"));
        let root = scratch.path.join("root");
        fs::create_dir_all(&root).unwrap();
        let connection = Connection::open(scratch.path.join("library.db")).unwrap();
        connection.pragma_update(None, "foreign_keys", "ON").unwrap();
        for migration in MIGRATIONS {
            connection.execute_batch(migration.sql).unwrap();
        }
        connection
            .execute(
                "INSERT INTO libraries (name, root_path, created_at) VALUES ('E', ?1, ?2)",
                params![root.to_string_lossy(), NOW],
            )
            .unwrap();
        Self {
            _scratch: scratch,
            root,
            connection,
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        path
    }

    fn photo(&self, relative: &str, width: u32, height: u32) {
        let image = RgbImage::from_fn(width, height, |x, y| Rgb([(x % 256) as u8, (y % 256) as u8, 90]));
        image.save(self.path(relative)).unwrap();
    }

    fn bytes(&self, relative: &str, bytes: &[u8]) {
        fs::write(self.path(relative), bytes).unwrap();
    }

    fn media(&self, relative: &str, media_type: &str, taken_at: Option<&str>) -> i64 {
        self.connection
            .execute(
                "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type,
                   size_bytes, taken_at, fs_modified_at, created_at, updated_at)
                 VALUES (1, ?1, ?1, 'jpg', ?2, 100, ?3, '2020-01-01T00:00:00.000Z', ?4, ?4)",
                params![relative, media_type, taken_at, NOW],
            )
            .unwrap();
        self.connection.last_insert_rowid()
    }

    fn pending_ids(&self, size: ThumbSize) -> Vec<i64> {
        let ids: Vec<i64> = pending(&self.connection, &self.root, size, false)
            .unwrap()
            .into_iter()
            .map(|job| job.media_id)
            .collect();
        assert_eq!(pending_count(&self.connection, &self.root, size, false).unwrap(), ids.len() as u64);
        ids
    }

    fn generate(self, size: ThumbSize, workers: usize) -> (Self, ThumbSummary) {
        let Self {
            _scratch,
            root,
            connection,
        } = self;
        let (connection, summary) = generate_with(connection, &root, size, workers, None, &mut |_, _| {}, &|| false);
        (
            Self {
                _scratch,
                root,
                connection,
            },
            summary.expect("generate"),
        )
    }

    fn thumbnail_row(&self, id: i64, size: ThumbSize) -> Option<(u32, u32, i64, String)> {
        self.connection
            .query_row(
                "SELECT width, height, bytes, last_accessed_at FROM thumbnails WHERE media_id = ?1 AND size = ?2",
                params![id, size.as_column()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .ok()
    }
}

fn dimensions(jpeg: &[u8]) -> (u32, u32) {
    assert_eq!(image::guess_format(jpeg).unwrap(), ImageFormat::Jpeg);
    let image = image::load_from_memory(jpeg).unwrap();
    (image.width(), image.height())
}

fn job(id: i64, taken_at: Option<&str>, priority: Priority) -> Job {
    Job {
        media_id: id,
        size: ThumbSize::Small,
        relative_path: format!("{id}.jpg"),
        orientation: None,
        fingerprint: String::new(),
        taken_at: taken_at.map(str::to_string),
        video: false,
        priority,
    }
}

#[test]
fn thumb01_each_size_fits_its_edge_and_keeps_the_aspect() {
    let library = Library::new("fit");
    library.photo("big.png", 3000, 2000);
    let original = library.root.join("big.png");
    for (size, expected) in [
        (ThumbSize::Small, (300, 200)),
        (ThumbSize::Medium, (1280, 853)),
        (ThumbSize::Large, (2560, 1707)),
    ] {
        let rendered = render(&original, size, None).unwrap();
        assert_eq!((rendered.width, rendered.height), expected, "{size:?}");
        assert_eq!(dimensions(&rendered.jpeg), expected, "{size:?}");
    }
}

#[test]
fn thumb01_small_originals_are_never_upscaled() {
    let library = Library::new("no-upscale");
    library.photo("tiny.jpg", 120, 60);
    let rendered = render(&library.root.join("tiny.jpg"), ThumbSize::Large, None).unwrap();
    assert_eq!(dimensions(&rendered.jpeg), (120, 60));
}

#[test]
fn thumb01_exif_orientation_is_baked_in() {
    let library = Library::new("orientation");
    library.photo("side.png", 600, 300);
    let original = library.root.join("side.png");
    let turned = render(&original, ThumbSize::Small, Some(6)).unwrap();
    assert_eq!((turned.width, turned.height), (150, 300));
    assert_eq!(dimensions(&turned.jpeg), (150, 300));
    let flipped = render(&original, ThumbSize::Small, Some(3)).unwrap();
    assert_eq!((flipped.width, flipped.height), (300, 150));
    // Out of range: ignored.
    let odd = render(&original, ThumbSize::Small, Some(42)).unwrap();
    assert_eq!((odd.width, odd.height), (300, 150));
}

#[test]
fn thumb01_transparency_is_flattened_on_white() {
    let library = Library::new("alpha");
    let image = RgbaImage::from_fn(40, 40, |x, _| {
        if x < 20 { Rgba([0, 0, 0, 0]) } else { Rgba([0, 0, 0, 255]) }
    });
    image.save(library.path("logo.png")).unwrap();
    let rendered = render(&library.root.join("logo.png"), ThumbSize::Small, None).unwrap();
    let decoded = image::load_from_memory(&rendered.jpeg).unwrap().into_rgb8();
    let clear = decoded.get_pixel(5, 20);
    let solid = decoded.get_pixel(35, 20);
    assert!(clear.0.iter().all(|&channel| channel > 240), "{clear:?}");
    assert!(solid.0.iter().all(|&channel| channel < 15), "{solid:?}");
}

#[test]
fn thumb02_unreadable_originals_say_why() {
    let library = Library::new("errors");
    library.bytes("phone.heic", b"\0\0\0\x18ftypheic\0\0\0\0mif1heic....");
    library.bytes("junk.jpg", b"not an image at all");
    library.photo("whole.jpg", 400, 300);
    let whole = fs::read(library.root.join("whole.jpg")).unwrap();
    // A JPEG cut mid-scan still decodes (the rest is grey); one cut in the header does not.
    library.bytes("cut.jpg", &whole[..20]);
    let result = |name: &str| render(&library.root.join(name), ThumbSize::Small, None).map(|_| ());
    assert_eq!(result("phone.heic"), Err(ThumbError::Unsupported));
    assert_eq!(result("junk.jpg"), Err(ThumbError::Corrupt));
    assert_eq!(result("cut.jpg"), Err(ThumbError::Corrupt));
    assert_eq!(result("missing.jpg"), Err(ThumbError::Unreadable));
    assert_eq!(ThumbError::Unsupported.as_column(), "unsupported_format");
    assert_eq!(ThumbError::Corrupt.as_column(), "corrupt_media");
    assert_eq!(ThumbError::Unreadable.as_column(), "unreadable");
}

#[test]
fn thumb03_requests_first_then_newest_first() {
    let mut queue = ThumbQueue::default();
    assert!(queue.push(job(1, Some("2020-01-01T00:00:00.000Z"), Priority::Background)));
    assert!(queue.push(job(2, Some("2024-06-01T00:00:00.000Z"), Priority::Background)));
    assert!(queue.push(job(3, None, Priority::Background)));
    assert!(queue.push(job(4, Some("2010-01-01T00:00:00.000Z"), Priority::Requested)));
    let order: Vec<i64> = std::iter::from_fn(|| queue.pop()).map(|job| job.media_id).collect();
    assert_eq!(order, [4, 2, 1, 3]);
    assert!(queue.is_empty());
}

#[test]
fn thumb03_a_request_moves_a_queued_job_up_once() {
    let mut queue = ThumbQueue::default();
    assert!(queue.push(job(1, Some("2024-01-01T00:00:00.000Z"), Priority::Background)));
    assert!(queue.push(job(2, Some("2020-01-01T00:00:00.000Z"), Priority::Background)));
    assert!(!queue.push(job(2, Some("2020-01-01T00:00:00.000Z"), Priority::Background)));
    assert!(queue.push(job(2, Some("2020-01-01T00:00:00.000Z"), Priority::Requested)));
    assert!(!queue.push(job(2, Some("2020-01-01T00:00:00.000Z"), Priority::Background)));
    let popped: Vec<(i64, Priority)> =
        std::iter::from_fn(|| queue.pop()).map(|job| (job.media_id, job.priority)).collect();
    assert_eq!(popped, [(2, Priority::Requested), (1, Priority::Background)]);
}

#[test]
fn thumb03_a_full_cache_drops_only_background_jobs() {
    let mut queue = ThumbQueue::default();
    queue.push(job(1, None, Priority::Background));
    queue.push(job(2, None, Priority::Background));
    queue.push(job(2, None, Priority::Requested));
    queue.push(job(3, None, Priority::Requested));
    assert_eq!(queue.drop_background(), 1);
    let order: Vec<i64> = std::iter::from_fn(|| queue.pop()).map(|job| job.media_id).collect();
    assert_eq!(order, [3, 2]);
}

#[test]
fn thumb04_pending_lists_photos_without_an_up_to_date_thumbnail_newest_first() {
    let library = Library::new("pending");
    let old = library.media("old.jpg", "photo", Some("2019-01-01T00:00:00.000Z"));
    let new = library.media("new.jpg", "photo", Some("2023-01-01T00:00:00.000Z"));
    let undated = library.media("undated.jpg", "photo", None);
    let video = library.media("clip.mp4", "video", Some("2024-01-01T00:00:00.000Z"));
    let gone = library.media("gone.jpg", "photo", Some("2024-01-01T00:00:00.000Z"));
    library
        .connection
        .execute("UPDATE media_items SET indexing_status = 'missing_file' WHERE id = ?1", [gone])
        .unwrap();
    assert_eq!(library.pending_ids(ThumbSize::Small), [new, old, undated]);
    assert!(job_for(&library.connection, video, ThumbSize::Small, false).unwrap().is_none());
    assert!(job_for(&library.connection, gone, ThumbSize::Small, true).unwrap().is_none());

    let jobs = pending(&library.connection, &library.root, ThumbSize::Small, false).unwrap();
    record_done(&library.connection, &jobs[0], 300, 200, 1000, NOW).unwrap();
    record_failure(&library.connection, &jobs[1], ThumbError::Unsupported, NOW).unwrap();
    assert_eq!(library.pending_ids(ThumbSize::Small), [undated]);
    // A failure covers every size; a thumbnail only its own.
    assert_eq!(library.pending_ids(ThumbSize::Medium), [new, undated]);

    // The file changed: both come back.
    library
        .connection
        .execute("UPDATE media_items SET size_bytes = 200 WHERE id IN (?1, ?2)", [new, old])
        .unwrap();
    assert_eq!(library.pending_ids(ThumbSize::Small), [new, old, undated]);
    // Rotated by the user later: the baked-in rotation is stale.
    record_done(&library.connection, &pending(&library.connection, &library.root, ThumbSize::Small, false).unwrap()[0], 300, 200, 1000, NOW).unwrap();
    assert_eq!(library.pending_ids(ThumbSize::Small), [old, undated]);
    library
        .connection
        .execute("UPDATE media_items SET orientation = 6 WHERE id = ?1", [new])
        .unwrap();
    assert_eq!(library.pending_ids(ThumbSize::Small), [new, old, undated]);
}

#[test]
fn thumb04_pending_ignores_other_libraries() {
    let library = Library::new("other-library");
    library
        .connection
        .execute(
            "INSERT INTO libraries (name, root_path, created_at) VALUES ('F', 'F:\\', ?1)",
            [NOW],
        )
        .unwrap();
    let mine = library.media("a.jpg", "photo", None);
    library
        .connection
        .execute(
            "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type, created_at, updated_at)
             VALUES (2, 'b.jpg', 'b.jpg', 'jpg', 'photo', ?1, ?1)",
            [NOW],
        )
        .unwrap();
    assert_eq!(library.pending_ids(ThumbSize::Small), [mine]);
}

#[test]
fn thumb04_fingerprint_and_paths() {
    assert_eq!(fingerprint(Some(10), Some("2020-01-01T00:00:00.000Z"), Some(6)), "10|2020-01-01T00:00:00.000Z|6");
    assert_eq!(fingerprint(None, None, None), "||");
    let root = Path::new("lib");
    assert_eq!(
        thumb_path(root, 1066, ThumbSize::Small),
        root.join(".myphotos").join("thumbs").join("small").join("2a").join("1066.jpg")
    );
    assert_eq!(
        thumb_path(root, 7, ThumbSize::Large),
        root.join(".myphotos").join("thumbs").join("large").join("07").join("7.jpg")
    );
    for size in ThumbSize::ALL {
        assert_eq!(ThumbSize::from_column(size.as_column()), Some(size));
    }
    assert_eq!(ThumbSize::from_column("huge"), None);
}

#[test]
fn thumb05_generate_library_makes_each_missing_thumbnail_once() {
    let library = Library::new("generate");
    library.photo("Fotos/a.png", 900, 600);
    library.photo("Fotos/b.jpg", 200, 400);
    library.bytes("Fotos/c.jpg", b"broken");
    library.bytes("Fotos/d.heic", b"\0\0\0\x18ftypheic\0\0\0\0mif1heic....");
    let a = library.media("Fotos/a.png", "photo", Some("2022-01-01T00:00:00.000Z"));
    let b = library.media("Fotos/b.jpg", "photo", Some("2023-01-01T00:00:00.000Z"));
    let c = library.media("Fotos/c.jpg", "photo", None);
    let d = library.media("Fotos/d.heic", "photo", None);
    let e = library.media("Fotos/e.jpg", "photo", None);
    library.media("Fotos/v.mp4", "video", None);

    let (library, summary) = library.generate(ThumbSize::Small, 2);
    assert_eq!(summary.queued, 5);
    assert_eq!(
        (summary.generated, summary.corrupt, summary.unsupported, summary.unreadable, summary.write_failed),
        (2, 1, 1, 1, 0)
    );
    assert!(!summary.stopped_cache_full);
    assert_eq!(summary.cache_limit_bytes, DEFAULT_CACHE_LIMIT_BYTES);
    // No ffmpeg in this run (F13): the video is counted, not failed.
    assert_eq!((summary.videos, summary.videos_skipped, summary.tool_failed), (0, 1, 0));

    let (width, height, bytes, _) = library.thumbnail_row(a, ThumbSize::Small).unwrap();
    assert_eq!((width, height), (300, 200));
    let file = fs::read(thumb_path(&library.root, a, ThumbSize::Small)).unwrap();
    assert_eq!(file.len() as i64, bytes);
    assert_eq!(dimensions(&file), (300, 200));
    assert_eq!(library.thumbnail_row(b, ThumbSize::Small).unwrap().0, 150);
    assert_eq!(summary.cache_bytes, cache_bytes(&library.connection).unwrap());
    let reasons: Vec<(i64, String)> = library
        .connection
        .prepare("SELECT media_id, reason FROM thumbnail_failures ORDER BY media_id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        reasons,
        [
            (c, "corrupt_media".to_string()),
            (d, "unsupported_format".to_string()),
            (e, "unreadable".to_string())
        ]
    );
    // Nothing is written next to the originals.
    let mut originals: Vec<String> = fs::read_dir(library.root.join("Fotos"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    originals.sort();
    assert_eq!(originals, ["a.png", "b.jpg", "c.jpg", "d.heic"]);

    let (library, again) = library.generate(ThumbSize::Small, 2);
    assert_eq!((again.queued, again.done()), (0, 0));

    // The broken file was replaced: tried again and its failure is cleared.
    library.photo("Fotos/c.jpg", 300, 300);
    library
        .connection
        .execute("UPDATE media_items SET size_bytes = 999 WHERE id = ?1", [c])
        .unwrap();
    let (library, fixed) = library.generate(ThumbSize::Small, 1);
    assert_eq!((fixed.queued, fixed.generated), (1, 1));
    let failures: i64 = library
        .connection
        .query_row("SELECT count(*) FROM thumbnail_failures WHERE media_id = ?1", [c], |row| row.get(0))
        .unwrap();
    assert_eq!(failures, 0);
}

#[test]
fn thumb06_requests_are_served_and_marked_used() {
    let library = Library::new("request");
    library.photo("p.jpg", 2000, 1000);
    let id = library.media("p.jpg", "photo", None);
    assert_eq!(thumbnail_for(&library.connection, &library.root, id, ThumbSize::Medium, NOW).unwrap(), None);

    let Library {
        _scratch,
        root,
        connection,
    } = library;
    let service = ThumbService::start_with(connection, root.clone(), 2, DEFAULT_CACHE_LIMIT_BYTES, None);
    service.request(id, ThumbSize::Medium);
    service.wait_idle(&mut |_, _| {});
    assert_eq!(service.summary().generated, 1);
    // Already up to date: asking again makes nothing new.
    service.request(id, ThumbSize::Medium);
    service.request(9999, ThumbSize::Medium);
    service.wait_idle(&mut |_, _| {});
    let (connection, summary) = service.shutdown();
    assert_eq!((summary.queued, summary.generated), (1, 1));

    let later = "2026-10-15T08:00:00.000Z";
    let path = thumbnail_for(&connection, &root, id, ThumbSize::Medium, later).unwrap();
    assert_eq!(path, Some(thumb_path(&root, id, ThumbSize::Medium)));
    let used: String = connection
        .query_row("SELECT last_accessed_at FROM thumbnails WHERE media_id = ?1", [id], |row| row.get(0))
        .unwrap();
    assert_eq!(used, later);
    assert_eq!(dimensions(&fs::read(path.unwrap()).unwrap()), (1280, 640));

    // File deleted by hand: not served.
    fs::remove_file(thumb_path(&root, id, ThumbSize::Medium)).unwrap();
    assert_eq!(thumbnail_for(&connection, &root, id, ThumbSize::Medium, later).unwrap(), None);
}

#[test]
fn thumb07_eviction_drops_least_recently_used_large_and_medium_first() {
    let library = Library::new("evict");
    let ids: Vec<i64> = (0..3).map(|n| library.media(&format!("{n}.jpg"), "photo", None)).collect();
    let rows = [
        (ids[0], ThumbSize::Small, "2026-01-01T00:00:00.000Z"),
        (ids[0], ThumbSize::Large, "2026-03-01T00:00:00.000Z"),
        (ids[1], ThumbSize::Medium, "2026-02-01T00:00:00.000Z"),
        (ids[1], ThumbSize::Small, "2026-04-01T00:00:00.000Z"),
        (ids[2], ThumbSize::Large, "2026-01-15T00:00:00.000Z"),
    ];
    for (id, size, used) in rows {
        let job = Job {
            media_id: id,
            size,
            relative_path: String::new(),
            orientation: None,
            fingerprint: String::new(),
            taken_at: None,
            video: false,
            priority: Priority::Background,
        };
        record_done(&library.connection, &job, 1, 1, 100, used).unwrap();
        write_thumb(&thumb_path(&library.root, id, size), &[0; 100]).unwrap();
    }
    let remaining = |library: &Library| -> Vec<(i64, String)> {
        library
            .connection
            .prepare("SELECT media_id, size FROM thumbnails ORDER BY media_id, size")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };

    let none = HashSet::new();
    let under = evict(&library.connection, &library.root, 500, false, &none).unwrap();
    assert_eq!((under.count, under.cache_bytes), (0, 500));

    // Over by 150: the two least recently used of medium/large go (ids[2] large, ids[1] medium).
    let eviction = evict(&library.connection, &library.root, 350, false, &none).unwrap();
    assert_eq!((eviction.count, eviction.bytes, eviction.cache_bytes), (2, 200, 300));
    assert!(!thumb_path(&library.root, ids[2], ThumbSize::Large).exists());
    assert!(!thumb_path(&library.root, ids[1], ThumbSize::Medium).exists());

    // Background fill: small ones stay even when still over.
    let stuck = evict(&library.connection, &library.root, 100, false, &none).unwrap();
    assert_eq!((stuck.count, stuck.cache_bytes), (1, 200));
    assert_eq!(
        remaining(&library),
        [(ids[0], "small".to_string()), (ids[1], "small".to_string())]
    );

    // A request may evict small ones, oldest first, but never what it just made.
    let keep = HashSet::from([(ids[0], ThumbSize::Small)]);
    let request = evict(&library.connection, &library.root, 100, true, &keep).unwrap();
    assert_eq!((request.count, request.cache_bytes), (1, 100));
    assert_eq!(remaining(&library), [(ids[0], "small".to_string())]);
}

#[test]
fn thumb07_cache_limit_comes_from_settings() {
    let library = Library::new("limit");
    assert_eq!(cache_limit(&library.connection).unwrap(), DEFAULT_CACHE_LIMIT_BYTES);
    for (value, expected) in [
        ("1048576", 1_048_576),
        (" 2048 ", 2048),
        ("0", DEFAULT_CACHE_LIMIT_BYTES),
        ("-5", DEFAULT_CACHE_LIMIT_BYTES),
        ("muito", DEFAULT_CACHE_LIMIT_BYTES),
    ] {
        library
            .connection
            .execute(
                "INSERT OR REPLACE INTO app_settings (key, value) VALUES (?1, ?2)",
                params![CACHE_LIMIT_SETTING, value],
            )
            .unwrap();
        assert_eq!(cache_limit(&library.connection).unwrap(), expected, "{value:?}");
    }
}

#[test]
fn thumb07_a_full_cache_stops_the_background_fill_and_keeps_what_it_made() {
    let library = Library::new("full");
    for n in 0..4 {
        library.photo(&format!("{n}.png"), 64, 64);
        library.media(&format!("{n}.png"), "photo", None);
    }
    library
        .connection
        .execute(
            "INSERT INTO app_settings (key, value) VALUES (?1, '1')",
            [CACHE_LIMIT_SETTING],
        )
        .unwrap();
    let (library, summary) = library.generate(ThumbSize::Small, 1);
    assert!(summary.stopped_cache_full);
    assert!(summary.generated >= 1);
    assert_eq!(summary.evicted, 0);
    let rows: i64 = library
        .connection
        .query_row("SELECT count(*) FROM thumbnails", [], |row| row.get(0))
        .unwrap();
    assert_eq!(rows as u64, summary.generated);
}

/// F36: a 45-byte PNG whose header claims 100 000 × 100 000 pixels (30 GB decoded).
fn png_bomb() -> Vec<u8> {
    fn crc32(bytes: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in bytes {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
            }
        }
        !crc
    }
    let mut chunk = b"IHDR".to_vec();
    chunk.extend_from_slice(&100_000u32.to_be_bytes());
    chunk.extend_from_slice(&100_000u32.to_be_bytes());
    chunk.extend_from_slice(&[8, 2, 0, 0, 0]);
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    png.extend_from_slice(&13u32.to_be_bytes());
    png.extend_from_slice(&chunk);
    png.extend_from_slice(&crc32(&chunk).to_be_bytes());
    png
}

#[test]
fn sec05_a_decompression_bomb_stops_at_the_header() {
    let scratch = Scratch::new("thumbs-bomb");
    let path = scratch.path.join("bomb.png");
    fs::write(&path, png_bomb()).unwrap();
    let started = std::time::Instant::now();
    assert_eq!(render(&path, ThumbSize::Small, None).unwrap_err(), ThumbError::Corrupt);
    assert!(started.elapsed().as_secs() < 2, "refused before allocating");
}
