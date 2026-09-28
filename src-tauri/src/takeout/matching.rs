use crate::takeout::classify::{classify, FileRole};
use crate::takeout::model::{Confidence, UnmatchedReason};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameMatch {
    pub media_name: String,
    pub confidence: Confidence,
    pub json_name: Option<String>,
    pub reason: Option<UnmatchedReason>,
}

pub fn truncate_sidecar_name(full: &str) -> Option<String> {
    if full.len() <= 51 {
        return Some(full.to_string());
    }
    if !full.is_char_boundary(46) {
        return None;
    }
    Some(format!("{}.json", &full[..46]))
}

struct Pending {
    name: String,
    extension: String,
    confidence: Confidence,
    json_name: Option<String>,
    reason: Option<UnmatchedReason>,
}

pub fn match_directory(names: &[String]) -> Vec<NameMatch> {
    let mut sidecars = Vec::new();
    let mut media = Vec::new();
    for name in names {
        match classify(name) {
            FileRole::JsonSidecar => sidecars.push(name.clone()),
            FileRole::Media { extension } => media.push((name.clone(), extension)),
            FileRole::AlbumMetadata | FileRole::Ignored => {}
        }
    }
    let mut pending: Vec<Pending> = media
        .iter()
        .map(|(name, extension)| {
            let matched = resolve_steps_1_to_4(name, extension, &sidecars);
            Pending {
                name: name.clone(),
                extension: extension.clone(),
                confidence: matched.confidence,
                json_name: matched.json_name,
                reason: matched.reason,
            }
        })
        .collect();
    clear_shared_json(&mut pending);
    apply_edited(&mut pending, &sidecars);
    clear_shared_json(&mut pending);
    pending
        .into_iter()
        .map(|item| NameMatch {
            media_name: item.name,
            confidence: item.confidence,
            json_name: item.json_name,
            reason: item.reason,
        })
        .collect()
}

fn resolve_steps_1_to_4(name: &str, extension: &str, sidecars: &[String]) -> NameMatch {
    let stem = stem_of(name);
    let file_json = format!("{name}.json");
    let stem_json = format!("{stem}.json");

    if let Some(matched) = step(&[file_json.clone(), stem_json.clone()], sidecars, Confidence::Exact)
    {
        return matched_or_none(name, matched);
    }

    let supplemental = [
        format!("{name}.supplemental-metadata.json"),
        format!("{stem}.supplemental-metadata.json"),
    ];
    let supplemental: Vec<String> = supplemental
        .iter()
        .filter_map(|logical| truncate_sidecar_name(logical))
        .collect();
    if let Some(matched) = step(&supplemental, sidecars, Confidence::Heuristic) {
        return matched_or_none(name, matched);
    }

    let truncated: Vec<String> = [file_json, stem_json]
        .iter()
        .filter_map(|logical| truncated_only(logical))
        .collect();
    if let Some(matched) = step(&truncated, sidecars, Confidence::Heuristic) {
        return matched_or_none(name, matched);
    }

    if let Some((base, marker)) = duplicate_marker(stem) {
        let dotted = format!("{base}.{extension}");
        let expected = [
            format!("{dotted}.json"),
            format!("{dotted}.supplemental-metadata.json"),
        ];
        let expected: Vec<String> = expected
            .iter()
            .filter_map(|logical| append_marker(logical, &marker))
            .collect();
        if let Some(matched) = step(&expected, sidecars, Confidence::Heuristic) {
            return matched_or_none(name, matched);
        }
    }

    NameMatch {
        media_name: name.to_string(),
        confidence: Confidence::None,
        json_name: None,
        reason: None,
    }
}

fn duplicate_marker(stem: &str) -> Option<(String, String)> {
    if !stem.ends_with(')') {
        return None;
    }
    let open = stem.rfind('(')?;
    let inner = &stem[open + 1..stem.len() - 1];
    let mut chars = inner.chars();
    let first = chars.next()?;
    if !first.is_ascii_digit() || first == '0' || !chars.all(|character| character.is_ascii_digit())
    {
        return None;
    }
    Some((stem[..open].to_string(), stem[open..].to_string()))
}

fn append_marker(logical: &str, marker: &str) -> Option<String> {
    let truncated = truncate_sidecar_name(logical)?;
    let without_json = truncated.strip_suffix(".json")?;
    Some(format!("{without_json}{marker}.json"))
}

