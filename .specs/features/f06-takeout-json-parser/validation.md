# F06 Takeout JSON Parser Validation

**Date**: 2026-10-06
**Spec**: `.specs/features/f06-takeout-json-parser/spec.md`
**Design**: `.specs/features/f06-takeout-json-parser/design.md`
**Diff range**: `4969f77..0ae4438` (one commit, `0ae4438`: spec, design, tasks, `takeout/parse.rs`, `takeout/model.rs`, `takeout/mod.rs`, `scan/time.rs` → `time.rs`, path-only changes in `scan/mod.rs`, `scan/walk.rs`, `scan/tests.rs`, `lib.rs`)
**Verifier**: independent sub-agent (author ≠ verifier)
**Environment**: Linux cloud container. `cargo test --lib -- takeout:: time:: scan::` → **108 passed, 1 failed, 1 ignored** (scan 37, takeout 66, time 5). The one failure is `takeout::tests::unreadable_subdirectory_is_a_problem_and_keeps_the_neighbor`: it needs Windows `icacls`, passes on Windows CI, and is not F06's. The ignored test is `real_takeout_survey` (manual).

**Verdict**: ❌ FAIL, narrowly.

The code does what the spec and design say when you read it, and I found no real bug. Every AC has a test. The FAIL is about discrimination: 4 of 32 mutants survive, and each changes behaviour the spec states:

- **M07**: a point with only *one* coordinate at `0.0` (equator or Greenwich meridian) is treated as invalid. The spec says only "não as duas `0.0`".
- **M28**: altitude `0.0` on a valid point is dropped. JSON-02 AC4 says "altitude é número → SHALL ser copiada", and Takeout often writes `0.0`.
- **M27**: the size cap is checked after the BOM is stripped, so a BOM file of 1 048 577–1 048 579 bytes is accepted.
- **M20**: `discover` goes back to an unbounded `fs::read`. JSON-05 AC1 / JSON-06 still pass, because only `read_json_capped` is tested in isolation.

