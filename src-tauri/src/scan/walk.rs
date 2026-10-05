use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::time::iso_utc;
use crate::takeout::{classify, is_system_dir, FileRole};

const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "mkv", "avi", "3gp", "webm", "m4v"];

/// The database lives here (AD-011); it is never part of the library.
const APP_FOLDER: &str = ".myphotos";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryKind {
    Dir,
    File,
    /// Link, junction, hidden+system folder or anything that is neither file nor folder.
    Skip,
    /// Listed but its metadata could not be read.
    Unreadable,
}

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    /// `Err` holds a lossy rendering of a name that is not valid Unicode.
    pub name: Result<String, String>,
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub created: Option<SystemTime>,
}

/// Everything the scanner does to the disk. Tests wrap `RealFs` to inject failures.
pub(crate) trait FileSystem {
    fn list(&self, dir: &Path) -> io::Result<Vec<Entry>>;
    fn open(&self, file: &Path) -> io::Result<Box<dyn Read>>;
}

pub(crate) struct RealFs;

impl FileSystem for RealFs {
    fn list(&self, dir: &Path) -> io::Result<Vec<Entry>> {
        let mut entries = Vec::new();
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|raw| raw.to_string_lossy().into_owned());
            let mut listed = Entry {
                name,
                kind: EntryKind::Skip,
                size: 0,
                modified: None,
                created: None,
            };
            // DirEntry::file_type and metadata do not follow links.
            match entry.file_type() {
                Err(_) => listed.kind = EntryKind::Unreadable,
                Ok(file_type) if file_type.is_symlink() => {}
                Ok(file_type) if file_type.is_dir() => {
                    if !is_system_dir(&entry) {
                        listed.kind = EntryKind::Dir;
                    }
                }
                Ok(file_type) if file_type.is_file() => match entry.metadata() {
                    Ok(meta) => {
                        listed.kind = EntryKind::File;
                        listed.size = meta.len();
                        listed.modified = meta.modified().ok();
                        listed.created = meta.created().ok();
                    }
                    Err(_) => listed.kind = EntryKind::Unreadable,
                },
                Ok(_) => {}
            }
            entries.push(listed);
        }
        Ok(entries)
    }

    fn open(&self, file: &Path) -> io::Result<Box<dyn Read>> {
        Ok(Box::new(File::open(file)?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileFact {
    pub relative_path: String,
    pub filename: String,
    pub extension: String,
    pub media_type: &'static str,
    pub size: u64,
    pub modified: Option<String>,
    pub created: Option<String>,
}

#[derive(Debug, Default)]
pub(crate) struct Walk {
    /// Physical folders under the root, sorted, root excluded.
    pub folders: Vec<String>,
    /// Media files, sorted by relative path.
    pub files: Vec<FileFact>,
    /// Paths whose contents could not be read; known rows under them are left alone.
    pub protected: Vec<String>,
    /// `(relative_path, error_type)` for `indexing_errors`.
    pub errors: Vec<(String, &'static str)>,
}

/// The root stopped being a folder while it was being listed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RootVanished;

pub(crate) fn walk(root: &Path, fs: &dyn FileSystem) -> Result<Walk, RootVanished> {
    let mut found = Walk::default();
    visit(root, root.to_path_buf(), "", fs, &mut found)?;
    found.folders.sort();
    found
        .files
        .sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(found)
}

fn visit(
    root: &Path,
    dir: PathBuf,
    relative: &str,
    fs: &dyn FileSystem,
    found: &mut Walk,
) -> Result<(), RootVanished> {
    let mut entries = match fs.list(&dir) {
        Ok(entries) => entries,
        Err(_) => {
            if !root.is_dir() {
                return Err(RootVanished);
            }
            found.protected.push(relative.to_string());
            found
                .errors
                .push((relative.to_string(), "unreadable_directory"));
            return Ok(());
        }
    };
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    let mut seen = HashSet::new();
    for entry in entries {
        let name = match entry.name {
            Ok(name) => name,
            Err(lossy) => {
                found.errors.push((join(relative, &lossy), "invalid_name"));
                continue;
            }
        };
        let path = join(relative, &name);
        // A case-sensitive folder can hold `A` and `a`, but the database compares paths
        // NOCASE (NTFS semantics). Among the folders and media that would be recorded, the
        // first in order wins; the twin and everything in it is skipped.
        let mut is_twin = |path: &str| {
            let twin = !seen.insert(name.to_ascii_lowercase());
            if twin {
                found.errors.push((path.to_string(), "invalid_name"));
            }
            twin
        };
        match entry.kind {
            EntryKind::Skip => {}
            EntryKind::Unreadable => {
                // Protect whatever it was; report it only if it looks like media (SCAN-02 AC5).
                found.protected.push(path.clone());
                if matches!(classify(&name), FileRole::Media { .. }) {
                    found.errors.push((path, "unreadable"));
                }
            }
            EntryKind::Dir => {
                if relative.is_empty() && name.eq_ignore_ascii_case(APP_FOLDER) {
                    continue;
                }
                if is_twin(&path) {
                    continue;
                }
                found.folders.push(path.clone());
                visit(root, dir.join(&name), &path, fs, found)?;
            }
            EntryKind::File => {
                let FileRole::Media { extension } = classify(&name) else {
                    continue;
                };
                if is_twin(&path) {
                    continue;
                }
                let media_type = if VIDEO_EXTENSIONS.contains(&extension.as_str()) {
                    "video"
                } else {
                    "photo"
                };
                found.files.push(FileFact {
                    relative_path: path,
                    filename: name,
                    extension,
                    media_type,
                    size: entry.size,
                    modified: entry.modified.and_then(iso_utc),
                    created: entry.created.and_then(iso_utc),
                });
            }
        }
    }
    Ok(())
}

pub(crate) fn join(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.to_string()
    } else {
        format!("{directory}/{name}")
    }
}

/// `a/b.jpg` under root → `root\a\b.jpg`.
pub(crate) fn absolute(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{self, Read};
    use std::path::Path;

    use super::{walk, Entry, EntryKind, FileSystem, RealFs, RootVanished};
    use crate::db::tests::Scratch;

    fn write(root: &Path, relative: &str, bytes: &[u8]) {
        let path = super::absolute(root, relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn lists_physical_folders_and_media_only() {
        let scratch = Scratch::new("walk");
        let root = &scratch.path;
        write(
            root,
            "Takeout-x/takeout-20260921T183943Z-1-001/Fotos/a.JPG",
            b"a",
        );
        write(
            root,
            "Takeout-x/takeout-20260921T183943Z-1-002/Fotos/a.JPG.json",
            b"{}",
        );
        write(
            root,
            "Takeout-x/takeout-20260921T183943Z-1-002/Fotos/b.mp4",
            b"bb",
        );
        write(root, "notes.txt", b"n");
        write(root, "archive.zip", b"z");
        write(root, "noext", b"z");
        fs::create_dir_all(root.join("vazia")).unwrap();

        let found = walk(root, &RealFs).unwrap();
        assert_eq!(
            found.folders,
            [
                "Takeout-x",
                "Takeout-x/takeout-20260921T183943Z-1-001",
                "Takeout-x/takeout-20260921T183943Z-1-001/Fotos",
                "Takeout-x/takeout-20260921T183943Z-1-002",
                "Takeout-x/takeout-20260921T183943Z-1-002/Fotos",
                "vazia",
            ]
        );
        let files: Vec<_> = found
            .files
            .iter()
            .map(|f| {
                (
                    f.relative_path.as_str(),
                    f.extension.as_str(),
                    f.media_type,
                    f.size,
                )
            })
            .collect();
        assert_eq!(
            files,
            [
                (
                    "Takeout-x/takeout-20260921T183943Z-1-001/Fotos/a.JPG",
                    "jpg",
                    "photo",
                    1
                ),
                (
                    "Takeout-x/takeout-20260921T183943Z-1-002/Fotos/b.mp4",
                    "mp4",
                    "video",
                    2
                ),
            ]
        );
        assert_eq!(found.files[0].filename, "a.JPG");
        assert!(found.files[0].modified.as_deref().unwrap().ends_with('Z'));
        assert!(found.errors.is_empty());
        assert!(found.protected.is_empty());
    }

    #[test]
    fn app_folder_is_skipped_only_at_the_root() {
        let scratch = Scratch::new("walk-app");
        let root = &scratch.path;
        write(root, ".MyPhotos/trash/x.jpg", b"x");
        write(root, "sub/.myphotos/y.jpg", b"y");
        let found = walk(root, &RealFs).unwrap();
        assert_eq!(found.folders, ["sub", "sub/.myphotos"]);
        assert_eq!(found.files.len(), 1);
        assert_eq!(found.files[0].relative_path, "sub/.myphotos/y.jpg");
    }

    #[cfg(unix)]
    #[test]
    fn links_are_not_followed() {
        let scratch = Scratch::new("walk-link");
        write(&scratch.path, "outside/secret.jpg", b"s");
        fs::create_dir_all(scratch.path.join("root")).unwrap();
        std::os::unix::fs::symlink(scratch.path.join("outside"), scratch.path.join("root/link"))
            .unwrap();
        std::os::unix::fs::symlink(
            scratch.path.join("outside/secret.jpg"),
            scratch.path.join("root/file.jpg"),
        )
        .unwrap();
        let found = walk(&scratch.path.join("root"), &RealFs).unwrap();
        assert!(found.folders.is_empty());
        assert!(found.files.is_empty());
    }

    #[cfg(windows)]
    #[test]
    fn hidden_system_folders_are_skipped_and_hidden_only_folders_are_not() {
        let scratch = Scratch::new("walk-system");
        write(&scratch.path, "sistema/a.jpg", b"a");
        write(&scratch.path, "oculta/b.jpg", b"b");
        let attrib = |args: &[&str], dir: &str| {
            let status = std::process::Command::new("attrib")
                .args(args)
                .arg(scratch.path.join(dir))
                .status()
                .unwrap();
            assert!(status.success());
        };
        attrib(&["+h", "+s"], "sistema");
        attrib(&["+h"], "oculta");
        let found = walk(&scratch.path, &RealFs).unwrap();
        attrib(&["-h", "-s"], "sistema");
        assert_eq!(found.folders, ["oculta"]);
        assert_eq!(found.files.len(), 1);
        assert_eq!(found.files[0].relative_path, "oculta/b.jpg");
    }

    #[cfg(windows)]
    #[test]
    fn junctions_are_not_followed() {
        let scratch = Scratch::new("walk-junction");
        write(&scratch.path, "outside/secret.jpg", b"s");
        fs::create_dir_all(scratch.path.join("root")).unwrap();
        let status = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(scratch.path.join("root").join("link"))
            .arg(scratch.path.join("outside"))
            .status()
            .unwrap();
        assert!(status.success());
        let found = walk(&scratch.path.join("root"), &RealFs).unwrap();
        assert!(found.folders.is_empty());
        assert!(found.files.is_empty());
    }

    struct Scripted<'a> {
        real: RealFs,
        fail_dir: Option<&'a str>,
        extra: Vec<Entry>,
        vanish: Option<&'a Path>,
    }

    impl FileSystem for Scripted<'_> {
        fn list(&self, dir: &Path) -> io::Result<Vec<Entry>> {
            if self.fail_dir.is_some_and(|name| dir.ends_with(name)) {
                if let Some(root) = self.vanish {
                    fs::remove_dir_all(root).unwrap();
                }
                return Err(io::Error::other("denied"));
            }
            let mut entries = self.real.list(dir)?;
            entries.extend(self.extra.iter().cloned());
            Ok(entries)
        }
        fn open(&self, file: &Path) -> io::Result<Box<dyn Read>> {
            self.real.open(file)
        }
    }

    #[test]
    fn unreadable_folder_is_reported_and_protected_and_the_rest_is_listed() {
        let scratch = Scratch::new("walk-denied");
        write(&scratch.path, "bloqueada/a.jpg", b"a");
        write(&scratch.path, "livre/b.jpg", b"b");
        let fs = Scripted {
            real: RealFs,
            fail_dir: Some("bloqueada"),
            extra: Vec::new(),
            vanish: None,
        };
        let found = walk(&scratch.path, &fs).unwrap();
        assert_eq!(found.folders, ["bloqueada", "livre"]);
        assert_eq!(found.files.len(), 1);
        assert_eq!(found.files[0].relative_path, "livre/b.jpg");
        assert_eq!(found.protected, ["bloqueada"]);
        assert_eq!(
            found.errors,
            [("bloqueada".to_string(), "unreadable_directory")]
        );
    }

    #[test]
    fn invalid_names_and_unreadable_entries_become_errors() {
        let scratch = Scratch::new("walk-names");
        fs::create_dir_all(&scratch.path).unwrap();
        let entry = |name: Result<String, String>, kind| Entry {
            name,
            kind,
            size: 1,
            modified: None,
            created: None,
        };
        let fs = Scripted {
            real: RealFs,
            fail_dir: None,
            extra: vec![
                entry(Err("bad\u{fffd}.jpg".to_string()), EntryKind::File),
                entry(Ok("locked.jpg".to_string()), EntryKind::Unreadable),
                entry(Ok("$RECYCLE.BIN".to_string()), EntryKind::Skip),
                entry(Ok("locked.json".to_string()), EntryKind::Unreadable),
            ],
            vanish: None,
        };
        let found = walk(&scratch.path, &fs).unwrap();
        assert!(found.files.is_empty());
        assert!(found.folders.is_empty());
        assert_eq!(found.protected, ["locked.jpg", "locked.json"]);
        assert_eq!(
            found.errors,
            [
                ("locked.jpg".to_string(), "unreadable"),
                ("bad\u{fffd}.jpg".to_string(), "invalid_name"),
            ]
        );
    }

    #[test]
    fn root_that_vanishes_while_listing_aborts() {
        let scratch = Scratch::new("walk-vanish");
        let root = scratch.path.join("root");
        write(&root, "a/x.jpg", b"x");
        let fs = Scripted {
            real: RealFs,
            fail_dir: Some("a"),
            extra: Vec::new(),
            vanish: Some(&root),
        };
        assert_eq!(walk(&root, &fs).unwrap_err(), RootVanished);
    }
}
