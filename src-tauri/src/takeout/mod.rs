// Reachable from later features. The shell does not call discovery yet.
#![allow(dead_code)]

mod classify;
mod matching;
mod model;
mod parse;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use classify::{classify, FileRole};
use matching::match_directory;
use parse::{parse_album_json, parse_media_json};

#[allow(unused_imports)]
pub use model::{
    AlbumHit, AlbumMetadata, Confidence, DiscoveryReport, MediaHit, MediaMetadata, Problem,
    ProblemKind, UnmatchedReason,
};

pub fn discover(root: &Path) -> Result<DiscoveryReport, String> {
    discover_with(root, |path| fs::read(path).map_err(|error| error.to_string()))
}

pub(crate) fn discover_with(
    root: &Path,
    read_json: impl Fn(&Path) -> Result<Vec<u8>, String>,
) -> Result<DiscoveryReport, String> {
    let metadata = fs::symlink_metadata(root).map_err(|error| error.to_string())?;
    if !metadata.is_dir() {
        return Err(format!("{} não é um diretório", root.display()));
    }

    let mut files = BTreeMap::new();
    let mut problems: Vec<Problem> = Vec::new();
    read_tree(root, "", &mut files, &mut problems)?;

    // Takeout splits one export into takeout-<stamp>-N-NNN folders, and a sidecar can land
    // in a different part than its media. Match inside the logical folder (part suffix
    // dropped) and keep reporting the physical paths.
    let mut groups: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for (directory, names) in &files {
        let entries = groups.entry(logical_dir(directory)).or_default();
        for name in names {
            entries.push((directory.clone(), name.clone()));
        }
    }

    let mut media = Vec::new();
    let mut albums = Vec::new();

    for entries in groups.values() {
        let names: Vec<String> = entries.iter().map(|(_, name)| name.clone()).collect();
        // match_directory returns one item per media name, in input order.
        let media_dirs = entries
            .iter()
            .filter(|(_, name)| matches!(classify(name), FileRole::Media { .. }))
            .map(|(directory, _)| directory.as_str());
        let json_dir = |json_name: &str| {
            entries
                .iter()
                .find(|(_, name)| name.eq_ignore_ascii_case(json_name))
                .map(|(directory, _)| directory.as_str())
                .unwrap_or_default()
        };
        let matches = match_directory(&names);
        let mut consumed = Vec::new();
        let mut parsed: BTreeMap<String, Result<MediaMetadata, ProblemKind>> = BTreeMap::new();

        for (item, directory) in matches.into_iter().zip(media_dirs) {
            let relative_path = join_relative(directory, &item.media_name);
            let Some(json_name) = item.json_name else {
                media.push(MediaHit {
                    relative_path,
                    confidence: item.confidence,
                    json_relative_path: None,
                    metadata: None,
                    reason: item.reason,
                });
                continue;
            };

            let json_relative = join_relative(json_dir(&json_name), &json_name);
            if !consumed
                .iter()
                .any(|used: &String| used.eq_ignore_ascii_case(&json_name))
            {
                consumed.push(json_name);
            }
            let outcome = if let Some(outcome) = parsed.get(&json_relative) {
                outcome.clone()
            } else {
                let outcome = read_media_json(&read_json, root, &json_relative);
                parsed.insert(json_relative.clone(), outcome.clone());
                outcome
            };
            match outcome {
                Ok(metadata) => media.push(MediaHit {
                    relative_path,
                    confidence: item.confidence,
                    json_relative_path: Some(json_relative),
                    metadata: Some(metadata),
                    reason: None,
                }),
                Err(kind) => {
                    if !problems.iter().any(|problem| {
                        problem.relative_path == json_relative && problem.kind == kind
                    }) {
                        problems.push(Problem {
                            relative_path: json_relative,
                            kind,
                        });
                    }
                    media.push(unmatched(relative_path));
                }
            }
        }

        for (directory, name) in entries {
            if classify(name) != FileRole::JsonSidecar {
                continue;
            }
            if consumed
                .iter()
                .any(|used| used.eq_ignore_ascii_case(name))
            {
                continue;
            }
            problems.push(Problem {
                relative_path: join_relative(directory, name),
                kind: ProblemKind::OrphanJson,
            });
        }

        for (directory, name) in entries {
            if classify(name) != FileRole::AlbumMetadata {
                continue;
            }
            let relative_path = join_relative(directory, name);
            match read_json(&absolute_from_relative(root, &relative_path)) {
                Err(_) => problems.push(Problem {
                    relative_path,
                    kind: ProblemKind::Unreadable,
                }),
                Ok(bytes) => match parse_album_json(&bytes) {
                    Err(()) => problems.push(Problem {
                        relative_path,
                        kind: ProblemKind::InvalidJson,
                    }),
                    Ok(metadata) => albums.push(AlbumHit {
                        relative_path,
                        metadata,
                    }),
                },
            }
        }
    }

    media.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    albums.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    problems.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(DiscoveryReport {
        media,
        albums,
        problems,
    })
}