fn strip_edited(stem: &str) -> Option<&str> {
    const SUFFIX: &str = "-edited";
    if stem.len() >= SUFFIX.len() && stem[stem.len() - SUFFIX.len()..].eq_ignore_ascii_case(SUFFIX)
    {
        Some(&stem[..stem.len() - SUFFIX.len()])
    } else {
        None
    }
}

fn apply_edited(pending: &mut [Pending], sidecars: &[String]) {
    let held: Vec<(String, String, Option<String>)> = pending
        .iter()
        .filter(|item| item.confidence != Confidence::None)
        .map(|item| (item.name.clone(), item.extension.clone(), item.json_name.clone()))
        .collect();

    for item in pending.iter_mut() {
        if item.confidence != Confidence::None || item.reason.is_some() {
            continue;
        }
        let Some(base) = strip_edited(stem_of(&item.name)) else {
            continue;
        };
        let synthetic = format!("{base}.{}", item.extension);
        let matched = resolve_steps_1_to_4(&synthetic, &item.extension, sidecars);
        let Some(json_name) = matched.json_name else {
            continue;
        };
        if matched.confidence == Confidence::None {
            continue;
        }
        let original_holds = held.iter().any(|(name, extension, json)| {
            stem_of(name).eq_ignore_ascii_case(base)
                && extension.eq_ignore_ascii_case(&item.extension)
                && json
                    .as_ref()
                    .is_some_and(|value| value.eq_ignore_ascii_case(&json_name))
        });
        if original_holds {
            item.confidence = Confidence::Heuristic;
            item.json_name = Some(json_name);
            item.reason = None;
        }
    }
}

fn clear_shared_json(pending: &mut [Pending]) {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for index in 0..pending.len() {
        let Some(json_name) = pending[index].json_name.clone() else {
            continue;
        };
        if groups.iter().any(|group| {
            group.iter().any(|&other| {
                pending[other]
                    .json_name
                    .as_ref()
                    .is_some_and(|value| value.eq_ignore_ascii_case(&json_name))
            })
        }) {
            continue;
        }
        let group = (0..pending.len())
            .filter(|&other| {
                pending[other]
                    .json_name
                    .as_ref()
                    .is_some_and(|value| value.eq_ignore_ascii_case(&json_name))
            })
            .collect();
        groups.push(group);
    }

    for group in groups {
        if group.len() < 2 {
            continue;
        }
        let owners: Vec<usize> = group
            .iter()
            .copied()
            .filter(|&index| !is_edited_partner(pending, index, &group))
            .collect();
        if owners.len() <= 1 {
            continue;
        }
        for index in owners {
            pending[index].confidence = Confidence::None;
            pending[index].json_name = None;
            pending[index].reason = Some(UnmatchedReason::Ambiguous);
        }
        for index in &group {
            if is_edited_partner(pending, *index, &group) {
                pending[*index].confidence = Confidence::None;
                pending[*index].json_name = None;
                pending[*index].reason = None;
            }
        }
    }
}

fn is_edited_partner(pending: &[Pending], index: usize, group: &[usize]) -> bool {
    let Some(base) = strip_edited(stem_of(&pending[index].name)) else {
        return false;
    };
    group.iter().any(|&other| {
        other != index
            && stem_of(&pending[other].name).eq_ignore_ascii_case(base)
            && pending[other]
                .extension
                .eq_ignore_ascii_case(&pending[index].extension)
    })
}

fn stem_of(name: &str) -> &str {
    name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name)
}

fn truncated_only(logical: &str) -> Option<String> {
    let truncated = truncate_sidecar_name(logical)?;
    if truncated.eq_ignore_ascii_case(logical) {
        None
    } else {
        Some(truncated)
    }
}

fn step(expected: &[String], sidecars: &[String], confidence: Confidence) -> Option<NameMatch> {
    let mut hits = Vec::new();
    for sidecar in sidecars {
        if expected
            .iter()
            .any(|candidate| candidate.eq_ignore_ascii_case(sidecar))
        {
            hits.push(sidecar.clone());
        }
    }
    match hits.len() {
        0 => None,
        1 => Some(NameMatch {
            media_name: String::new(),
            confidence,
            json_name: Some(hits.remove(0)),
            reason: None,
        }),
        _ => Some(NameMatch {
            media_name: String::new(),
            confidence: Confidence::None,
            json_name: None,
            reason: Some(UnmatchedReason::Ambiguous),
        }),
    }
}

fn matched_or_none(name: &str, mut matched: NameMatch) -> NameMatch {
    matched.media_name = name.to_string();
    matched
}

