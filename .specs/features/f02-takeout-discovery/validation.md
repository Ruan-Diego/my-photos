# F02 Takeout Discovery Validation

**Date**: 2026-09-28
**Spec**: `.specs/features/f02-takeout-discovery/spec.md`
**Diff range**: `66158ff` (`feat(takeout): add the discovery report types`) through `e6fd16c` (`test(takeout): keep unknown JSON values out of known fields`), inclusive. Implementation commits end at `ee16cfb`. `e6fd16c` is test-only.
**Verifier**: independent sub-agent, second iteration (author ≠ verifier; this verifier did not write the fix)

This iteration re-checked the extra-field edge and re-ran the sensor against `e6fd16c`. The 27/27 acceptance-criteria evidence from the first report is unchanged: those assertions still sit on the same lines.

---

## Task Completion

| Task | Status | Notes |
| ---- | ------ | ----- |
| T1 | ✅ Done | Report types, no `Serialize` |
| T2 | ✅ Done | Classify, 6 tests |
| T3 | ✅ Done | Truncation and cascade steps 1–3, 6 tests |
| T4 | ✅ Done | Steps 4–5 and shared-JSON refusal, 4 tests |
| T5 | ✅ Done | Media JSON parse, 10 tests |
| T6 | ✅ Done | Album JSON parse, 2 tests |
| T7 | ✅ Done | Discovery walk, 21 tests |

No task is blocked or partial. All Done-when boxes in `tasks.md` are checked. The follow-up commit `e6fd16c` only adds assertions inside `unknown_field_is_ignored`.

---

## Spec-Anchored Acceptance Criteria

