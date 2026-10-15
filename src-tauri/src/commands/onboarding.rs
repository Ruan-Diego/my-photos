//! F16: what the welcome screen shows (artboard `Main`) and the first indexing run.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::index::{index_default_library, IndexSummary, Progress};
use crate::takeout::takeout_export;
use crate::volume::{locate_library, volume_info};

pub const PROGRESS_EVENT: &str = "indexing://progress";
pub const FINISHED_EVENT: &str = "indexing://finished";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryFolder {
    /// `E:\`
    pub root_path: String,
    pub volume_label: Option<String>,
    /// Takeout exports at the root: `takeout-<stamp>-N-NNN` parts of one export count once;
    /// a plain `Takeout` folder counts as one.
    pub takeout_exports: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexingFinished {
    pub summary: Option<IndexSummary>,
    /// Error code from F05/F10/F11 when the run failed.
    pub error: Option<String>,
}

/// One run at a time.
#[derive(Default)]
pub struct IndexingState(AtomicBool);

pub(crate) fn count_exports<'a>(names: impl IntoIterator<Item = &'a str>) -> u32 {
    let mut exports: Vec<String> = Vec::new();
    for name in names {
        let export = if name.eq_ignore_ascii_case("takeout") {
            "takeout".to_string()
        } else if let Some(base) = takeout_export(name) {
            base.to_ascii_lowercase()
        } else {
            continue;
        };
        if !exports.contains(&export) {
            exports.push(export);
        }
    }
    exports.len() as u32
}

fn exports_at(root: &Path) -> u32 {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };
    let names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    count_exports(names.iter().map(String::as_str))
}

#[tauri::command]
pub fn library_folder() -> Result<LibraryFolder, String> {
    let root = locate_library().map_err(|error| error.message().to_string())?;
    let path = Path::new(&root);
    Ok(LibraryFolder {
        volume_label: volume_info(path).and_then(|volume| volume.label),
        takeout_exports: exports_at(path),
        root_path: root,
    })
}

/// Starts F10 on the library disk in the background. Progress and the end arrive as
/// events; a second call while one runs is refused.
#[tauri::command]
pub fn start_indexing(app: AppHandle, state: State<'_, IndexingState>) -> Result<(), String> {
    if state.0.swap(true, Ordering::SeqCst) {
        return Err("a indexação já está em andamento".into());
    }
    thread::spawn(move || {
        let result = index_default_library(&mut |progress: Progress| {
            let _ = app.emit(PROGRESS_EVENT, progress);
        });
        let finished = match result {
            Ok(summary) => IndexingFinished {
                summary: Some(summary),
                error: None,
            },
            Err(code) => IndexingFinished {
                summary: None,
                error: Some(code.to_string()),
            },
        };
        if let Some(state) = tauri::Manager::try_state::<IndexingState>(&app) {
            state.0.store(false, Ordering::SeqCst);
        }
        let _ = app.emit(FINISHED_EVENT, finished);
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::count_exports;

    #[test]
    fn onb01_parts_of_one_export_count_once() {
        assert_eq!(
            count_exports([
                "takeout-20260921T183943Z-1-001",
                "takeout-20260921T183943Z-1-002",
                "Takeout-20250101T000000Z-1-001",
                "Takeout",
                "Fotos antigas",
                "takeout-sem-numero",
                "$RECYCLE.BIN",
            ]),
            3
        );
        assert_eq!(count_exports(["takeout", "TAKEOUT"]), 1);
        assert_eq!(count_exports([]), 0);
    }
}