fn read_media_json(
    read_json: &impl Fn(&Path) -> Result<Vec<u8>, String>,
    root: &Path,
    relative: &str,
) -> Result<MediaMetadata, ProblemKind> {
    let bytes = read_json(&absolute_from_relative(root, relative))
        .map_err(|_| ProblemKind::Unreadable)?;
    parse_media_json(&bytes).map_err(|_| ProblemKind::InvalidJson)
}

fn unmatched(relative_path: String) -> MediaHit {
    MediaHit {
        relative_path,
        confidence: Confidence::None,
        json_relative_path: None,
        metadata: None,
        reason: None,
    }
}

fn read_tree(
    directory: &Path,
    relative: &str,
    files: &mut BTreeMap<String, Vec<String>>,
    problems: &mut Vec<Problem>,
) -> Result<(), String> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if relative.is_empty() => return Err(error.to_string()),
        Err(_) => {
            problems.push(Problem {
                relative_path: relative.to_string(),
                kind: ProblemKind::UnreadableDirectory,
            });
            return Ok(());
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) if relative.is_empty() => return Err(error.to_string()),
            Err(_) => {
                problems.push(Problem {
                    relative_path: relative.to_string(),
                    kind: ProblemKind::UnreadableDirectory,
                });
                continue;
            }
        };
        let name = match entry.file_name().into_string() {
            Ok(name) => name,
            Err(raw) => {
                problems.push(Problem {
                    relative_path: join_relative(relative, &raw.to_string_lossy()),
                    kind: ProblemKind::InvalidName,
                });
                continue;
            }
        };
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(error) if relative.is_empty() => return Err(error.to_string()),
            Err(_) => {
                problems.push(Problem {
                    relative_path: join_relative(relative, &name),
                    kind: ProblemKind::Unreadable,
                });
                continue;
            }
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if is_system_dir(&entry) {
                continue;
            }
            read_tree(&entry.path(), &join_relative(relative, &name), files, problems)?;
        } else if file_type.is_file() {
            files.entry(relative.to_string()).or_default().push(name);
        }
    }
    Ok(())
}

fn logical_dir(directory: &str) -> String {
    directory
        .split('/')
        .map(|part| takeout_export(part).unwrap_or(part))
        .collect::<Vec<_>>()
        .join("/")
}

/// `takeout-20260921T183943Z-1-002` → `takeout-20260921T183943Z`.
fn takeout_export(component: &str) -> Option<&str> {
    let prefix = component.get(..8)?;
    if !prefix.eq_ignore_ascii_case("takeout-") {
        return None;
    }
    let mut pieces = component.rsplitn(3, '-');
    let part = pieces.next()?;
    let archive = pieces.next()?;
    let base = pieces.next()?;
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if base.len() <= 8 || !digits(part) || !digits(archive) {
        return None;
    }
    Some(base)
}