### P1: Associar mídia ao JSON da mesma pasta

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| TD-01 WHEN `IMG_20220810_143700.jpg` and `IMG_20220810_143700.jpg.json` are in the same folder THEN associate them with confidence `exact` and the JSON relative path | `Confidence::Exact` and JSON path `IMG_20220810_143700.jpg.json` | `src-tauri/src/takeout/mod.rs:250` — `assert_eq!(media.confidence, Confidence::Exact)`; `:251` — `assert_eq!(media.json_relative_path.as_deref(), Some("IMG_20220810_143700.jpg.json"))` | ✅ PASS |
| TD-02 WHEN the media is `IMG_20220810_143700.JPG` and the JSON is `IMG_20220810_143700.jpg.json` THEN associate with confidence `exact` | `Confidence::Exact` and that JSON path | `src-tauri/src/takeout/mod.rs:263` — `assert_eq!(media.confidence, Confidence::Exact)`; `:264` — `assert_eq!(media.json_relative_path.as_deref(), Some("IMG_20220810_143700.jpg.json"))` | ✅ PASS |
| TD-03 WHEN the folder has `foto.jpg` and only `foto.json` THEN associate with confidence `exact` | `Confidence::Exact` and JSON path `foto.json` | `src-tauri/src/takeout/mod.rs:274` — `assert_eq!(media.confidence, Confidence::Exact)`; `:275` — `assert_eq!(media.json_relative_path.as_deref(), Some("foto.json"))` | ✅ PASS |
| TD-04 WHEN the folder has `foto.jpg` and `foto.jpg.supplemental-metadata.json` THEN associate with confidence `heuristic` | `Confidence::Heuristic` and that sidecar path | `src-tauri/src/takeout/mod.rs:282` — `assert_eq!(media.confidence, Confidence::Heuristic)`; `:283` — `assert_eq!(media.json_relative_path.as_deref(), Some("foto.jpg.supplemental-metadata.json"))` | ✅ PASS |
| TD-05 WHEN the media is `foto_com_nome_longo_demais_para_o_takeout_001.jpg` and the only JSON is `foto_com_nome_longo_demais_para_o_takeout_001..json` THEN associate with confidence `heuristic` | `Confidence::Heuristic` and that truncated JSON path | `src-tauri/src/takeout/mod.rs:293` — `assert_eq!(media.confidence, Confidence::Heuristic)`; `:294` — `assert_eq!(media.json_relative_path.as_deref(), Some("foto_com_nome_longo_demais_para_o_takeout_001..json"))` | ✅ PASS |
| TD-06 WHEN the media is `camphoto_1271212614(1).jpg` and the JSON is `camphoto_1271212614.jpg.supplemental-metadata(1).json` THEN associate with confidence `heuristic` | `Confidence::Heuristic` and that sidecar path | `src-tauri/src/takeout/mod.rs:304` — `assert_eq!(media.confidence, Confidence::Heuristic)`; `:305` — `assert_eq!(media.json_relative_path.as_deref(), Some("camphoto_1271212614.jpg.supplemental-metadata(1).json"))` | ✅ PASS |
| TD-07 WHEN the folder has `foto.jpg`, `foto.jpg.json`, and `foto-edited.jpg` THEN the original is `exact` on that JSON and the edited file is `heuristic` on the same JSON | original `Exact`, edited `Heuristic`, both paths `foto.jpg.json` | `src-tauri/src/takeout/mod.rs:316` — `assert_eq!(original.confidence, Confidence::Exact)`; `:317` — `assert_eq!(edited.confidence, Confidence::Heuristic)`; `:319` — `assert_eq!(original.json_relative_path.as_deref(), Some("foto.jpg.json"))`; `:320` — `assert_eq!(edited.json_relative_path.as_deref(), Some("foto.jpg.json"))` | ✅ PASS |
| TD-08 WHEN two non edited-pair media share one truncated JSON THEN both are `none` with reason `ambiguous`, and the JSON is not associated | both `None`, `json_relative_path` `None`, reason `Ambiguous`; JSON not chosen | `src-tauri/src/takeout/mod.rs:337` — `assert_eq!(media.confidence, Confidence::None)`; `:338` — `assert_eq!(media.json_relative_path, None)`; `:339` — `assert_eq!(media.reason, Some(UnmatchedReason::Ambiguous))`; `:341` — orphan problem path is the shared `{prefix}.json` with `ProblemKind::OrphanJson` | ✅ PASS |
| TD-09 WHEN the matching JSON is in the parent folder and the media folder has no sidecar THEN the media is `none` | `Confidence::None` and no JSON path for `child/foto.jpg` | `src-tauri/src/takeout/mod.rs:351` — `assert_eq!(media.confidence, Confidence::None)`; `:352` — `assert_eq!(media.json_relative_path, None)` | ✅ PASS |
| TD-10 WHEN the same tree is discovered twice THEN the associations are equal, in lexicographic order of the media relative path | second report equals the first; paths `a/a.jpg` then `b.jpg` | `src-tauri/src/takeout/mod.rs:361` — `assert_eq!(first, second)`; `:367` — `assert_eq!(paths, vec!["a/a.jpg", "b.jpg"])` | ✅ PASS |

