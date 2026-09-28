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
    media
        .into_iter()
        .map(|(name, extension)| resolve_steps_1_to_3(&name, &extension, &sidecars))
        .collect()
}

fn resolve_steps_1_to_3(name: &str, _extension: &str, sidecars: &[String]) -> NameMatch {
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

    NameMatch {
        media_name: name.to_string(),
        confidence: Confidence::None,
        json_name: None,
        reason: None,
    }
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
}
