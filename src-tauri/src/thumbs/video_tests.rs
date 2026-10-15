//! VID-01..05 (F13). The service is driven by a fake ffmpeg; one test runs the real one
//! when it is installed (it is on the Linux dev box; the Windows CI runner may not have it).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};

use image::{Rgb, RgbImage};
use rusqlite::{params, Connection};

use super::video::{cover_time, executable, locate_in, parse_probe, VideoError};
use super::*;
use crate::db::tests::Scratch;
use crate::db::MIGRATIONS;

const NOW: &str = "2026-10-15T12:00:00.000Z";

#[test]
fn vid01_probe_reads_size_duration_and_rotation() {
    let plain = r#"{"streams":[{"width":1920,"height":1080}],"format":{"duration":"12.3456"}}"#;
    assert_eq!(
        parse_probe(plain),
        Some(VideoProbe { duration_ms: Some(12346), width: Some(1920), height: Some(1080) })
    );
    let phone = r#"{"streams":[{"width":1920,"height":1080,"side_data_list":[{"side_data_type":"Display Matrix","rotation":-90}]}],"format":{"duration":"3.0"}}"#;
    assert_eq!(parse_probe(phone).unwrap().width, Some(1080));
    let old_tag = r#"{"streams":[{"width":640,"height":480,"tags":{"rotate":"270"}}],"format":{}}"#;
    assert_eq!(
        parse_probe(old_tag),
        Some(VideoProbe { duration_ms: None, width: Some(480), height: Some(640) })
    );
    let upside_down = r#"{"streams":[{"width":640,"height":480,"side_data_list":[{"rotation":180}]}]}"#;
    assert_eq!(parse_probe(upside_down).unwrap().width, Some(640));
    let odd = r#"{"streams":[{"width":0}],"format":{"duration":"N/A"}}"#;
    assert_eq!(parse_probe(odd), Some(VideoProbe::default()));
    assert_eq!(parse_probe(r#"{"streams":[],"format":{"duration":"1.0"}}"#), None);
    assert_eq!(parse_probe("not json"), None);
}

#[test]
fn vid01_cover_is_one_second_in_or_mid_clip() {
    assert_eq!(cover_time(None), 0);
    assert_eq!(cover_time(Some(600)), 300);
    assert_eq!(cover_time(Some(2000)), 1000);
    assert_eq!(cover_time(Some(3_600_000)), 1000);
}

#[test]
fn vid02_ffmpeg_needs_both_binaries_in_the_same_folder() {
    let scratch = Scratch::new("f13-locate");
    let half = scratch.path.join("half");
    let full = scratch.path.join("full");
    fs::create_dir_all(&half).unwrap();
    fs::create_dir_all(&full).unwrap();
    fs::write(half.join(executable("ffmpeg")), b"").unwrap();
    fs::write(full.join(executable("ffmpeg")), b"").unwrap();
    fs::write(full.join(executable("ffprobe")), b"").unwrap();
    let found = locate_in([scratch.path.join("absent"), half.clone(), full.clone()]).unwrap();
    assert_eq!(found.ffmpeg, full.join(executable("ffmpeg")));
    assert_eq!(found.ffprobe, full.join(executable("ffprobe")));
    assert_eq!(locate_in([half]), None);
}

/// Behaves by file name: `ok*` is a portrait phone clip, `broken*` fails the probe,
/// `tool*` cannot run ffmpeg, `audio*` has no video stream.
#[derive(Default)]
struct FakeTool {
    frames: Mutex<Vec<(String, u64, u32)>>,
}

fn name(path: &Path) -> String {
    path.file_name().unwrap().to_string_lossy().into_owned()
}

impl VideoTool for FakeTool {
    fn probe(&self, path: &Path) -> Result<VideoProbe, VideoError> {
        let name = name(path);
        if name.starts_with("broken") {
            Err(VideoError::Media(ThumbError::Corrupt))
        } else if name.starts_with("tool") {
            Err(VideoError::Tool)
        } else if name.starts_with("audio") {
            Err(VideoError::Media(ThumbError::Unsupported))
        } else {
            Ok(VideoProbe { duration_ms: Some(4000), width: Some(1080), height: Some(1920) })
        }
    }

    fn frame(&self, path: &Path, at_ms: u64, max_edge: u32) -> Result<Vec<u8>, VideoError> {
        self.frames.lock().unwrap().push((name(path), at_ms, max_edge));
        let (width, height) = (max_edge * 9 / 16, max_edge);
        let mut jpeg = Vec::new();
        RgbImage::from_pixel(width, height, Rgb([10, 20, 30]))
            .write_to(&mut std::io::Cursor::new(&mut jpeg), image::ImageFormat::Jpeg)
            .unwrap();
        Ok(jpeg)
    }
}

struct Library {
    _scratch: Scratch,
    root: PathBuf,
    connection: Option<Connection>,
}

impl Library {
    fn new(label: &str) -> Self {
        let scratch = Scratch::new(&format!("f13-{label}"));
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
        Self { _scratch: scratch, root, connection: Some(connection) }
    }

    fn connection(&self) -> &Connection {
        self.connection.as_ref().unwrap()
    }

    fn media(&self, relative: &str, media_type: &str, file: bool) -> i64 {
        if file {
            let path = self.root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, b"not really a video").unwrap();
        }
        self.connection()
            .execute(
                "INSERT INTO media_items (library_id, relative_path, filename, extension, media_type,
                   size_bytes, fs_modified_at, created_at, updated_at)
                 VALUES (1, ?1, ?1, 'mp4', ?2, 100, '2020-01-01T00:00:00.000Z', ?3, ?3)",
                params![relative, media_type, NOW],
            )
            .unwrap();
        self.connection().last_insert_rowid()
    }

    fn generate(&mut self, size: ThumbSize, tool: Option<Arc<dyn VideoTool>>) -> ThumbSummary {
        let connection = self.connection.take().unwrap();
        let (connection, summary) = generate_with(connection, &self.root, size, 2, tool, &mut |_, _| {});
        self.connection = Some(connection);
        summary.unwrap()
    }

    fn video_row(&self, id: i64) -> (Option<i64>, Option<i64>, Option<i64>) {
        self.connection()
            .query_row(
                "SELECT duration_ms, width, height FROM media_items WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap()
    }
}

#[test]
fn vid03_videos_get_a_cover_and_their_duration_and_size() {
    let mut library = Library::new("covers");
    let ok = library.media("Fotos/ok.mp4", "video", true);
    let broken = library.media("Fotos/broken.mov", "video", true);
    let audio = library.media("Fotos/audio.m4v", "video", true);
    let gone = library.media("Fotos/ok-gone.mp4", "video", false);
    let tool = Arc::new(FakeTool::default());

    let summary = library.generate(ThumbSize::Small, Some(tool.clone()));
    assert_eq!(summary.queued, 4);
    assert_eq!(
        (summary.generated, summary.videos, summary.corrupt, summary.unsupported, summary.unreadable),
        (1, 1, 1, 1, 1)
    );
    assert_eq!(summary.videos_skipped, 0);
    assert_eq!(*tool.frames.lock().unwrap(), [("ok.mp4".to_string(), 1000, 300)]);

    let jpeg = fs::read(thumb_path(&library.root, ok, ThumbSize::Small)).unwrap();
    let decoded = image::load_from_memory(&jpeg).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (168, 300));
    let (width, height): (u32, u32) = library
        .connection()
        .query_row(
            "SELECT width, height FROM thumbnails WHERE media_id = ?1 AND size = 'small'",
            [ok],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((width, height), (168, 300));
    assert_eq!(library.video_row(ok), (Some(4000), Some(1080), Some(1920)));
    assert_eq!(library.video_row(broken), (None, None, None));

    let reasons: Vec<(i64, String)> = library
        .connection()
        .prepare("SELECT media_id, reason FROM thumbnail_failures ORDER BY media_id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        reasons,
        [
            (broken, "corrupt_media".to_string()),
            (audio, "unsupported_format".to_string()),
            (gone, "unreadable".to_string())
        ]
    );

    // A larger size probes again and takes a bigger frame; the failures are not retried.
    let medium = library.generate(ThumbSize::Medium, Some(tool.clone()));
    assert_eq!((medium.queued, medium.generated), (1, 1));
    assert_eq!(tool.frames.lock().unwrap().last().unwrap().2, 1280);
}

#[test]
fn vid03_a_missing_ffmpeg_is_retried_not_recorded() {
    let mut library = Library::new("tool");
    let video = library.media("tool.mp4", "video", true);
    let summary = library.generate(ThumbSize::Small, Some(Arc::new(FakeTool::default())));
    assert_eq!((summary.queued, summary.tool_failed, summary.done()), (1, 1, 1));
    let failures: i64 = library
        .connection()
        .query_row("SELECT count(*) FROM thumbnail_failures", [], |row| row.get(0))
        .unwrap();
    assert_eq!(failures, 0);
    assert!(job_for(library.connection(), video, ThumbSize::Small, true).unwrap().is_some());
    let again = library.generate(ThumbSize::Small, Some(Arc::new(FakeTool::default())));
    assert_eq!(again.queued, 1);
}

#[test]
fn vid04_without_ffmpeg_videos_are_counted_and_photos_still_done() {
    let mut library = Library::new("no-ffmpeg");
    let video = library.media("clip.mp4", "video", true);
    let photo_path = library.root.join("a.png");
    RgbImage::from_pixel(40, 20, Rgb([1, 2, 3])).save(&photo_path).unwrap();
    library.media("a.png", "photo", false);
    let summary = library.generate(ThumbSize::Small, None);
    assert_eq!((summary.queued, summary.generated, summary.videos_skipped), (1, 1, 1));
    assert_eq!(pending(library.connection(), &library.root, ThumbSize::Small, true).unwrap().len(), 1);
    assert_eq!(pending(library.connection(), &library.root, ThumbSize::Small, false).unwrap().len(), 0);
    assert!(job_for(library.connection(), video, ThumbSize::Small, false).unwrap().is_none());
}

#[test]
fn vid04_a_video_request_jumps_the_queue_like_a_photo() {
    let library = Library::new("request");
    let video = library.media("ok.mp4", "video", true);
    let Library { _scratch, root, connection } = library;
    let tool: Arc<dyn VideoTool> = Arc::new(FakeTool::default());
    let service =
        ThumbService::start_with(connection.unwrap(), root.clone(), 1, DEFAULT_CACHE_LIMIT_BYTES, Some(tool));
    service.request(video, ThumbSize::Large);
    service.wait_idle(&mut |_, _| {});
    let (connection, summary) = service.shutdown();
    assert_eq!((summary.generated, summary.videos), (1, 1));
    let path = thumbnail_for(&connection, &root, video, ThumbSize::Large, NOW).unwrap();
    assert_eq!(path, Some(thumb_path(&root, video, ThumbSize::Large)));
}

/// VID-05: the real ffmpeg, when this machine has one.
#[test]
fn vid05_real_ffmpeg_probes_and_takes_a_frame() {
    let path = std::env::var_os("PATH")
        .map(|value| std::env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    let Some(tool) = locate_in(path) else {
        eprintln!("vid05: no ffmpeg on PATH; covered by the fake-tool tests");
        return;
    };
    let scratch = Scratch::new("f13-real");
    let clip = scratch.path.join("clip.mp4");
    let made = Command::new(&tool.ffmpeg)
        .args(["-v", "error", "-f", "lavfi", "-i", "testsrc=size=640x360:rate=10", "-t", "2"])
        .args(["-c:v", "mpeg4", "-pix_fmt", "yuv420p"])
        .arg(&clip)
        .status()
        .unwrap();
    assert!(made.success());

    let probe = tool.probe(&clip).unwrap();
    assert_eq!((probe.width, probe.height), (Some(640), Some(360)));
    let duration = probe.duration_ms.unwrap();
    assert!((1900..=2100).contains(&duration), "{duration}");

    let jpeg = tool.frame(&clip, cover_time(probe.duration_ms), 300).unwrap();
    let image = image::load_from_memory(&jpeg).unwrap();
    assert_eq!((image.width(), image.height()), (300, 169));
    // Small video: never upscaled.
    let full = tool.frame(&clip, 0, 2560).unwrap();
    assert_eq!(image::load_from_memory(&full).unwrap().width(), 640);
    // Seek past the end: falls back to the first frame.
    assert!(!tool.frame(&clip, 60_000, 300).unwrap().is_empty());

    let junk = scratch.path.join("junk.mp4");
    fs::write(&junk, b"definitely not a video").unwrap();
    assert!(matches!(tool.probe(&junk), Err(VideoError::Media(_))));
}