### P1: Extrair os campos do JSON de mídia

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| TD-11 WHEN `photoTakenTime.timestamp` is `"1660142220"` and `creationTime.timestamp` is `"1660145820"` THEN `taken_at_unix` is `1660142220` and `creation_time_unix` is `1660145820` | those two integers on those two fields | `src-tauri/src/takeout/parse.rs:122` — `assert_eq!(metadata.taken_at_unix, Some(1660142220))`; `:123` — `assert_eq!(metadata.creation_time_unix, Some(1660145820))` | ✅ PASS |
| TD-12 WHEN `photoTakenTime` is absent and `creationTime.timestamp` is present THEN `taken_at_unix` is absent and `creation_time_unix` receives that timestamp | `taken_at_unix` `None`, `creation_time_unix` `Some(1660145820)` | `src-tauri/src/takeout/parse.rs:132` — `assert_eq!(metadata.taken_at_unix, None)`; `:133` — `assert_eq!(metadata.creation_time_unix, Some(1660145820))` | ✅ PASS |
| TD-13 WHEN `geoData.latitude` and `geoData.longitude` are both `0.0` THEN latitude, longitude, and altitude are absent, even if altitude is `0.0` | all three `None` | `src-tauri/src/takeout/parse.rs:142` — `assert_eq!(metadata.latitude, None)`; `:143` — `assert_eq!(metadata.longitude, None)`; `:144` — `assert_eq!(metadata.altitude, None)` | ✅ PASS |
| TD-14 WHEN `geoData` is `0.0/0.0` and `geoDataExif` has latitude `-23.5505` and longitude `-46.6333` THEN the result uses those coordinates and the `geoDataExif` altitude | latitude `-23.5505`, longitude `-46.6333`, altitude `760.0` from the Exif object in the same input | `src-tauri/src/takeout/parse.rs:156` — `assert_eq!(metadata.latitude, Some(-23.5505))`; `:157` — `assert_eq!(metadata.longitude, Some(-46.6333))`; `:158` — `assert_eq!(metadata.altitude, Some(760.0))` | ✅ PASS |
| TD-15 WHEN `geoData` has latitude `-23.5505` and longitude `-46.6333` and `geoDataExif` has other coordinates THEN the result uses only `geoData` | latitude `-23.5505`, longitude `-46.6333`, altitude `760.0` from `geoData`, not `10.0/20.0` | `src-tauri/src/takeout/parse.rs:170` — `assert_eq!(metadata.latitude, Some(-23.5505))`; `:171` — `assert_eq!(metadata.longitude, Some(-46.6333))`; `:172` — `assert_eq!(metadata.altitude, Some(760.0))` | ✅ PASS |
| TD-16 WHEN `people` is `[{ "name": "Pessoa A" }, { "name": "" }, { "name": "Pessoa B" }]` THEN the list is exactly `Pessoa A`, then `Pessoa B` | `vec!["Pessoa A", "Pessoa B"]` | `src-tauri/src/takeout/parse.rs:181` — `assert_eq!(metadata.people, vec!["Pessoa A".to_string(), "Pessoa B".to_string()])` | ✅ PASS |
| TD-17 WHEN `title`, `description`, or `url` is `""` or absent THEN that field is absent. WHEN `url` is `"https://photos.google.com/photo/x"` THEN the result contains exactly that text and discovery finishes without a network call | empty and missing text fields are `None`; url is that exact string | `src-tauri/src/takeout/parse.rs:190` — `assert_eq!(empty.title, None)` (and `:191` description, `:192` url); `:195` — `assert_eq!(missing.title, None)` (and `:196` description, `:197` url); `:201` — `assert_eq!(copied.url.as_deref(), Some(url))` with `url` bound to `"https://photos.google.com/photo/x"` | ✅ PASS |
| TD-18 WHEN `photoTakenTime.timestamp` is `"ontem"` THEN `taken_at_unix` is absent and the association remains | `taken_at_unix` `None` and parse returns `Ok` | `src-tauri/src/takeout/parse.rs:212` — `.expect("association remains")`; `:213` — `assert_eq!(metadata.taken_at_unix, None)` | ✅ PASS |