All four fixes are test-only, about one assertion each (see [Required to reach PASS](#required-to-reach-pass)).

---

## Task Completion

| Task | Commit | Status | Notes |
| ---- | ------ | ------ | ----- |
| T1 Move ISO to `crate::time`, add `iso_from_unix_seconds` | `0ae4438` | ✅ Done | `time.rs:112-124`: range `[-5 364 662 400, 253 402 300 799]`, `format_iso` uses `div_euclid`/`rem_euclid` for negatives. `iso_utc` keeps its old behaviour (`None` before 1970, ms truncated). The old 4 `time` tests are unchanged, and `time.rs:187-216` adds 0, -1, -86 400, both range ends, one past each end, `i64::MIN/MAX`. `scan::` 37/37 green |
| T2 Models + parser hardening | `0ae4438` | ⚠️ Done, weak tests | Every AC has a test. Not pinned: one-zero coordinate (M07), altitude `0.0` (M28), cap-before-BOM order (M27) |
| T3 Capped reader in `discover` | `0ae4438` | ⚠️ Done, weak test | `mod.rs:26-40`. The read bound is proven on `read_json_capped` alone (`mod.rs:835-846`). Nothing proves that `discover` uses it (M20) |

`tasks.md` marks T1–T3 `[x]`, which matches the code.

---

## Spec-Anchored Acceptance Criteria

Test lines are in `src-tauri/src/takeout/parse.rs` unless another file is named.

### P1: Sidecar de mídia pronto para o banco (JSON-01..04)

| Criterion | Spec outcome | `file:line` + assertion | Result |
| --------- | ------------ | ----------------------- | ------ |
| AC1 `"1660142220"` or `1660142220` → unix + ISO; same for `creationTime` | `taken_at_unix = 1660142220`, `taken_at = "2022-08-10T14:37:00.000Z"`; `creation_time(_unix)` | `:330` loops over string and number. `:335` `taken_at_unix == Some(1_660_142_220)`, `:337-338` `taken_at == "2022-08-10T14:37:00.000Z"`, `:340-343` creation unix + `"2022-08-10T15:37:00.000Z"` | ✅ PASS. `creationTime` as a number is covered at `:374` |
| AC2 `0`, non-numeric, out of 1800–9999 → pair absent, rest read; `-86400` → `"1969-12-31T00:00:00.000Z"` | | `:350-360` `"0"`, `0`, `"ontem"`, `" 1660142220"`, `"+1"`, `"-5364662401"`, `253402300800`, `1.5`, `null`. `:364-366` `(taken_at_unix, taken_at) == (None, None)`. `:369` `title == "ainda lido"` (rest still read). `:372-373` `-86_400` → `"1969-12-31T00:00:00.000Z"`. `:376-382` range ends `1800-01-01T00:00:00.000Z` and `9999-12-31T23:59:59.000Z` accepted. `time.rs:212-213` one past each end → `None` | ✅ PASS (M09–M12, M21–M23, M30, M31 killed) |
| AC3 location source: `geoData` valid → `takeout_geoData`; otherwise `geoDataExif` valid → `takeout_geoDataExif`; otherwise all absent | | `:392` `location_source == GeoData` with both present. `:398-401` `as_column()` strings. `:407-416` fallback for `0/0`, lat 90.5 / -91, lon 180.01 / -181, string lat, missing lon, non-object, missing `geoData`. `:421-430` source `GeoDataExif` + exif lat/lon/alt. `:440-447` both invalid → `(None, None, None, None)` | ⚠️ PASS for the listed cases. **Not pinned**: a valid point with one coordinate exactly `0.0` (M07 survives). Spec: "ambas … e não as duas `0.0`" |
| AC4 valid point + numeric altitude → copied; otherwise absent | | `:397`, `:430` altitude `760.0` copied. `:454-456` `"alto"` → `None` with a valid point. `td13` (`:183-191`) `0/0` drops altitude. `:440-447` no point → altitude `None` | ⚠️ PASS for the listed cases. **Not pinned**: altitude `0.0` on a valid point (M28 survives) |
| AC5 `" Ana  Maria "`, `"ana maria"`, `""`, `"   "`, `"Bruno"` → `["Ana Maria","Bruno"]`, normalized `"ana maria"`, `"bruno"` | | `:468-470` the spec list, plus `"BRUNO"`, `"Caio \tLima"`, `{other}`, `{name: 3}`. `:472` `people == ["Ana Maria","Bruno","Caio Lima"]`. `:478` normalized `["ana maria","bruno","caio lima"]`. `:479` `normalized_person_name("ÉLISE") == "élise"` (Unicode, not ASCII) | ✅ PASS (M14–M16, M26 killed) |
| AC6 text trimmed; empty → absent; inner newlines kept | | `:491-493` `"IMG_1.jpg"`, `"linha 1\nlinha 2"`, `"https://x/y"`. `:494-497` whitespace-only → `(None, None, None)` | ✅ PASS (M13, M24 killed) |
| AC7 `raw_json` = file text byte for byte, without a leading BOM | | `:503-504` text with odd spacing and `á` → `raw_json == text`. `:505-508` BOM + text → `raw_json == text` | ✅ PASS (M02, M17 killed). The irregular whitespace makes a re-serialization visible |

### P1: Álbum normalizado (JSON-04)

| Criterion | Spec outcome | `file:line` + assertion | Result |
| --------- | ------------ | ----------------------- | ------ |
| AC1 `date.timestamp` → `date_unix` + ISO, JSON-01 rules | | `:522-523` `1_660_000_000`, `"2022-08-08T23:06:40.000Z"`. `:530`, `:534` `"0"` → `(None, None)` | ✅ PASS (M25 killed). Out-of-range album dates are not tested separately, but they go through the same `instant_at` |
| AC2 valid `geoData` → copied; invalid → absent; no `geoDataExif` | | `:525-526` `(48.8584, 2.2945, 35.0)`. `:530-537` lat 91 with a valid `geoDataExif` → `(None, None, None)` | ✅ PASS (M29 killed) |
| AC3 title/description trim / empty | | `:520` `" Viagem "` → `"Viagem"`. `:521` `"  "` → `None` | ✅ PASS |

### P1: Sidecar hostil não derruba nada (JSON-05, JSON-06)

| Criterion | Spec outcome | `file:line` + assertion | Result |
| --------- | ------------ | ----------------------- | ------ |
| AC1 > 1 048 576 bytes → `parse_*` fails; reader reads ≤ 1 048 577 | | `:552`, `:554` `MAX+1` → `is_err()` for media and album. `:555` `MAX_SIDECAR_BYTES == 1_048_576`. `mod.rs:837-843` endless reader: `bytes.len() == MAX+1` **and** `served == MAX+1` (counts bytes actually pulled) | ⚠️ PASS for `parse_*` and `read_json_capped`. **Not pinned**: (a) that `discover` uses `read_json_capped` (M20 survives); (b) that the cap counts the BOM bytes, so a file > 1 MiB with a BOM still fails (M27 survives) |
| AC2 exactly 1 048 576 bytes of valid JSON → accepted | | `:543-551`, `:553` `fill(MAX)` (`assert_eq!(bytes.len(), total)`) → `is_ok()` for both parsers | ✅ PASS (M01 killed) |
| AC3 leading BOM ignored | | `:507` media parses, `:509` `title == "á"`. `:573-574` `BOM{}` OK for both. `:567` BOM alone → `Err`. `mod.rs:860` neighbour sidecar with a BOM is matched in `discover` | ✅ PASS |
| AC4 non-object root → invalid | | `:560-571` `[]`, `"x"`, `3`, `null`, `true`, empty, BOM-only, invalid UTF-8 → `is_err()` for both parsers | ✅ PASS (M03 killed) |
| AC5 `discover`: oversize / non-object → `invalid_json`, media unmatched, neighbours matched (JSON-06) | | `mod.rs:849-889`. Disk: `grande.jpg.json` (MAX+2 bytes), `lista.jpg.json` (`[]`), `vizinha.jpg.json` (BOM). `:876` invalid exactly `["grande.jpg.json","lista.jpg.json"]`. `:877` `problems.len() == 2`. `:879-881` both `Confidence::None` and `metadata.is_none()`. `:884` neighbour `Exact`. `:885-888` neighbour `taken_at` ISO | ✅ PASS for the report. The test cannot tell whether `discover` read the oversize file capped or whole (M20) |

---

## Edge Cases

| Edge case | Evidence | Result |
| --------- | -------- | ------ |
| `"BRUNO"`, `"Bruno"` → `["BRUNO"]`, normalized `"bruno"` | `:480-481` `first.people == ["BRUNO"]`. Normalization at `:478` | ✅ PASS (M15 killed) |
| `people` not a list / item without a text `name` → ignored | `:482-483` object → empty. `:470` `{ "other": 1 }`, `{ "name": 3 }` absent from `:472`. `missing_people_is_an_empty_list` `:266-269` | ✅ PASS |
| Timestamp string with spaces or `+` → absent | `:354-355` `" 1660142220"`, `"+1"` | ✅ PASS (M21, M31 killed) |
| Lat ±90, lon ±180 valid | `:454-457` `(90, -180)` with source `GeoData`. `:458-462` `(-90, 180)` | ✅ PASS (M04, M05 killed) |

---

## F02 Regression Check

**TD tests.** `git diff -w 0ae4438^ 0ae4438 -- src-tauri/src/takeout/parse.rs` shows that the TD tests (`td11`–`td18`, `td20`, `missing_people_is_an_empty_list`, `unknown_field_is_ignored`, `album_bytes_that_are_not_json_return_an_error`) changed only in rustfmt line wrapping. No assertion changed. The `mod.rs` TD tests (`td01`–`td10`, `td19`–`td24`, `td27`) are untouched. All pass, except the icacls one noted above.

**Changes to F02 output.** I compared the old `parse.rs` (`0ae4438^`) with the new one. Every change to the output is one the F06 spec calls for:

| F02 output change | Spec clause |
| ----------------- | ----------- |
| `taken_at_unix` / `creation_time_unix` / album `date_unix`: `0` and years outside 1800–9999 → `None` | JSON-01 AC2 ("o par … ausente"); album AC1 ("regras de JSON-01") |
| Coordinates outside [-90, 90] / [-180, 180] → no point; fall back to `geoDataExif` | JSON-02 AC3 |
| `title`/`description`/`url` trimmed; whitespace-only → `None` | JSON-03 AC6; album AC3 |
| `people`: whitespace collapsed, blank dropped, deduplicated by normalized name | JSON-03 AC5 + edge case |
| Leading BOM accepted | JSON-05 AC3 |
| Non-object root → `Err` (was `Ok` with every field empty) | JSON-05 AC4 |
| > 1 MiB → `Err` | JSON-05 AC1 |
| New fields `taken_at`, `creation_time`, `location_source`, `raw_json`, album `date` | JSON-01, -02, -04 |

No unrequested change. Two notes:

- Invalid UTF-8 was already rejected by `serde_json::from_slice`, so the new `str::from_utf8` step changes nothing.
- In `discover`, an open or read error is still `Unreadable` (`mod.rs:27`, `:37`, `:191-192`). A directory named `*.json` was `Unreadable` before and still is (`File::open` succeeds on Linux, then `read_to_end` fails with EISDIR).

**`scan::` moves.** `scan/time.rs` → `time.rs`. `iso_utc` now calls `format_iso(since.as_secs() as i64, millis)`. For any `SystemTime` after the epoch this gives the same text as the old `u64` arithmetic: the cast cannot overflow for real times, and `div_euclid`/`rem_euclid` equal `/`/`%` for non-negative values. `scan/mod.rs`, `walk.rs` and `tests.rs` change only `use` paths. `scan::` 37/37 green.

---

## Implementation Review

What I confirmed by reading the code:

- **Timestamp** (`parse.rs:89-121`). String: optional `-`, then ASCII digits only. `+`, spaces, empty string and `"-"` are rejected. `"-0"` → 0 → absent. Number: `as_i64` only, so `1.5`, `1e3` and a `u64` above `i64::MAX` are absent. `0` is filtered, then the range is checked by `iso_from_unix_seconds`. Unix and ISO are set or cleared together.
- **Range constants** (`time.rs:114-115`). `-5 364 662 400` = 1800-01-01T00:00:00Z and `253 402 300 799` = 9999-12-31T23:59:59Z, as the tests show. Negative instants format correctly through `div_euclid`/`rem_euclid` (`time.rs:127-128`).
- **Location** (`parse.rs:15-26`, `:76-86`). `find_map` over `[geoData, geoDataExif]` gives the source exactly when lat/lon are set. `-0.0/-0.0` counts as `0/0`. `serde_json` (default features, `Cargo.toml:26`) rejects `1e400` as invalid JSON, so NaN or infinity cannot occur. The album uses `geoData` only.
- **People** (`parse.rs:124-150`). `split_whitespace` (Unicode `White_Space`, which includes U+00A0 and U+3000) collapses runs, then the name is deduplicated by `to_lowercase` with first-wins, in JSON order. O(n²) `Vec::contains` is fine for sidecar sizes.
- **Text** (`parse.rs:152-155`). `str::trim` (Unicode whitespace); inner newlines kept.
- **decode** (`parse.rs:63-73`). Size check on the raw bytes first, then one BOM stripped, then UTF-8, then JSON, then object root. `raw_json` is the `&str` slice after the BOM, not re-serialized.
- **Capped reader** (`mod.rs:33-40`). `Read::take(MAX+1)` makes `read_to_end` ask the inner reader for at most MAX+1 bytes in total. The counting reader in the test confirms this. Album `metadata.json` goes through the same `read_json` closure (`mod.rs:157`), so it is capped too.

Observations (not bugs):

| # | Severity | Where | Finding |
| - | -------- | ----- | ------- |
| O1 | Info | `parse.rs:64-67` | The cap counts the BOM (raw file size), which matches the spec text "o arquivo tem mais de 1 048 576 bytes". So a BOM file with exactly 1 MiB of JSON after the BOM is rejected. That is correct, but no test pins it (M27) |
| O2 | Info | `model.rs:52` | `to_lowercase` is context-free (`"İ"` → `"i̇"`, final sigma is not special). This is acceptable under "minúsculas Unicode" |
| O3 | Info | `parse.rs:44-60` | `AlbumMetadata` has no `raw_json`. The spec asks for `raw_json` only for the media sidecar (P1 media AC7), so this is consistent |

---

## Discrimination Sensor (mutation log)

Each mutant was applied alone with a `perl` substitution, checked as applied (`git diff` non-empty), and then `cargo test --lib -- takeout:: time:: scan::` was run. The file was restored each time with `git checkout -- <file>`. Baseline: 1 failure (the icacls test). A mutant counts as **killed** only when failures exceed 1. Afterwards `git status --short` showed only this report.

| ID | File | Mutation | Result | Killed by |
| -- | ---- | -------- | ------ | --------- |
| M01 | parse.rs:64 | cap `>` → `>=` (exactly 1 MiB rejected) | ✅ Killed | `json05_size_cap_is_inclusive` |
| M02 | parse.rs:67 | no BOM strip | ✅ Killed | `json04_raw_json…`, `json05_root…`, `json06_…` |
| M03 | parse.rs:71 | non-object root → `Ok` with empty map | ✅ Killed | `json05_root…`, `json06_…` |
| M04 | parse.rs:79 | latitude range end-exclusive | ✅ Killed | `json02_range_ends…` |
| M05 | parse.rs:79 | longitude range widened to ±181 | ✅ Killed | `json02_invalid_geo_data_falls_back_to_exif` |
| M06 | parse.rs:80 | `0/0` no longer invalid | ✅ Killed | td13, td14, td20 (×2), `json02_…` (×2) |
| **M07** | parse.rs:80 | `lat == 0 && lon == 0` → `\|\|` (one zero invalidates) | ❌ **Survived** | — JSON-02 AC3 / Assumptions: "não as duas `0.0`". A photo on the equator or the Greenwich meridian (exactly `0.0`) would lose its location or fall to `geoDataExif` |
| M08 | parse.rs:17 | `geoDataExif` labelled `GeoData` | ✅ Killed | `json02_invalid_geo_data_falls_back_to_exif` |
| M09 | parse.rs:94 | `0` not filtered | ✅ Killed | `json01_zero_junk…`, `json04_album…` |
| M10 | time.rs:114 | `FIRST_SECOND` + 1 | ✅ Killed | `json01_zero_junk…`, `time::unix_seconds…` |
| M11 | time.rs:115 | `LAST_SECOND` + 1 | ✅ Killed | same |
| M12 | time.rs:120 | no range check | ✅ Killed | same |
| M13 | parse.rs:153 | `trim` → `trim_end` | ✅ Killed | `json03_text…`, `json04_album…` |
| M14 | parse.rs:125 | `clean_name` = `trim` only (no collapse) | ✅ Killed | `json03_people…` |
| M15 | parse.rs:144 | no dedup | ✅ Killed | `json03_people…` |
| M16 | model.rs:53 | `to_ascii_lowercase` | ✅ Killed | `json03_people…` (`ÉLISE`) |
| M17 | parse.rs:40 | `raw_json` re-serialized from `Value` | ✅ Killed | `json04_raw_json…` |
| M18 | mod.rs:36 | `take(MAX)` (no +1) | ✅ Killed | `json05_capped_reader…` |
| M19 | mod.rs:36 | `take(MAX * 2)` | ✅ Killed | `json05_capped_reader…` |
| **M20** | mod.rs:27 | `discover` uses `fs::read` (unbounded) | ❌ **Survived** | — JSON-05 AC1 "o leitor SHALL NOT ler mais que 1 048 577 bytes" / JSON-06 "`discover` usa o leitor limitado". The oversize sidecar is still rejected by the parser, so the report is identical |
| M21 | parse.rs:116 | accept a leading `+` | ✅ Killed | `json01_zero_junk…` |
| M22 | parse.rs:94 | negatives rejected (`> 0`) | ✅ Killed | `json01_zero_junk…` |
| M23 | time.rs:128 | `rem_euclid` → `%` | ✅ Killed | `time::unix_seconds…` |
| M24 | parse.rs:153 | text cleaned like names (newlines collapsed) | ✅ Killed | `json03_text…` |
| M25 | parse.rs:50 | album `date` ISO never set | ✅ Killed | `json04_album…` |
| M26 | parse.rs:126 | blank name kept | ✅ Killed | `json03_people…`, td16 |
| **M27** | parse.rs:64-67 | strip the BOM **before** the size check | ❌ **Survived** | — JSON-05 AC1: a 1 048 579-byte file (BOM + 1 MiB JSON) would be accepted. Low impact, but spec-visible |
| **M28** | parse.rs:84 | altitude `0.0` dropped on a valid point | ❌ **Survived** | — JSON-02 AC4 "altitude é número → SHALL ser copiada". Takeout commonly writes `"altitude": 0.0` |
| M29 | parse.rs:46 | album falls back to `geoDataExif` | ✅ Killed | `json04_album…` |
| M30 | parse.rs:107 | number timestamp via `as_f64 as i64` (accepts `1.5`) | ✅ Killed | `json01_zero_junk…` |
| M31 | parse.rs:106 | string timestamp trimmed before parse | ✅ Killed | `json01_zero_junk…` |
| M32 | model.rs:24 | `as_column` typo `takeout_geoDataexif` | ✅ Killed | `json02_geo_data_wins…` |

**Totals**: 32 mutants (21 in parse.rs, 4 in time.rs, 3 in model.rs, 4 in mod.rs/`read_json_capped`/`discover`). **28 killed**, **4 spec-visible survivors (M07, M20, M27, M28)**, 0 equivalent.

---

## Not Run

| Item | Reason |
| ---- | ------ |
| `takeout::tests::unreadable_subdirectory_is_a_problem_and_keeps_the_neighbor` | Needs Windows `icacls`; fails on Linux in the baseline; passes on Windows CI. Not F06 |
| `takeout::tests::real_takeout_survey` | `#[ignore]`, manual against a real export |
| Full `cargo test` | 42 known Linux-only failures in `bench::`, `db::`, `takeout::`; not F06's |

---

## Required to reach PASS

All test-only. No production change is needed unless the author prefers one.

1. **M07 (JSON-02 AC3).** In `parse.rs` tests, assert that `geoData {latitude: 0.0, longitude: 10.0}` and `{latitude: 10.0, longitude: 0.0}` are valid with source `GeoData`.
2. **M28 (JSON-02 AC4).** Assert that a valid point with `"altitude": 0.0` gives `altitude == Some(0.0)`.
3. **M27 (JSON-05 AC1).** Assert that `BOM + fill(MAX)` (1 048 579 bytes) is `Err`, and that `BOM + fill(MAX - 3)` (exactly 1 048 576 bytes) is `Ok`.
4. **M20 (JSON-05 AC1 / JSON-06).** Prove that the reader `discover` uses is the capped one. One option: pull the closure out into a named `fn read_sidecar(path: &Path) -> Result<Vec<u8>, String>` that `discover` passes, then test it on a sparse file (`File::set_len(64 MiB)`) and assert `len == MAX + 1`. Optionally add an oversize album `metadata.json` to the JSON-06 tree.

## Requirement Status

| Requirement | Status |
| ----------- | ------ |
| JSON-01 | ✅ Verified |
| JSON-02 | ❌ Needs tests: M07 (one zero coordinate), M28 (altitude `0.0`) |
| JSON-03 | ✅ Verified |
| JSON-04 | ✅ Verified |
| JSON-05 | ⚠️ Needs tests: M27 (cap counts the BOM); AC1 read bound proven only on `read_json_capped` (M20) |
| JSON-06 | ⚠️ Report outcome verified; capped reader wiring not pinned (M20) |

**Overall**: ❌ FAIL. No real bug was found: the implementation matches the spec, and F02 behaviour holds, changed only where F06 says so. But 4 spec-visible mutants survive (M07, M20, M27, M28). Four small tests would close them.

---

## Iteration 2 (re-verification)

**Diff range**: `0ae4438..c04906c` (`c04906c`). Tests were added in `parse.rs` and `mod.rs`. `discover` now passes a named `read_sidecar` (`mod.rs:25-32`). No other production change: I read `git show c04906c -- src-tauri/src`, and the only non-test hunk is the extraction of `read_sidecar`, which has the same body as the old closure.
**Gate**: `cargo test --lib -- takeout:: time:: scan::` → **111 passed, 1 failed, 1 ignored** (108 before, plus 3 new tests). The failure is still only the icacls test (`unreadable_subdirectory_is_a_problem_and_keeps_the_neighbor`).

### Iteration-1 survivors, re-checked

| Mutant | New test (`file:line` + assertion) | Result |
| ------ | ---------------------------------- | ------ |
| M07 one zero coordinate invalid | `parse.rs:578` `json02_one_zero_coordinate_is_a_real_place_and_zero_altitude_is_kept`: `(0.0, 10.0)` and `(10.0, 0.0)` → source `GeoData`, lat/lon kept | ✅ Killed |
| M28 altitude `0.0` dropped | same test, `parse.rs:585` `altitude == Some(0.0)` | ✅ Killed |
| M27 BOM stripped before the cap | `parse.rs:590` `json05_the_bom_counts_toward_the_size_cap`: BOM + JSON = exactly 1 048 576 → `Ok` (length asserted); 1 048 577 → `Err` (`:600`); BOM + 1 MiB → `Err` (`:601`) | ✅ Killed |
| M20b `read_sidecar` body → `fs::read` | `mod.rs:895` `json05_discover_reads_a_huge_sidecar_only_up_to_the_cap`: sparse 64 MiB file, `read_sidecar(..).len() == MAX + 1` (`:905`); missing file → `Err` (`:903`) | ✅ Killed |
| M20c `read_sidecar` with an uncapped `read_to_end` | same | ✅ Killed |
| **M20a** `discover` bypasses `read_sidecar` (`discover_with(root, \|p\| fs::read(p)…)` at `mod.rs:26`) | — | ❌ **Survived** |

### M20a is visible in the report

The new test calls `read_sidecar` directly, so it proves the helper is capped but not that `discover` uses it. Whether a bypass matters to the spec, rather than only to memory use, was settled by a probe. I appended a temporary test, ran it, and removed it with `git checkout`. The probe ran `discover` on a temp dir holding `a.jpg` and an `a.jpg.json` sparse to 1 TiB (`set_len(1 << 40)` → `Ok` on this ext4/overlay disk):

| Code | `report.problems` |
| ---- | ----------------- |
| HEAD (`read_sidecar`) | `[a.jpg.json: InvalidJson]` |
| M20a (`fs::read`) | `[a.jpg.json: Unreadable]`. `fs::read` pre-allocates the file length and fails with out-of-memory |

JSON-05 AC5 / JSON-06 say a sidecar over the cap SHALL be `invalid_json` in the `discover` report. So M20a changes spec-visible output, and no test catches it. This is the original M20 finding, moved from the closure to the call site.

**Fix (test only).** Add a `discover`-level variant of the probe: `a.jpg` plus `a.jpg.json` sparse to 1 TiB. Assert that the problem kind is `InvalidJson` and that `a.jpg` has `Confidence::None`. Mark it `#[cfg(unix)]`, or skip when `set_len` fails: on NTFS a non-sparse `set_len` reserves real clusters. The 64 MiB `read_sidecar` test can stay as it is.

### Full mutation set, re-run

All 32 iteration-1 mutants were re-applied with the same patterns, plus M20a/M20b/M20c in place of M20. Each was applied alone and checked as applied. `cargo test --lib -- takeout:: time:: scan::` was then run, and the file was restored with `git checkout -- <file>`. A mutant is killed only if there are more than 1 failure. Afterwards `git status --short` showed only this report.

| IDs | Result |
| --- | ------ |
| M01–M19, M21–M32 except M20 (31 mutants, incl. M07, M27, M28) | ✅ All killed. Killing tests as in iteration 1; M01 and M02 are now also killed by `json05_the_bom_counts…`, and M18 and M19 also by `json05_discover_reads…` |
| M20b, M20c | ✅ Killed |
| **M20a** | ❌ Survived (report-visible, see above) |

**Totals**: 34 mutants. **33 killed**, **1 spec-visible survivor (M20a)**.

### Regressions

None. The `scan::` (37), `time::` (5) and `takeout::` TD tests are unchanged and green. The 3 new tests pass. The production diff is only the `read_sidecar` extraction, so the behaviour is identical.

### Requirement status (iteration 2)

| Requirement | Iteration 1 | Iteration 2 |
| ----------- | ----------- | ----------- |
| JSON-01 | ✅ | ✅ Verified |
| JSON-02 | ❌ (M07, M28) | ✅ Verified |
| JSON-03 | ✅ | ✅ Verified |
| JSON-04 | ✅ | ✅ Verified |
| JSON-05 | ⚠️ (M27, M20) | ✅ Verified (`parse_*` cap, BOM counting, and the `read_sidecar` bound all pinned) |
| JSON-06 | ⚠️ (M20) | ❌ Needs one test: `discover` wiring of the capped reader (M20a) |

### Summary (iteration 2)

**Overall**: ❌ FAIL, on one narrow item. M07, M27 and M28 are killed, and M20 is killed where it now lives (inside `read_sidecar`). But replacing `read_sidecar` at the `discover` call site still survives, and it changes the report for a huge sidecar from `invalid_json` to `unreadable`. One `discover`-level test with a 1 TiB sparse sidecar (Unix only) closes it. No implementation bug was found. With that test added, the verdict would be PASS.

---

## Iteration 3 (re-verification)

**Diff range**: `c04906c..3d37bc7` (`3d37bc7`). It adds one test, `mod.rs:912` `json06_discover_reports_a_terabyte_sidecar_as_invalid_json` (`#[cfg(unix)]`), and commits the iteration-2 report. No production change: `git show 3d37bc7 -- src-tauri/src` touches only the `mod tests` block.
**Gate**: `cargo test --lib -- takeout:: time:: scan::` → **112 passed, 1 failed, 1 ignored** (111 before, plus 1 new test). The failure is still only the icacls test.

### M20a, re-checked

| Check | Evidence | Result |
| ----- | -------- | ------ |
| The new test really runs here and is not skipped | `cargo test --lib -- json06_discover_reports_a_terabyte --nocapture` → `... ok`, and the `skipped` message is **not** printed, so `set_len(1 << 40)` succeeded and the assertions ran | ✅ Runs |
| It pins the spec outcome through `discover` | `assert_eq!(report.problems.len(), 1)`; problem `a.jpg.json` with kind `ProblemKind::InvalidJson`; `hit(&report, "a.jpg").confidence == Confidence::None` (JSON-06 AC5: `invalid_json`, media unmatched) | ✅ Discriminating |
| M20a: `discover_with(root, \|p\| fs::read(p)…)` at `mod.rs:26` | 111 passed, **2 failed**. The new test fails (kind becomes `Unreadable`) | ✅ **Killed** |

### Spot-check of the earlier mutants

Each was re-applied alone, the gate was run, and the file was restored with `git checkout -- <file>`.

| ID | Result | Killed by |
| -- | ------ | --------- |
| M20b `read_sidecar` → `fs::read` | ✅ Killed | `json05_discover_reads_a_huge_sidecar…`, `json06_discover_reports_a_terabyte…` |
| M18 `take(MAX)` | ✅ Killed | `json05_capped_reader…`, `json05_discover_reads_a_huge_sidecar…` |
| M01 cap `>=` | ✅ Killed | `json05_size_cap_is_inclusive`, `json05_the_bom_counts…` |
| M07 one zero coordinate | ✅ Killed | `json02_one_zero_coordinate…` |
| M27 BOM before the cap | ✅ Killed | `json05_the_bom_counts…` |
| M28 altitude `0.0` dropped | ✅ Killed | `json02_one_zero_coordinate…` |

The other 27 mutants from iteration 2 run against production code that has not changed since then (`3d37bc7` only adds a test), so they stay killed. Afterwards `git status --short` showed only this report modified.

Note: M18 alone does not trip the new 1 TiB test, because a sidecar truncated to 1 MiB of zero bytes is still invalid JSON. It is killed by the two reader tests, so this is not a gap.

### Regressions

None. `scan::` (37), `time::` (5) and every `takeout::` TD and F06 test pass. The only failure is the known icacls test.

### Not run (iteration 3)

| Item | Reason |
| ---- | ------ |
| `unreadable_subdirectory_is_a_problem_and_keeps_the_neighbor` | Windows `icacls`; Windows CI |
| `json06_discover_reports_a_terabyte…` on Windows | `#[cfg(unix)]` by design (NTFS `set_len` reserves real space). On Windows, JSON-06 wiring is covered only by the `read_sidecar` test plus reading `mod.rs:26` |
| `real_takeout_survey` | `#[ignore]`, manual |
| Full `cargo test` | 42 known Linux-only failures outside F06 |

### Requirement status (final)

| Requirement | Iteration 1 | Iteration 2 | Iteration 3 |
| ----------- | ----------- | ----------- | ----------- |
| JSON-01 | ✅ | ✅ | ✅ Verified |
| JSON-02 | ❌ | ✅ | ✅ Verified |
| JSON-03 | ✅ | ✅ | ✅ Verified |
| JSON-04 | ✅ | ✅ | ✅ Verified |
| JSON-05 | ⚠️ | ✅ | ✅ Verified |
| JSON-06 | ⚠️ | ❌ | ✅ Verified (on Unix; `#[cfg(unix)]` test) |

### Summary (final)

**Overall**: ✅ PASS.
**Spec-anchored check**: every JSON-01..06 AC and all 4 edge cases have tests that pin the spec's outcome. F02 behaviour holds, and changes only where F06 says so.
**Sensor**: 34 mutants, 34 killed, 0 survivors.
**Bugs**: none found.
**Gate**: 112 passed. The 1 failure is the Windows-only icacls test.
**Fix→re-verify iterations used**: 3.