#[cfg(test)]
mod tests {
    use super::{match_directory, truncate_sidecar_name};
    use crate::takeout::model::{Confidence, UnmatchedReason};

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    fn find<'a>(matches: &'a [super::NameMatch], media_name: &str) -> &'a super::NameMatch {
        matches
            .iter()
            .find(|item| item.media_name == media_name)
            .unwrap_or_else(|| panic!("missing {media_name}"))
    }

    #[test]
    fn td01_exact_file_json() {
        let matches = match_directory(&names(&[
            "IMG_20220810_143700.jpg",
            "IMG_20220810_143700.jpg.json",
        ]));
        let hit = find(&matches, "IMG_20220810_143700.jpg");
        assert_eq!(hit.confidence, Confidence::Exact);
        assert_eq!(
            hit.json_name.as_deref(),
            Some("IMG_20220810_143700.jpg.json")
        );
        assert_eq!(hit.reason, None);
    }

    #[test]
    fn td02_exact_ignores_ascii_case() {
        let matches = match_directory(&names(&[
            "IMG_20220810_143700.JPG",
            "IMG_20220810_143700.jpg.json",
        ]));
        let hit = find(&matches, "IMG_20220810_143700.JPG");
        assert_eq!(hit.confidence, Confidence::Exact);
        assert_eq!(
            hit.json_name.as_deref(),
            Some("IMG_20220810_143700.jpg.json")
        );
    }

    #[test]
    fn td03_stem_json_is_exact_and_both_spellings_tie() {
        let matches = match_directory(&names(&["foto.jpg", "foto.json"]));
        let hit = find(&matches, "foto.jpg");
        assert_eq!(hit.confidence, Confidence::Exact);
        assert_eq!(hit.json_name.as_deref(), Some("foto.json"));
        assert_eq!(hit.reason, None);

        let tied = match_directory(&names(&["foto.jpg", "foto.jpg.json", "foto.json"]));
        let tied = find(&tied, "foto.jpg");
        assert_eq!(tied.confidence, Confidence::None);
        assert_eq!(tied.json_name, None);
        assert_eq!(tied.reason, Some(UnmatchedReason::Ambiguous));
    }

    #[test]
    fn td04_supplemental_metadata_is_heuristic() {
        let matches = match_directory(&names(&[
            "foto.jpg",
            "foto.jpg.supplemental-metadata.json",
        ]));
        let hit = find(&matches, "foto.jpg");
        assert_eq!(hit.confidence, Confidence::Heuristic);
        assert_eq!(
            hit.json_name.as_deref(),
            Some("foto.jpg.supplemental-metadata.json")
        );
        assert_eq!(hit.reason, None);

        let media_name = format!("{}extra.jpg", "n".repeat(40));
        let logical = format!("{media_name}.supplemental-metadata.json");
        let sidecar = truncate_sidecar_name(&logical).expect("truncated supplemental");
        assert!(logical.len() > 51);
        assert_ne!(sidecar, logical);
        let matches = match_directory(&names(&[&media_name, &sidecar]));
        let hit = find(&matches, &media_name);
        assert_eq!(hit.confidence, Confidence::Heuristic);
        assert_eq!(hit.json_name.as_deref(), Some(sidecar.as_str()));
    }

    #[test]
    fn td05_truncated_file_json_is_heuristic_and_one_file_is_one_candidate() {
        assert_eq!(
            truncate_sidecar_name("foto.json"),
            Some("foto.json".to_string())
        );
        let long = format!("{}X.json", "b".repeat(50));
        assert_eq!(
            truncate_sidecar_name(&long),
            Some(format!("{}.json", "b".repeat(46)))
        );

        let media_name = "foto_com_nome_longo_demais_para_o_takeout_001.jpg";
        let sidecar = "foto_com_nome_longo_demais_para_o_takeout_001..json";
        let matches = match_directory(&names(&[media_name, sidecar]));
        let hit = find(&matches, media_name);
        assert_eq!(hit.confidence, Confidence::Heuristic);
        assert_eq!(hit.json_name.as_deref(), Some(sidecar));
        assert_eq!(hit.reason, None);

        let stem = "c".repeat(48);
        let shared_media = format!("{stem}.jpg");
        let file_json = format!("{shared_media}.json");
        let stem_json = format!("{stem}.json");
        let shared = truncate_sidecar_name(&file_json).expect("file truncation");
        assert_eq!(
            truncate_sidecar_name(&stem_json).as_deref(),
            Some(shared.as_str())
        );
        let matches = match_directory(&names(&[&shared_media, &shared]));
        let hit = find(&matches, &shared_media);
        assert_eq!(hit.confidence, Confidence::Heuristic);
        assert_eq!(hit.json_name.as_deref(), Some(shared.as_str()));
        assert_eq!(hit.reason, None);
    }

    #[test]
    fn truncation_that_splits_a_utf8_character_is_absent() {
        let mut name = "a".repeat(45);
        name.push('é');
        name.push_str(&"b".repeat(10));
        assert!(name.len() > 51);
        assert!(!name.is_char_boundary(46));
        assert_eq!(truncate_sidecar_name(&name), None);
    }

    #[test]
    fn duplicate_marker_matches_shifted_sidecar_after_truncation() {
        let matches = match_directory(&names(&[
            "camphoto_1271212614(1).jpg",
            "camphoto_1271212614.jpg.supplemental-metadata(1).json",
        ]));
        let hit = find(&matches, "camphoto_1271212614(1).jpg");
        assert_eq!(hit.confidence, Confidence::Heuristic);
        assert_eq!(
            hit.json_name.as_deref(),
            Some("camphoto_1271212614.jpg.supplemental-metadata(1).json")
        );
        assert_ne!(hit.confidence, Confidence::Exact);

        let base = "q".repeat(40);
        let media_name = format!("{base}(1).jpg");
        let logical = format!("{base}.jpg.supplemental-metadata.json");
        let truncated = truncate_sidecar_name(&logical).expect("truncated");
        let sidecar = format!("{}(1).json", truncated.strip_suffix(".json").expect("json"));
        assert!(sidecar.len() > 51);
        let matches = match_directory(&names(&[&media_name, &sidecar]));
        let hit = find(&matches, &media_name);
        assert_eq!(hit.confidence, Confidence::Heuristic);
        assert_eq!(hit.json_name.as_deref(), Some(sidecar.as_str()));

        for (media_name, sidecar) in [
            ("foto(0).jpg", "foto.jpg(0).json"),
            ("foto(01).jpg", "foto.jpg(01).json"),
        ] {
            let matches = match_directory(&names(&[media_name, sidecar]));
            let hit = find(&matches, media_name);
            assert_eq!(hit.confidence, Confidence::None);
            assert_eq!(hit.json_name, None);
            assert_eq!(hit.reason, None);
        }
    }

    #[test]
    fn edited_file_reuses_original_json_as_heuristic() {
        let matches = match_directory(&names(&["foto.jpg", "foto.jpg.json", "foto-edited.jpg"]));
        let original = find(&matches, "foto.jpg");
        let edited = find(&matches, "foto-edited.jpg");
        assert_eq!(original.confidence, Confidence::Exact);
        assert_eq!(original.json_name.as_deref(), Some("foto.jpg.json"));
        assert_eq!(edited.confidence, Confidence::Heuristic);
        assert_ne!(edited.confidence, Confidence::Exact);
        assert_eq!(edited.json_name.as_deref(), Some("foto.jpg.json"));
        assert_eq!(edited.reason, None);
    }

    #[test]
    fn shared_json_is_ambiguous_and_edited_partner_has_no_reason() {
        let matches = match_directory(&names(&[
            "foto.jpg",
            "foto.png",
            "foto.json",
            "foto-edited.jpg",
        ]));
        let jpg = find(&matches, "foto.jpg");
        let png = find(&matches, "foto.png");
        let edited = find(&matches, "foto-edited.jpg");
        assert_eq!(jpg.confidence, Confidence::None);
        assert_eq!(jpg.json_name, None);
        assert_eq!(jpg.reason, Some(UnmatchedReason::Ambiguous));
        assert_eq!(png.confidence, Confidence::None);
        assert_eq!(png.json_name, None);
        assert_eq!(png.reason, Some(UnmatchedReason::Ambiguous));
        assert_eq!(edited.confidence, Confidence::None);
        assert_eq!(edited.json_name, None);
        assert_eq!(edited.reason, None);
    }

    #[test]
    fn edited_without_original_json_stays_unmatched() {
        let missing = match_directory(&names(&["foto-edited.jpg"]));
        let missing = find(&missing, "foto-edited.jpg");
        assert_eq!(missing.confidence, Confidence::None);
        assert_eq!(missing.json_name, None);
        assert_eq!(missing.reason, None);

        let original_none = match_directory(&names(&["foto.jpg", "foto-edited.jpg"]));
        let edited = find(&original_none, "foto-edited.jpg");
        assert_eq!(edited.confidence, Confidence::None);
        assert_eq!(edited.json_name, None);
        assert_eq!(edited.reason, None);
        assert_eq!(find(&original_none, "foto.jpg").confidence, Confidence::None);
    }
}