### P1: Separar o metadata.json de álbum

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| TD-19 WHEN `Viagem Europa 2022` contains `metadata.json`, `foto001.jpg`, and `foto001.jpg.json` THEN `foto001.jpg` associates to `foto001.jpg.json` with confidence `exact`, and `metadata.json` is not that media's JSON | `Exact` and JSON path `Viagem Europa 2022/foto001.jpg.json`, not `metadata.json` | `src-tauri/src/takeout/mod.rs:375` — `assert_eq!(media.confidence, Confidence::Exact)`; `:376` — `assert_eq!(media.json_relative_path.as_deref(), Some("Viagem Europa 2022/foto001.jpg.json"))`; `:380` — `assert_ne!(media.json_relative_path.as_deref(), Some("Viagem Europa 2022/metadata.json"))` | ✅ PASS |
| TD-20 WHEN `metadata.json` has title `"Viagem Europa 2022"`, description `""`, `date.timestamp` `"1660000000"`, and `geoData` `0.0/0.0` THEN the album has that title, absent description, `date_unix` `1660000000`, and no location | title `Viagem Europa 2022`, description `None`, `date_unix` `Some(1660000000)`, latitude/longitude/altitude `None` | `src-tauri/src/takeout/mod.rs:392` — `assert_eq!(album.metadata.title.as_deref(), Some("Viagem Europa 2022"))`; `:393` — `assert_eq!(album.metadata.description, None)`; `:394` — `assert_eq!(album.metadata.date_unix, Some(1660000000))`; `:395` — `assert_eq!(album.metadata.latitude, None)`; `:396` — `assert_eq!(album.metadata.longitude, None)`; `:397` — `assert_eq!(album.metadata.altitude, None)` | ✅ PASS |
| TD-21 WHEN `Photos from 2022` has no `metadata.json` THEN discovery does not create an album record | `report.albums` is empty | `src-tauri/src/takeout/mod.rs:403` — `assert!(report.albums.is_empty())` | ✅ PASS |
| TD-22 WHEN `metadata.json` is not valid JSON THEN discovery records its relative path with reason `invalid_json` and still associates the media in the folder | problem path `Viagem Europa 2022/metadata.json`, kind `InvalidJson`; media confidence `Exact`; no album | `src-tauri/src/takeout/mod.rs:414` — `assert_eq!(media.confidence, Confidence::Exact)`; `:415` — `assert!(report.albums.is_empty())`; `:416` — problem path `Viagem Europa 2022/metadata.json` and `ProblemKind::InvalidJson` | ✅ PASS |

### P1: Falhar por arquivo, sem tocar nos originais

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| TD-23 WHEN `foto.jpg.json` is not valid JSON and the folder also has `outra.jpg` with a valid `outra.jpg.json` THEN `foto.jpg` is `none`, `foto.jpg.json` is recorded with `invalid_json`, and `outra.jpg` is `exact` | broken `None` with no JSON path; problem path `foto.jpg.json` kind `InvalidJson`; neighbor `Exact` on `outra.jpg.json` | `src-tauri/src/takeout/mod.rs:427` — `assert_eq!(broken.confidence, Confidence::None)`; `:428` — `assert_eq!(broken.json_relative_path, None)`; `:430` — `assert_eq!(neighbor.confidence, Confidence::Exact)`; `:431` — `assert_eq!(neighbor.json_relative_path.as_deref(), Some("outra.jpg.json"))`; `:432` — problem path `foto.jpg.json` and `ProblemKind::InvalidJson` | ✅ PASS |
| TD-24 WHEN a `.json` file in the folder cannot be read THEN discovery records it with `unreadable` and still returns the other media | problem path `foto.jpg.json` kind `Unreadable`; blocked media `None`; neighbor `Exact` | `src-tauri/src/takeout/mod.rs:449` — `assert_eq!(blocked.confidence, Confidence::None)`; `:451` — `assert_eq!(neighbor.confidence, Confidence::Exact)`; `:453` — problem path `foto.jpg.json` and `ProblemKind::Unreadable` | ✅ PASS |
| TD-25 WHEN the folder contains `archive_browser.html` THEN that file does not appear as media, album, sidecar, or problem | media, albums, and problems are all empty. Fixture `td25` contains `archive_browser.html` | `src-tauri/src/takeout/mod.rs:461` — `assert!(report.media.is_empty())`; `:462` — `assert!(report.albums.is_empty())`; `:463` — `assert!(report.problems.is_empty())` | ✅ PASS |
| TD-26 WHEN `notas.txt` is in the folder THEN that file does not appear in the result | same empty report. Fixture `td25` contains `notas.txt` | `src-tauri/src/takeout/mod.rs:461` — `assert!(report.media.is_empty())`; `:462` — `assert!(report.albums.is_empty())`; `:463` — `assert!(report.problems.is_empty())` | ✅ PASS |
| TD-27 WHEN discovery finishes THEN paths, sizes, and contents of the tree are identical to before the call. Discovery does not create, rename, delete, or write a file | `tree_bytes` after equals `tree_bytes` before, including path, size, and bytes | `src-tauri/src/takeout/mod.rs:471` — `assert_eq!(tree_bytes(&root), before)` | ✅ PASS |

