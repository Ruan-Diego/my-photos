//! F13: cover frame and duration of a video through ffmpeg/ffprobe. The binaries are
//! looked up next to the app (the Tauri sidecar) and then on `PATH`; without them,
//! videos simply get no thumbnail.

use std::env;
use std::ffi::OsStr;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::ThumbError;

/// A video that takes longer than this to probe or to give a frame is treated as broken.
const TOOL_TIMEOUT: Duration = Duration::from_secs(60);

/// Seconds into the video for the cover: past a black first frame, inside short clips.
const COVER_AT_MS: u64 = 1000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct VideoProbe {
    pub duration_ms: Option<u64>,
    /// As displayed: a phone video recorded upright reports its rotation, and the two
    /// are swapped here.
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoError {
    /// The file is the problem; recorded in `thumbnail_failures`.
    Media(ThumbError),
    /// ffmpeg could not be run; nothing is recorded, so it is tried again next time.
    Tool,
}

/// What F13 needs from ffmpeg; tests swap in a fake.
pub trait VideoTool: Send + Sync {
    fn probe(&self, path: &Path) -> Result<VideoProbe, VideoError>;
    /// One JPEG frame at `at_ms`, fitted inside `max_edge` and never upscaled.
    fn frame(&self, path: &Path, at_ms: u64, max_edge: u32) -> Result<Vec<u8>, VideoError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ffmpeg {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

/// The folder of the running app first (where Tauri puts a sidecar), then `PATH`.
pub fn locate_ffmpeg() -> Option<Ffmpeg> {
    let beside_app = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let path = env::var_os("PATH")
        .map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .unwrap_or_default();
    locate_in(beside_app.into_iter().chain(path))
}

/// The first folder that has both `ffmpeg` and `ffprobe`.
pub(crate) fn locate_in(folders: impl IntoIterator<Item = PathBuf>) -> Option<Ffmpeg> {
    folders.into_iter().find_map(|folder| {
        let ffmpeg = folder.join(executable("ffmpeg"));
        let ffprobe = folder.join(executable("ffprobe"));
        (ffmpeg.is_file() && ffprobe.is_file()).then_some(Ffmpeg { ffmpeg, ffprobe })
    })
}

pub(crate) fn executable(name: &str) -> String {
    format!("{name}{}", env::consts::EXE_SUFFIX)
}

/// Where the cover is taken: one second in, or the middle of a shorter clip.
pub(crate) fn cover_time(duration_ms: Option<u64>) -> u64 {
    duration_ms.map_or(0, |duration| COVER_AT_MS.min(duration / 2))
}

/// `ffprobe -of json` output → probe. `None` when there is no video stream.
pub(crate) fn parse_probe(json: &str) -> Option<VideoProbe> {
    let value: Value = serde_json::from_str(json).ok()?;
    let stream = value.get("streams")?.as_array()?.first()?;
    let dimension = |key: &str| {
        stream
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .filter(|&value| value > 0)
    };
    let (mut width, mut height) = (dimension("width"), dimension("height"));
    if quarter_turn(stream) {
        std::mem::swap(&mut width, &mut height);
    }
    let duration_ms = value
        .get("format")
        .and_then(|format| format.get("duration"))
        .and_then(Value::as_str)
        .and_then(|text| text.trim().parse::<f64>().ok())
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0)
        .map(|seconds| (seconds * 1000.0).round() as u64);
    Some(VideoProbe {
        duration_ms,
        width,
        height,
    })
}

/// Rotation from the display matrix (new ffmpeg) or the `rotate` tag (old files).
fn quarter_turn(stream: &Value) -> bool {
    let from_side_data = stream
        .get("side_data_list")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|data| data.get("rotation"))
        .find_map(|rotation| {
            rotation
                .as_i64()
                .or_else(|| rotation.as_f64().map(|value| value.round() as i64))
        });
    let from_tag = || {
        stream
            .get("tags")
            .and_then(|tags| tags.get("rotate"))
            .and_then(Value::as_str)
            .and_then(|text| text.trim().parse::<i64>().ok())
    };
    from_side_data
        .or_else(from_tag)
        .is_some_and(|degrees| degrees.rem_euclid(180) == 90)
}

impl VideoTool for Ffmpeg {
    fn probe(&self, path: &Path) -> Result<VideoProbe, VideoError> {
        let mut command = Command::new(&self.ffprobe);
        command
            .args(["-v", "error", "-select_streams", "v:0", "-show_entries"])
            .arg("stream=width,height:stream_tags=rotate:stream_side_data=rotation:format=duration")
            .args(["-of", "json"])
            .arg(path);
        let output = run(command)?;
        let text = String::from_utf8_lossy(&output);
        parse_probe(&text).ok_or(VideoError::Media(ThumbError::Unsupported))
    }

    fn frame(&self, path: &Path, at_ms: u64, max_edge: u32) -> Result<Vec<u8>, VideoError> {
        let at = || {
            let mut command = Command::new(&self.ffmpeg);
            command.args(["-v", "error", "-nostdin", "-ss"]);
            command.arg(format!("{}.{:03}", at_ms / 1000, at_ms % 1000));
            command
        };
        let jpeg = run(frame_command(at(), path, max_edge))?;
        if !jpeg.is_empty() {
            return Ok(jpeg);
        }
        // A wrong duration puts the seek past the end: take the first frame instead.
        let mut first = Command::new(&self.ffmpeg);
        first.args(["-v", "error", "-nostdin"]);
        let jpeg = run(frame_command(first, path, max_edge))?;
        if jpeg.is_empty() {
            Err(VideoError::Media(ThumbError::Corrupt))
        } else {
            Ok(jpeg)
        }
    }
}

fn frame_command(mut command: Command, path: &Path, max_edge: u32) -> Command {
    command
        .arg("-i")
        .arg(path)
        .args(["-frames:v", "1", "-vf"])
        .arg(format!(
            "scale='min({max_edge},iw)':'min({max_edge},ih)':force_original_aspect_ratio=decrease"
        ))
        .args(["-q:v", "3", "-f", "image2pipe", "-c:v", "mjpeg", "-"]);
    command
}

/// Runs a tool and returns its stdout. Non-zero exit → the file is broken; could not
/// start → `Tool`; too slow → killed and treated as broken.
fn run(mut command: Command) -> Result<Vec<u8>, VideoError> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    hide_window(&mut command);
    let mut child = command.spawn().map_err(|_| VideoError::Tool)?;
    let mut stdout = child.stdout.take().ok_or(VideoError::Tool)?;
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let status = wait(&mut child)?;
    let bytes = reader
        .join()
        .map_err(|_| VideoError::Tool)?
        .map_err(|_| VideoError::Tool)?;
    if status {
        Ok(bytes)
    } else {
        Err(VideoError::Media(ThumbError::Corrupt))
    }
}

/// True when the tool exited with success.
fn wait(child: &mut Child) -> Result<bool, VideoError> {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status.success()),
            Ok(None) if started.elapsed() > TOOL_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(VideoError::Media(ThumbError::Corrupt));
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(_) => return Err(VideoError::Tool),
        }
    }
}

#[cfg(windows)]
fn hide_window(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    // CREATE_NO_WINDOW: no console flashing while the app works.
    command.creation_flags(0x0800_0000);
}

#[cfg(not(windows))]
fn hide_window(_command: &mut Command) {}

/// Shown in the summary when no ffmpeg was found.
pub(crate) fn describe(tool: Option<&Ffmpeg>) -> Option<String> {
    tool.map(|tool| {
        tool.ffmpeg
            .parent()
            .map(Path::as_os_str)
            .unwrap_or_else(|| OsStr::new(""))
            .to_string_lossy()
            .into_owned()
    })
}