/// `$RECYCLE.BIN` and `System Volume Information` are hidden system folders at a volume
/// root; listing them fails and they never hold the library.
fn is_system_dir(entry: &fs::DirEntry) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const HIDDEN_AND_SYSTEM: u32 = 0x2 | 0x4;
        entry
            .metadata()
            .is_ok_and(|meta| meta.file_attributes() & HIDDEN_AND_SYSTEM == HIDDEN_AND_SYSTEM)
    }
    #[cfg(not(windows))]
    {
        let _ = entry;
        false
    }
}

fn join_relative(directory: &str, name: &str) -> String {
    if directory.is_empty() {
        name.to_string()
    } else {
        format!("{directory}/{name}")
    }
}

fn absolute_from_relative(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        path.push(part);
    }
    path
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{discover, discover_with};
    use crate::takeout::model::{Confidence, ProblemKind, UnmatchedReason};

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join("takeout")
            .join(name)
    }

    fn hit<'a>(
        report: &'a crate::takeout::model::DiscoveryReport,
        relative_path: &str,
    ) -> &'a crate::takeout::model::MediaHit {
        report
            .media
            .iter()
            .find(|item| item.relative_path == relative_path)
            .unwrap_or_else(|| panic!("missing {relative_path}"))
    }

    #[test]
    fn td01_exact_sidecar_in_the_same_folder() {
        let report = discover(&fixture("td01")).expect("directory");
        let media = hit(&report, "IMG_20220810_143700.jpg");
        assert_eq!(media.confidence, Confidence::Exact);
        assert_eq!(
            media.json_relative_path.as_deref(),
            Some("IMG_20220810_143700.jpg.json")
        );
        assert!(media.metadata.is_some());
        assert_eq!(media.reason, None);
    }

    #[test]
    fn td02_case_difference_is_still_exact() {
        let report = discover(&fixture("td02")).expect("directory");
        let media = hit(&report, "IMG_20220810_143700.JPG");
        assert_eq!(media.confidence, Confidence::Exact);
        assert_eq!(
            media.json_relative_path.as_deref(),
            Some("IMG_20220810_143700.jpg.json")
        );
    }

    #[test]
    fn td03_stem_json_is_exact() {
        let report = discover(&fixture("td03")).expect("directory");
        let media = hit(&report, "foto.jpg");
        assert_eq!(media.confidence, Confidence::Exact);
        assert_eq!(media.json_relative_path.as_deref(), Some("foto.json"));
    }

    #[test]
    fn td04_supplemental_metadata_is_heuristic() {
        let report = discover(&fixture("td04")).expect("directory");
        let media = hit(&report, "foto.jpg");
        assert_eq!(media.confidence, Confidence::Heuristic);
        assert_eq!(
            media.json_relative_path.as_deref(),
            Some("foto.jpg.supplemental-metadata.json")
        );
    }

    #[test]
    fn td05_truncated_name_is_heuristic() {
        let report = discover(&fixture("td05")).expect("directory");
        let media = hit(&report, "foto_com_nome_longo_demais_para_o_takeout_001.jpg");
        assert_eq!(media.confidence, Confidence::Heuristic);
        assert_eq!(
            media.json_relative_path.as_deref(),
            Some("foto_com_nome_longo_demais_para_o_takeout_001..json")
        );
    }

    #[test]
    fn td06_duplicate_marker_sidecar_is_heuristic() {
        let report = discover(&fixture("td06")).expect("directory");
        let media = hit(&report, "camphoto_1271212614(1).jpg");
        assert_eq!(media.confidence, Confidence::Heuristic);
        assert_eq!(
            media.json_relative_path.as_deref(),
            Some("camphoto_1271212614.jpg.supplemental-metadata(1).json")
        );
    }

    #[test]
    fn td07_edited_reuses_original_json() {
        let report = discover(&fixture("td07")).expect("directory");
        let original = hit(&report, "foto.jpg");
        let edited = hit(&report, "foto-edited.jpg");
        assert_eq!(original.confidence, Confidence::Exact);
        assert_eq!(edited.confidence, Confidence::Heuristic);
        assert_ne!(edited.confidence, Confidence::Exact);
        assert_eq!(original.json_relative_path.as_deref(), Some("foto.jpg.json"));
        assert_eq!(edited.json_relative_path.as_deref(), Some("foto.jpg.json"));
        assert_eq!(
            original.metadata.as_ref().and_then(|item| item.taken_at_unix),
            Some(1660142220)
        );
        assert_eq!(
            edited.metadata.as_ref().and_then(|item| item.taken_at_unix),
            Some(1660142220)
        );
    }

    #[test]
    fn td08_shared_truncated_json_is_ambiguous() {
        let prefix = "a".repeat(46);
        let report = discover(&fixture("td08")).expect("directory");
        for name in [format!("{prefix}1.jpg"), format!("{prefix}2.jpg")] {
            let media = hit(&report, &name);
            assert_eq!(media.confidence, Confidence::None);
            assert_eq!(media.json_relative_path, None);
            assert_eq!(media.reason, Some(UnmatchedReason::Ambiguous));
        }
        assert!(report.problems.iter().any(|problem| {
            problem.relative_path == format!("{prefix}.json")
                && problem.kind == ProblemKind::OrphanJson
        }));
    }

    #[test]
    fn td09_json_in_the_parent_folder_does_not_match() {
        let report = discover(&fixture("td09")).expect("directory");
        let media = hit(&report, "child/foto.jpg");
        assert_eq!(media.confidence, Confidence::None);
        assert_eq!(media.json_relative_path, None);
        assert_eq!(media.reason, None);
    }

    #[test]
    fn td10_second_discover_matches_the_first_in_path_order() {
        let root = fixture("td10");
        let first = discover(&root).expect("directory");
        let second = discover(&root).expect("directory");
        assert_eq!(first, second);
        let paths: Vec<&str> = first
            .media
            .iter()
            .map(|item| item.relative_path.as_str())
            .collect();
        assert_eq!(paths, vec!["a/a.jpg", "b.jpg"]);
        assert!(paths.iter().all(|path| !path.contains('\\')));
    }

    #[test]
    fn td19_metadata_json_is_not_the_media_sidecar() {
        let report = discover(&fixture("td19")).expect("directory");
        let media = hit(&report, "Viagem Europa 2022/foto001.jpg");
        assert_eq!(media.confidence, Confidence::Exact);
        assert_eq!(
            media.json_relative_path.as_deref(),
            Some("Viagem Europa 2022/foto001.jpg.json")
        );
        assert_ne!(
            media.json_relative_path.as_deref(),
            Some("Viagem Europa 2022/metadata.json")
        );
    }

    #[test]
    fn td20_album_metadata_fields() {
        let report = discover(&fixture("td19")).expect("directory");
        assert_eq!(report.albums.len(), 1);
        let album = &report.albums[0];
        assert_eq!(album.relative_path, "Viagem Europa 2022/metadata.json");
        assert_eq!(album.metadata.title.as_deref(), Some("Viagem Europa 2022"));
        assert_eq!(album.metadata.description, None);
        assert_eq!(album.metadata.date_unix, Some(1660000000));
        assert_eq!(album.metadata.latitude, None);
        assert_eq!(album.metadata.longitude, None);
        assert_eq!(album.metadata.altitude, None);
    }

    #[test]
    fn td21_folder_without_metadata_json_has_no_album() {
        let report = discover(&fixture("td21")).expect("directory");
        assert!(report.albums.is_empty());
        assert_eq!(
            hit(&report, "Photos from 2022/foto.jpg").relative_path,
            "Photos from 2022/foto.jpg"
        );
    }

    #[test]
    fn td22_invalid_album_metadata_stays_a_problem() {
        let report = discover(&fixture("td22")).expect("directory");
        let media = hit(&report, "Viagem Europa 2022/foto001.jpg");
        assert_eq!(media.confidence, Confidence::Exact);
        assert!(report.albums.is_empty());
        assert!(report.problems.iter().any(|problem| {
            problem.relative_path == "Viagem Europa 2022/metadata.json"
                && problem.kind == ProblemKind::InvalidJson
        }));
    }

    #[test]
    fn td23_invalid_json_does_not_drop_the_neighbor() {
        let report = discover(&fixture("td23")).expect("directory");
        let broken = hit(&report, "foto.jpg");
        let neighbor = hit(&report, "outra.jpg");
        assert_eq!(broken.confidence, Confidence::None);
        assert_eq!(broken.json_relative_path, None);
        assert_eq!(broken.reason, None);
        assert_eq!(neighbor.confidence, Confidence::Exact);
        assert_eq!(neighbor.json_relative_path.as_deref(), Some("outra.jpg.json"));
        assert!(report.problems.iter().any(|problem| {
            problem.relative_path == "foto.jpg.json" && problem.kind == ProblemKind::InvalidJson
        }));
    }

    #[test]
    fn td24_unreadable_json_does_not_drop_the_neighbor() {
        let report = discover_with(&fixture("td24"), |path| {
            if path.file_name().and_then(|name| name.to_str()) == Some("foto.jpg.json") {
                Err("bloqueado".into())
            } else {
                fs::read(path).map_err(|error| error.to_string())
            }
        })
        .expect("directory");
        let blocked = hit(&report, "foto.jpg");
        let neighbor = hit(&report, "outra.jpg");
        assert_eq!(blocked.confidence, Confidence::None);
        assert_eq!(blocked.reason, None);
        assert_eq!(neighbor.confidence, Confidence::Exact);
        assert!(neighbor.metadata.is_some());
        assert!(report.problems.iter().any(|problem| {
            problem.relative_path == "foto.jpg.json" && problem.kind == ProblemKind::Unreadable
        }));
    }

    #[test]
    fn ignored_files_are_absent_from_the_report() {
        let report = discover(&fixture("td25")).expect("directory");
        assert!(report.media.is_empty());
        assert!(report.albums.is_empty());
        assert!(report.problems.is_empty());
    }

    #[test]
    fn td27_discover_does_not_change_fixture_bytes() {
        let root = fixture("td27");
        let before = tree_bytes(&root);
        let report = discover(&root).expect("directory");
        assert_eq!(tree_bytes(&root), before);
        assert!(before.contains_key("nested/foto.jpg"));
        assert_eq!(
            hit(&report, "nested/foto.jpg").confidence,
            Confidence::Exact
        );
        assert!(discover(&root.join("nested").join("foto.jpg")).is_err());
        assert!(discover(&fixture("missing-root")).is_err());
        #[cfg(windows)]
        assert_symlink_is_not_followed();
    }

    #[test]
    fn media_without_json_is_none_without_an_invalid_json_problem() {
        let report = discover(&fixture("edge-none")).expect("directory");
        let media = hit(&report, "foto.jpg");
        assert_eq!(media.confidence, Confidence::None);
        assert_eq!(media.reason, None);
        assert_eq!(media.json_relative_path, None);
        assert!(report
            .problems
            .iter()
            .all(|problem| problem.kind != ProblemKind::InvalidJson));
    }

    #[test]
    fn unmatched_json_is_an_unread_orphan() {
        let report = discover_with(&fixture("edge-orphan"), |_| Err("lido".into())).expect("directory");
        assert!(report.media.is_empty());
        assert_eq!(report.problems.len(), 1);
        assert_eq!(report.problems[0].relative_path, "lonely.json");
        assert_eq!(report.problems[0].kind, ProblemKind::OrphanJson);
    }

    #[test]
    fn edited_without_original_json_is_none() {
        let report = discover(&fixture("edge-edited")).expect("directory");
        let edited = hit(&report, "foto-edited.jpg");
        assert_eq!(edited.confidence, Confidence::None);
        assert_eq!(edited.json_relative_path, None);
        assert_eq!(edited.reason, None);
    }

    #[test]
    fn sidecar_in_another_part_of_the_same_export_matches_with_physical_paths() {
        let temp = std::env::temp_dir().join(format!("my-photos-f02-parts-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let folder = |part: &str| {
            let path = temp.join(part).join("Takeout").join("Google Fotos").join("Fotos de 2025");
            fs::create_dir_all(&path).expect("part folder");
            path
        };
        let json = br#"{ "photoTakenTime": { "timestamp": "1660142220" } }"#;
        fs::write(folder("takeout-20260921T183943Z-1-001").join("a.jpg"), b"a").expect("media");
        fs::write(
            folder("takeout-20260921T183943Z-1-002").join("a.jpg.supplemental-metadata.json"),
            json,
        )
        .expect("sidecar");
        // Another export never lends its sidecars.
        fs::write(folder("takeout-20250101T000000Z-1-001").join("b.jpg"), b"b").expect("media");
        fs::write(
            folder("takeout-20260921T183943Z-1-001").join("b.jpg.supplemental-metadata.json"),
            json,
        )
        .expect("sidecar");
        fs::write(temp.join("drive-download.zip"), b"PK").expect("zip");

        let report = discover(&temp);
        fs::remove_dir_all(&temp).expect("cleanup");
        let report = report.expect("directory");

        let a = hit(
            &report,
            "takeout-20260921T183943Z-1-001/Takeout/Google Fotos/Fotos de 2025/a.jpg",
        );
        assert_eq!(a.confidence, Confidence::Heuristic);
        assert_eq!(
            a.json_relative_path.as_deref(),
            Some("takeout-20260921T183943Z-1-002/Takeout/Google Fotos/Fotos de 2025/a.jpg.supplemental-metadata.json")
        );
        assert_eq!(a.metadata.as_ref().and_then(|item| item.taken_at_unix), Some(1660142220));

        let b = hit(
            &report,
            "takeout-20250101T000000Z-1-001/Takeout/Google Fotos/Fotos de 2025/b.jpg",
        );
        assert_eq!(b.confidence, Confidence::None);
        assert!(report.problems.iter().any(|problem| {
            problem.kind == ProblemKind::OrphanJson
                && problem.relative_path.ends_with("b.jpg.supplemental-metadata.json")
        }));
        assert!(report.media.iter().all(|item| !item.relative_path.ends_with(".zip")));
        assert!(report.problems.iter().all(|problem| !problem.relative_path.ends_with(".zip")));
    }

    #[test]
    fn takeout_part_suffix_is_dropped_only_from_part_folders() {
        use super::{logical_dir, takeout_export};
        assert_eq!(
            takeout_export("takeout-20260921T183943Z-1-002"),
            Some("takeout-20260921T183943Z")
        );
        assert_eq!(takeout_export("Takeout"), None);
        assert_eq!(takeout_export("takeout-1-2"), None);
        assert_eq!(takeout_export("takeout-x-a-001"), None);
        assert_eq!(takeout_export("Photos-1-001"), None);
        assert_eq!(
            logical_dir("Ruan/takeout-20260921T183943Z-2-001/Takeout/Drive"),
            "Ruan/takeout-20260921T183943Z/Takeout/Drive"
        );
        assert_eq!(logical_dir(""), "");
    }

    /// Read-only survey of a real Takeout. Run with
    /// `TAKEOUT_ROOT=<dir> cargo test real_takeout_survey -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_takeout_survey() {
        let root = PathBuf::from(std::env::var("TAKEOUT_ROOT").expect("TAKEOUT_ROOT"));
        let report = discover(&root).expect("directory");
        let total = report.media.len();
        let count = |confidence: Confidence| {
            report.media.iter().filter(|item| item.confidence == confidence).count()
        };
        let (exact, heuristic, none) = (
            count(Confidence::Exact),
            count(Confidence::Heuristic),
            count(Confidence::None),
        );
        let ambiguous = report
            .media
            .iter()
            .filter(|item| item.reason == Some(UnmatchedReason::Ambiguous))
            .count();
        println!("media={total} exact={exact} heuristic={heuristic} none={none} ambiguous={ambiguous}");
        if total > 0 {
            println!(
                "matched={:.2}%",
                100.0 * (exact + heuristic) as f64 / total as f64
            );
        }
        println!("albums={}", report.albums.len());
        let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
        for problem in &report.problems {
            *kinds.entry(format!("{:?}", problem.kind)).or_default() += 1;
        }
        println!("problems={kinds:?}");
        for item in report.media.iter().filter(|item| item.confidence == Confidence::None) {
            println!("NONE {}", item.relative_path);
        }
        for problem in &report.problems {
            println!("PROBLEM {:?} {}", problem.kind, problem.relative_path);
        }
    }

    #[test]
    fn unreadable_subdirectory_is_a_problem_and_keeps_the_neighbor() {
        let temp = std::env::temp_dir().join(format!("my-photos-f02-locked-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        let locked = temp.join("locked");
        fs::create_dir_all(&locked).expect("locked");
        fs::write(locked.join("foto.jpg"), b"x").expect("hidden media");
        fs::write(temp.join("outra.jpg"), b"y").expect("neighbor");
        // Dropping every inherited ACE blocks the process whatever SID it runs as.
        // The owner keeps WRITE_DAC, so /reset restores the inherited ACL.
        let acl = |args: &[&str]| {
            let output = std::process::Command::new("icacls")
                .arg(&locked)
                .args(args)
                .output()
                .expect("icacls");
            assert!(output.status.success(), "icacls {args:?}");
        };
        acl(&["/inheritance:r"]);

        let report = discover(&temp);
        acl(&["/reset"]);
        fs::remove_dir_all(&temp).expect("cleanup");

        let report = report.expect("directory");
        assert!(report.problems.iter().any(|problem| {
            problem.relative_path == "locked" && problem.kind == ProblemKind::UnreadableDirectory
        }));
        assert_eq!(hit(&report, "outra.jpg").confidence, Confidence::None);
        assert!(report.media.iter().all(|item| item.relative_path != "locked/foto.jpg"));
    }

    fn tree_bytes(root: &Path) -> BTreeMap<String, (u64, Vec<u8>)> {
        let mut found = BTreeMap::new();
        fn walk(directory: &Path, relative: &str, found: &mut BTreeMap<String, (u64, Vec<u8>)>) {
            for entry in fs::read_dir(directory).expect("read dir") {
                let entry = entry.expect("entry");
                let file_type = entry.file_type().expect("type");
                if file_type.is_symlink() {
                    continue;
                }
                let name = entry.file_name().into_string().expect("utf-8");
                let relative = if relative.is_empty() {
                    name
                } else {
                    format!("{relative}/{name}")
                };
                if file_type.is_dir() {
                    walk(&entry.path(), &relative, found);
                } else if file_type.is_file() {
                    let bytes = fs::read(entry.path()).expect("bytes");
                    let size = bytes.len() as u64;
                    found.insert(relative, (size, bytes));
                }
            }
        }
        walk(root, "", &mut found);
        found
    }

    #[cfg(windows)]
    fn assert_symlink_is_not_followed() {
        let temp = std::env::temp_dir().join(format!("my-photos-f02-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("outside")).expect("outside");
        fs::write(temp.join("outside").join("secret.jpg"), b"secret").expect("secret");
        fs::create_dir_all(temp.join("root")).expect("root");
        std::os::windows::fs::symlink_dir(temp.join("outside"), temp.join("root").join("link"))
            .expect("symlink");
        let report = discover(&temp.join("root")).expect("directory");
        assert!(report.media.is_empty());
        assert!(report.problems.is_empty());
        fs::remove_dir_all(&temp).expect("cleanup");
    }
}