**Status**: ✅ All 27 acceptance criteria matched the spec-defined outcome. The extra-field edge now matches as well. See Edge Cases.

---

## Discrimination Sensor

Iteration 1 (unchanged, on `ee16cfb`) injected eight compiling mutations. Mutations 1–7 were killed. Mutation 8 survived because `unknown_field_is_ignored` did not lock the known fields.

Iteration 2 re-ran that mutant only, on a detached worktree of `e6fd16c`. The main working tree was not edited. The worktree was removed with `git worktree remove --force` and the temp directory was deleted.

| Mutation | File:line | Description | Killed? |
| -------- | --------- | ----------- | ------- |
| 1 | `src-tauri/src/takeout/parse.rs:53` | Flipped `latitude == 0.0 && longitude == 0.0` to `!=` | ✅ Killed (iteration 1) — `td13`, `td14`, `td15`, both `td20` tests |
| 2 | `src-tauri/src/takeout/matching.rs:19` | Off-by-one: truncation slice `[..46]` changed to `[..45]` | ✅ Killed (iteration 1) — matching `td04`/`td05`, discovery `td05`/`td08` |
| 3 | `src-tauri/src/takeout/mod.rs:97` | Removed the loop that records an unmatched sidecar as `OrphanJson` | ✅ Killed (iteration 1) — `td08_shared_truncated_json_is_ambiguous`, `unmatched_json_is_an_unread_orphan` |
| 4 | `src-tauri/src/takeout/mod.rs:154` | Invalid media JSON returned `Ok` with empty metadata instead of `InvalidJson` | ✅ Killed (iteration 1) — `td23_invalid_json_does_not_drop_the_neighbor` |
| 5 | `src-tauri/src/takeout/classify.rs:19` | `metadata.json` classified as `JsonSidecar` instead of album metadata | ✅ Killed (iteration 1) — `album_metadata_name_ignores_ascii_case`, `td20`, `td22` |
| 6 | `src-tauri/src/takeout/mod.rs:188` | Directory and file symlinks were followed instead of skipped | ✅ Killed (iteration 1) — `td27_discover_does_not_change_fixture_bytes` (`assert_symlink_is_not_followed`) |
| 7 | `src-tauri/src/takeout/mod.rs:140` | `discover` wrote `mutant-wrote.txt` into the tree before returning | ✅ Killed (iteration 1) — `td27_discover_does_not_change_fixture_bytes` |
| 8 | `src-tauri/src/takeout/parse.rs:15` | When `imageViews` is present, its string is written into `title` (`read_text(..., "imageViews").or_else(\|\| read_text(..., "title"))`) | ✅ Killed (iteration 2) — `unknown_field_is_ignored` panicked at `parse.rs:235`: `assert_eq!(metadata.title, None)` left `Some("12")`, right `None` |

**Sensor depth**: P0-full (library reads are non-destructive, AD-004). Eight behavior-level mutations. Iteration 2 re-injected only the mutant that survived iteration 1.
**Result**: 8/8 killed — ✅ PASS

The kill command was `cargo test --manifest-path src-tauri/Cargo.toml takeout::parse::tests::unknown_field_is_ignored` from the scratch worktree. Result: `FAILED. 0 passed; 1 failed; 0 ignored; 53 filtered out`.

---

## Interactive UAT Results (if performed)

Not performed. The feature is backend-only: discovery returns an in-memory report and has no UI.

| # | Test | Result | Details |
| --- | ---- | ------ | ------- |
| - | Interactive UAT | ⏭️ Skip | Backend-only. Automated gate and sensor are the check. |

---

## Code Quality

| Principle | Status |
| --------- | ------ |
| Minimum code | ✅ |
| Surgical changes | ✅ The feature range adds `takeout` plus fixtures. `lib.rs` only declares `mod takeout`. `e6fd16c` adds seven assertion lines in one test. |
| No scope creep | ✅ No UI, database, EXIF, or library-root scan. |
| Matches patterns | ✅ Tests are `#[cfg(test)]` modules beside the code, same as `db` and `health`. |
| Spec-anchored outcome check (asserted values match spec) | ✅ TD-01 through TD-27 match. The extra-field edge now asserts the known fields stay absent. |
| Per-layer Coverage Expectation met (domain 1:1 ACs; routes happy+edge+error) | ✅ Domain tests exist for each listed edge, including the extra-field edge. There is no route layer in this feature. |
| Every test maps to a spec requirement — no unclaimed tests | ✅ The 49 new tests map to TD-01–TD-27, the five spec edges, or a normative rule / task Done-when (UTF-8 cut, step-1 tie, `(0)`/`(01)` markers, symlink, non-directory root, integer timestamp). |
| Documented guidelines followed: none — strong defaults applied, bounded by `.specs/features/f02-takeout-discovery/spec.md` | ✅ |

No `// SPEC_DEVIATION` marker is present in the feature diff.

---

## Edge Cases

- [x] Media with no JSON stays in the result as `none`, without reason `invalid_json`. `src-tauri/src/takeout/mod.rs:486` — `assert_eq!(media.confidence, Confidence::None)`; `:487` — `assert_eq!(media.reason, None)`; `:489` — every problem kind is not `InvalidJson`. `hit` panics if the media is missing.
- [x] A `.json` that is not `metadata.json` and matches no step is `orphan_json`. `src-tauri/src/takeout/mod.rs:500` — `assert_eq!(report.problems[0].relative_path, "lonely.json")`; `:501` — `assert_eq!(report.problems[0].kind, ProblemKind::OrphanJson)`. The injected reader always errors, so a read would have become `Unreadable`.
- [x] Missing `people` yields an empty list. `src-tauri/src/takeout/parse.rs:221` — `assert_eq!(metadata.people, Vec::<String>::new())`.
- [x] Extra JSON field must not appear, and known fields must remain. Input in `unknown_field_is_ignored` is `photoTakenTime.timestamp` `1660142220` plus `"imageViews": "12"` and no title. Assertions: `src-tauri/src/takeout/parse.rs:233` — `assert_eq!(metadata.taken_at_unix, Some(1660142220))`; `:234` — `assert_eq!(metadata.creation_time_unix, None)`; `:235` — `assert_eq!(metadata.title, None)`; `:236` — `assert_eq!(metadata.description, None)`; `:237` — `assert_eq!(metadata.url, None)`; `:238` — `assert_eq!(metadata.latitude, None)`; `:239` — `assert_eq!(metadata.longitude, None)`; `:240` — `assert_eq!(metadata.altitude, None)`; `:241` — `assert_eq!(metadata.people, Vec::<String>::new())`. Copying `"12"` into `title`, `description`, or `url` makes that `Option<String>` `Some("12")`, which fails the matching `None` assert while `:233` still holds. Copying it into `people` yields a non-empty `Vec<String>` and fails `:241`. Latitude, longitude, and altitude are `Option<f64>` (`model.rs:17-19`), so the string cannot inhabit them; a numeric copy such as `Some(12.0)` fails `:238`–`:240`. Mutation 8 proved the title case: left `Some("12")`, right `None`.
- [x] A name ending in `-edited` whose original has no JSON is `none`. `src-tauri/src/takeout/mod.rs:508` — `assert_eq!(edited.confidence, Confidence::None)`; `:509` — `assert_eq!(edited.json_relative_path, None)`. Fixture `edge-edited` contains `foto.jpg` and `foto-edited.jpg` with no JSON.

---

## Gate Check

Re-run this iteration on the unmutated tree at `e6fd16c`.

- **Gate command**: `cargo test --manifest-path src-tauri/Cargo.toml` and `npm run build` and `node --experimental-strip-types --test src/health/view.test.ts`
- **Result**: Rust `54` passed, `0` failed, `0` skipped (`0 ignored`). Node `3` passed, `0` failed, `0` skipped. `npm run build` exit 0.
- **Test count before feature**: `5` (`git grep` of `#[test]` at `9f89b92`, the commit before `66158ff`)
- **Test count after feature**: `54` Rust library tests (`5` existing + `6` classify + `10` matching + `12` parse + `21` discover). `e6fd16c` added assertions, not a new test.
- **Delta**: `+49` new tests
- **Skipped tests**: none (`0 ignored` Rust, `0 skipped` Node)
- **Failures**: none on the unmodified tree

---

## Fix Plans (if issues found)

None. Iteration 1 Fix 1 is closed: `unknown_field_is_ignored` now names the absent known fields, and mutation 8 is killed.

---

## Requirement Traceability Update

Updated here only. `spec.md` was not rewritten.

| Requirement | Previous Status | New Status |
| ----------- | --------------- | ---------- |
| TD-01 | Done | ✅ Verified |
| TD-02 | Done | ✅ Verified |
| TD-03 | Done | ✅ Verified |
| TD-04 | Done | ✅ Verified |
| TD-05 | Done | ✅ Verified |
| TD-06 | Done | ✅ Verified |
| TD-07 | Done | ✅ Verified |
| TD-08 | Done | ✅ Verified |
| TD-09 | Done | ✅ Verified |
| TD-10 | Done | ✅ Verified |
| TD-11 | Done | ✅ Verified |
| TD-12 | Done | ✅ Verified |
| TD-13 | Done | ✅ Verified |
| TD-14 | Done | ✅ Verified |
| TD-15 | Done | ✅ Verified |
| TD-16 | Done | ✅ Verified |
| TD-17 | Done | ✅ Verified |
| TD-18 | Done | ✅ Verified |
| TD-19 | Done | ✅ Verified |
| TD-20 | Done | ✅ Verified |
| TD-21 | Done | ✅ Verified |
| TD-22 | Done | ✅ Verified |
| TD-23 | Done | ✅ Verified |
| TD-24 | Done | ✅ Verified |
| TD-25 | Done | ✅ Verified |
| TD-26 | Done | ✅ Verified |
| TD-27 | Done | ✅ Verified |

The extra-field edge is a spec consequence, not a new requirement id. Iteration 2 verified it. See Edge Cases.

---

## Summary

**Overall**: ✅ Ready

**Spec-anchored check**: 27/27 acceptance criteria matched the spec outcome. 0 spec-precision gaps. The extra-field edge was re-checked and matches.
**Sensor**: 8/8 mutations killed. Iteration 2 re-ran the `imageViews` → `title` mutant; it failed.
**Gate**: 54 Rust passed, 0 skipped; 3 Node passed, 0 skipped; build exit 0

**What works**: Same-folder matching for exact, case, stem, supplemental, truncation, `(N)`, and `-edited`; album `metadata.json` kept off the media sidecar; per-file `invalid_json` and `unreadable`; ignored files; orphan JSON; idempotent ordered discovery; an unchanged fixture tree, including a symlink that is not followed; and an unknown JSON string that cannot land in `title` or the other known fields.

**Issues found**: none.

**Next steps**: none. No lesson recorded (clean PASS).
