# F03 Performance Baseline Validation

**Date**: 2026-09-28
**Spec**: `.specs/features/f03-performance-baseline/spec.md`
**Diff range**: `ca6c5c3..4f749ad` (first feature commit `ac74d48`, HEAD `4f749ad`)
**Verifier**: independent sub-agent (author ≠ verifier)

---

## Task Completion

| Task | Status | Notes |
| ---- | ------ | ----- |
| T1 | ✅ Done | Report types and error codes exist; no tests added |
| T2 | ✅ Done | Byte and path vectors match the normative formulas |
| T3 | ✅ Done | `classify_seek_penalty` and `volume_root_of` match the three inputs and `D:\` |
| T4 | ✅ Done | Unsafe roots return the stable codes and leave markers untouched |
| T5 | ✅ Done | Known SHA-256 vectors and the 64-file aggregate match the spec hex |
| T6 | ✅ Done | Live query classifies without panicking; failure maps to `None` → `unknown` |
| T7 | ✅ Done | 64-file run, cleanup, junction, and marker behavior match the corpus and hash criteria |
| T8 | ✅ Done | Unreadable files, setup failure, and cleanup failure match the error contract |
| T9 | ✅ Done | CLI exit codes and the count-1 JSON file match the task done-when |
| T10 | ✅ Done | Four official JSON reports are present; the HDD corpus path is gone |

All tasks in `tasks.md` are checked done. None are blocked or partial.

---

## Spec-Anchored Acceptance Criteria

### P1: Corpus que não toca o acervo (PERF-01)

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| WHEN the count is 0 or greater than 150000 THEN `count_out_of_range` and no file is created or deleted | Error code `count_out_of_range`; counts 1 and 150000 are accepted; the root is not created | `src-tauri/src/bench/corpus.rs:118` — `assert_eq!(validate(&root, 0).unwrap_err().as_code(), "count_out_of_range")`; `corpus.rs:122` — `assert_eq!(…150_001…, "count_out_of_range")`; `corpus.rs:126` — `assert!(validate(&root, 1).is_ok())`; `corpus.rs:127` — `assert!(validate(&root, 150_000).is_ok())`; `corpus.rs:128` — `assert!(!root.exists())` | ✅ PASS |
| WHEN the root is relative or UNC THEN `root_not_absolute` and no file is created or deleted | Error code `root_not_absolute` | `src-tauri/src/bench/corpus.rs:134` — `assert_eq!(validate(Path::new(r"relative\root"), 1).unwrap_err().as_code(), "root_not_absolute")`; `corpus.rs:140` — `assert_eq!(validate(Path::new(r"\\server\share\root"), 1)…, "root_not_absolute")`; a `..` path keeps a marker at `corpus.rs:154` — `assert_eq!(…as_code(), "root_not_absolute")` and `corpus.rs:158` — `assert_eq!(fs::read(&marker).unwrap(), b"keep")` | ✅ PASS |
| WHEN the normalized root is only a volume root THEN `root_is_volume` and no file is created or deleted | Error code `root_is_volume` for `C:\`, `C:`, and `\\?\C:\` | `src-tauri/src/bench/corpus.rs:165` — `assert_eq!(validate(Path::new(root), 1).unwrap_err().as_code(), "root_is_volume")` | ✅ PASS |
| WHEN the parent does not exist THEN `parent_missing` and nothing is created | Error code `parent_missing`; parent and root stay absent | `src-tauri/src/bench/corpus.rs:178` — `assert_eq!(…as_code(), "parent_missing")`; `corpus.rs:182` — `assert!(!absent_parent.exists())`; `corpus.rs:183` — `assert!(!root.exists())` | ✅ PASS |
| WHEN the root exists and has any entry THEN `root_not_empty` and the marker is unchanged | Error code `root_not_empty`; marker bytes stay `keep` | `src-tauri/src/bench/corpus.rs:203` — `assert_eq!(…as_code(), "root_not_empty")`; `corpus.rs:207` — `assert_eq!(fs::read(&marker).unwrap(), b"keep")`; `src-tauri/src/bench/mod.rs:527` — `assert_eq!(error.as_code(), "root_not_empty")`; `mod.rs:528` — `assert_eq!(fs::read(&marker).unwrap(), b"keep")` | ✅ PASS |
| WHEN a valid run finishes THEN files exist only under that root and, after successful cleanup, the root is gone | `ok` true with the 64-file contract, the outside marker unchanged, and `!root.exists()` | `src-tauri/src/bench/mod.rs:322` — `assert_success(&report)` (see `mod.rs:532`–`539`); `mod.rs:323` — `assert!(!root.exists())`; `mod.rs:335` — `assert_success` on a pre-created empty root; `mod.rs:336` — `assert!(!root.exists())`; `mod.rs:372` — `assert_eq!(fs::read(&marker).unwrap(), b"keep-outside")` | ✅ PASS |
| WHEN corpus creation fails THEN `setup_failed`, created files are removed, and there is no `ok: true` report | Error code `setup_failed` (`Result::Err`, so no report); root gone; parent marker unchanged | `src-tauri/src/bench/mod.rs:496` — `assert_eq!(error.as_code(), "setup_failed")`; `mod.rs:497` — `assert!(!root.exists())`; `mod.rs:498` — `assert_eq!(fs::read(&marker).unwrap(), b"parent")` | ✅ PASS |

### P1: Varredura e SHA-256 determinísticos (PERF-02)

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| WHEN 64 files are read THEN the aggregate digest, counts, and `ok` match the normative vector | `contentDigest` `c067a4e6cde3d7306e1fef52a92ebee10c1e562368ad06488bb1398610067415`, `filesHashed` 64, `totalBytes` 262144, `bytesPerItem` 4096, `ok` true | `src-tauri/src/bench/hash.rs:73` — `assert_eq!(content_digest(&digests), "c067a4e6cde3d7306e1fef52a92ebee10c1e562368ad06488bb1398610067415")`; `src-tauri/src/bench/mod.rs:532` — `assert!(report.ok)`; `mod.rs:534` — `assert_eq!(report.files_hashed, 64)`; `mod.rs:535` — `assert_eq!(report.total_bytes, 262_144)`; `mod.rs:536` — `assert_eq!(report.bytes_per_item, 4096)`; `mod.rs:539` — `assert_eq!(report.content_digest, DIGEST_64)` | ✅ PASS |
| WHEN indexes 0, 1, and 63 are hashed THEN the digests are the published hex | Index 0 `d67c656e01756650d77717b0839985a056ec28ffe174601d690fc407a2ceffca`; index 1 `29fa02d3381a0b093ff2d6d783c1c78176fd1aafce643c8d205631dc85df5e95`; index 63 `b21fe85ea1fba0db5690a657db55547f0cf8cb4a9ea81ac332e67de8de84be06` | `src-tauri/src/bench/hash.rs:54` — `assert_eq!(hex_of(hash_reader(Limited::new(file_bytes(0))).unwrap()), "d67c656e…")`; `hash.rs:58` — index 1 hex; `hash.rs:62` — index 63 hex | ✅ PASS |
| WHEN the walk and the hash finish THEN both elapsed fields are integers ≥ 0, measured apart from corpus creation and from each other | Integers ≥ 0. The spec also says no test may fail for exceeding a time limit, and 0 is valid, so there is no numeric ceiling | `src-tauri/src/bench/mod.rs:325` — `assert!(json["walkElapsedMs"].as_u64().is_some())`; `mod.rs:326` — `assert!(json["hashElapsedMs"].as_u64().is_some())` | ✅ PASS |
| WHEN one regular file cannot be read THEN the others continue, that relative path is `unreadable`, `ok` is false, and `filesHashed` is the success count | `ok` false, `filesHashed` 63, one failure with `reason` `unreadable` and path `f03/2000/01/item-0000000.bin`, root removed | `src-tauri/src/bench/mod.rs:447` — `assert!(!report.ok)`; `mod.rs:448` — `assert_eq!(report.files_hashed, 63)`; `mod.rs:450` — `assert_eq!(report.failures[0].reason, "unreadable")`; `mod.rs:451` — `assert_eq!(report.failures[0].relative_path, "f03/2000/01/item-0000000.bin")`; `mod.rs:455` — `assert!(!root.exists())` | ✅ PASS |
| WHEN the same index is hashed twice in different processes THEN that file digest is equal | The published index-0 hex, and two full runs both equal the 64-file digest | `src-tauri/src/bench/hash.rs:54` — index 0 hex (stable across test processes); `src-tauri/src/bench/mod.rs:348` — `assert_eq!(first, DIGEST_64)`; `mod.rs:349` — `assert_eq!(second, DIGEST_64)` | ✅ PASS |
| WHEN a reparse-point directory inside the tree points outside THEN the walk does not count the outside files | The 64-file success contract still holds (outside bytes are not in the digest or the count) and the sentinel remains | `src-tauri/src/bench/mod.rs:389` — `assert_success(&report)`; `mod.rs:391` — `assert_eq!(fs::read(&sentinel).unwrap(), b"outside")` | ✅ PASS |

Normative byte and path rules used by those criteria are also exact: `src-tauri/src/bench/corpus.rs:100` — `assert_eq!(bytes.len(), 4096)` and `corpus.rs:102` — `assert_eq!(*byte, ((250 + offset as u32) % 251) as u8)`; `corpus.rs:108` — `assert_eq!(relative_path(0), "f03/2000/01/item-0000000.bin")` through `corpus.rs:111` — index 1200 `f03/2001/01/item-0001200.bin`. Streaming chunks of at most 65536 bytes are asserted by `src-tauri/src/bench/hash.rs:123` — `assert!(buf.len() <= 65_536)`.

### P1: Classificar HDD, SSD ou desconhecido (PERF-03)

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| WHEN `classify` receives seek penalty `true` THEN `hdd`; `false` THEN `ssd`; absent THEN `unknown` | `VolumeClass::Hdd`, `VolumeClass::Ssd`, `VolumeClass::Unknown` | `src-tauri/src/bench/volume.rs:121` — `assert_eq!(classify_seek_penalty(Some(true)), VolumeClass::Hdd)`; `volume.rs:126` — `assert_eq!(classify_seek_penalty(Some(false)), VolumeClass::Ssd)`; `volume.rs:131` — `assert_eq!(classify_seek_penalty(None), VolumeClass::Unknown)` | ✅ PASS |
| WHEN the function runs on Windows and the volume query responds THEN `volumeClass` is `classify(IncursSeekPenalty)` and `volumeRoot` is that volume’s uppercase root with a trailing backslash | Report `volumeClass` equals `classify_seek_penalty(query_seek_penalty(volumeRoot))`; `volumeRoot` equals an uppercase letter plus `:\` (for example `D:\`) | The report asserts a disjunction, not the live query result: `src-tauri/src/bench/mod.rs:358` — `assert!(matches!(report.volume_class, VolumeClass::Hdd \| VolumeClass::Ssd \| VolumeClass::Unknown))`; `mod.rs:362` — `assert!(report.volume_root.ends_with(":\\"))`; `mod.rs:428` — `assert!(matches!(value["volumeClass"].as_str(), Some("hdd" \| "ssd" \| "unknown")))`. The pure formatter is exact at `volume.rs:136` — `assert_eq!(volume_root_of(Path::new(r"d:\temp\bench")), r"D:\")`, and `volume.rs:145` pairs the query with `classify`, but neither assertion reads `BaselineReport.volume_class` | ❌ GAP |
| WHEN the volume query fails, or the system is not Windows THEN `volumeClass` is `unknown` and the measurement continues. Off Windows, `volumeRoot` is empty | Absent penalty → `unknown`. Off-Windows empty `volumeRoot` is not executed on this Windows gate | `src-tauri/src/bench/volume.rs:131` — `assert_eq!(classify_seek_penalty(None), VolumeClass::Unknown)`; `volume.rs:145` — `assert!(matches!((penalty, class), (Some(true), VolumeClass::Hdd) \| (Some(false), VolumeClass::Ssd) \| (None, VolumeClass::Unknown)))` | ✅ PASS |
| WHEN `workerCount` is read THEN it is an integer ≥ 1 and it is the process available parallelism, or 1 if that number cannot be obtained | `worker_count` equals `thread::available_parallelism()` (minimum 1), or 1 if that call fails | `src-tauri/src/bench/mod.rs:357` — `assert!(report.worker_count >= 1)` only. The official reports record `workerCount` 20 (`baselines/ssd-50000.json:6`), so the equality is a concrete value the test never checks | ❌ GAP |

### P1: Evidência em 50.000 e 150.000 (PERF-04)

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| WHEN the operator runs 50000 and 150000 on an empty directory whose class is `hdd` or `ssd` THEN each report has `ok` true, `filesHashed` equal to the requested count, `bytesPerItem` 4096, `failureCount` 0, and the root is gone | Those four fields, and the chosen root no longer exists | `baselines/ssd-50000.json:2`–`9` and `:14` — `itemCount`/`filesHashed` 50000, `bytesPerItem` 4096, `volumeClass` `ssd`, `failureCount` 0, `ok` true, `totalBytes` 204800000. `ssd-150000.json` same shape with 150000, `ssd`, `totalBytes` 614400000. `hdd-50000.json` and `hdd-150000.json` match with `volumeClass` `hdd` and `volumeRoot` `D:\`. `D:\my-photos-f03-baseline\corpus` does not exist. The 50k digests match each other and the 150k digests match each other | ✅ PASS |
| WHEN those two runs finish THEN the operator saves both JSON files as `{volumeClass}-{itemCount}.json` | Files `ssd-50000.json`, `ssd-150000.json`, `hdd-50000.json`, `hdd-150000.json` | The four files are in `.specs/features/f03-performance-baseline/baselines/` | ✅ PASS |
| WHEN one of the two media is absent THEN that medium is recorded as not measured, the present runs stay valid, and the automatic test does not fail because of the absent medium | Both `hdd` and `ssd` were present on this machine, so the “not measured” clause does not apply. The suite accepts any of the three classes | `baselines/README.md:12`–`14` records both classes as measured (`C:` ssd, `D:` hdd; `E:` answered `unknown` and was not used). `mod.rs:358` accepts `Hdd`, `Ssd`, or `Unknown` | ✅ PASS |
| WHEN CI runs the suite THEN it runs the 64-file contract and does not create the 50000 or 150000 corpus | Automated runs use 64 (or CLI count 1). Accepting count 150000 in `validate` creates no file | `src-tauri/src/bench/mod.rs:321` — `run_baseline(&root, 64)`; `corpus.rs:127` — `assert!(validate(&root, 150_000).is_ok())`; `corpus.rs:128` — `assert!(!root.exists())`. No test calls `run_baseline` with 50000 or 150000 | ✅ PASS |

**Status**: ❌ Gaps present — 19/21 acceptance criteria match the spec outcome. 0 spec-precision gaps. 2 gaps (PERF-03 report class, PERF-03 `workerCount`).

---

## Discrimination Sensor

Mutations were applied in the working tree, the covering tests were run, and the files were restored with `git checkout` before the next mutation. HEAD stayed `4f749ad`. No stash and no extra commit.

| Mutation | File:line | Description | Killed? |
| -------- | --------- | ----------- | ------- |
| 1 | `src-tauri/src/bench/corpus.rs:9` | File byte formula `% 251` changed to `% 250` | ✅ Killed. `bench::hash::tests::known_file_digests_match_the_spec` failed at `hash.rs:54`: left `08b6369fb86e53a292c96dcf9c0f4d275b871dc47869c5515d5daff41a61eab6`, right `d67c656e01756650d77717b0839985a056ec28ffe174601d690fc407a2ceffca` |
| 2 | `src-tauri/src/bench/mod.rs:78` | Report `volume_class` forced to `VolumeClass::Unknown` instead of `classify_seek_penalty(query_seek_penalty(…))` | ❌ Survived. `report_records_workers_and_the_volume_class` and `report_json_uses_camel_case_names` passed (2 passed, 0 failed) |
| 3 | `src-tauri/src/bench/mod.rs:294` | `available_workers` returned `1` instead of the process parallelism (official reports record `20`) | ❌ Survived. All 12 `bench::tests::` tests passed |

**Sensor depth**: lightweight (not a payment, auth, or persisted-data integrity path)
**Result**: 1/3 killed — FAIL ❌

---

## Interactive UAT Results (if performed)

Not performed. This feature is a local baseline command and a report. There is no user-facing UI flow.

---

## Code Quality

| Principle | Status |
| --------- | ------ |
| Minimum code | ✅ |
| Surgical changes | ✅ |
| No scope creep | ✅ |
| Matches patterns | ✅ |
| Spec-anchored outcome check (asserted values match spec) | ❌ PERF-03 report class and `workerCount` |
| Per-layer Coverage Expectation met (domain 1:1 ACs; routes happy+edge+error) | ❌ Orchestration tests do not lock those two report fields |
| Every test maps to a spec requirement — no unclaimed tests | ✅ Library tests map to PERF-01–PERF-03 and the normative vectors. `src-tauri/tests/f03_bench.rs` maps to T9 done-when (usage exit 2, `count_out_of_range` exit 1, count-1 pretty JSON) |
| Documented guidelines followed: none — strong defaults applied, bounded by the feature spec (`tasks.md` coverage matrix) | ✅ |

The hash story is non-shallow: the assertions are the spec hex strings. The volume-class story’s report check is shallow: any of the three legal names passes.

`// SPEC_DEVIATION` markers (lessons signal, not additional AC failures):

- `src-tauri/tests/f03_bench.rs:7` — `CARGO_BIN_EXE_f03-bench` keeps the hyphen.
- `src-tauri/src/bench/volume.rs:49` — `windows-sys` 0.61.2 also enables `Win32_Security` so `CreateFileW` compiles.
- `src-tauri/src/bench/mod.rs:30` — the junction is planted in `before_walk` because `validate` rejects a non-empty root.

---

## Edge Cases

- [x] Count exactly 1 or exactly 150000 is accepted, and 150001 returns `count_out_of_range` (`corpus.rs:118`–`128`).
- [x] An existing empty root is usable and is removed after cleanup (`corpus.rs:192`–`193`, `mod.rs:335`–`336`).
- [x] When every read fails, `contentDigest` is empty, `filesHashed` is 0, and `ok` is false (`mod.rs:471`–`473`).
- [x] A zero elapsed time stays valid: `as_u64()` accepts 0 and the spec forbids a millisecond ceiling (`mod.rs:325`–`326`).
- [x] Cleanup failure returns `cleanup_failed` and leaves the root (`mod.rs:514` — `assert_eq!(error.as_code(), "cleanup_failed")`; `mod.rs:515` — `assert!(root.join("f03").is_dir())`).
- [x] A later call on a leftover non-empty root returns `root_not_empty` and does not delete the marker (`mod.rs:527`–`528`). The process-kill itself is not simulated; the specified next-call outcome is.

---

## Gate Check

- **Gate command**: `$env:Path = "$env:USERPROFILE\.cargo\bin;" + $env:Path`; `cargo test --manifest-path src-tauri/Cargo.toml`. The tasks.md build gate was also run: `npm run build` and `node --experimental-strip-types --test src/health/view.test.ts`.
- **Result**: cargo 87 passed, 0 failed, 0 skipped (84 library + 3 in `tests/f03_bench.rs`). `npm run build` exited 0. The health view test passed 3, failed 0.
- **Test count before feature**: 54 (`#[test]` count at `ca6c5c3`: health 3, db 2, classify 6, matching 10, takeout 21, parse 12).
- **Test count after feature**: 87.
- **Delta**: +33 new tests. No existing test was deleted. No assertion on a pre-feature test was weakened.
- **Skipped tests**: none.
- **Failures**: none.

---

## Fix Plans (if issues found)

### Fix 1: Lock the report volume class to the live seek penalty

- **Root cause**: `report_records_workers_and_the_volume_class` and the JSON test accept `hdd`, `ssd`, or `unknown`. Forcing `volume_class` to `Unknown` while the query still returns a penalty still passes. This machine’s `C:` baseline is `ssd`, so the gap is real, not hypothetical.
- **Fix task**: In `src-tauri/src/bench/mod.rs`, assert `report.volume_class == classify_seek_penalty(query_seek_penalty(&report.volume_root))` and `report.volume_root == volume_root_of(&root)` (uppercase letter and trailing `:\`). When the query returns `Some`, the class must be `hdd` or `ssd`.
- **Verify**: Re-apply the `VolumeClass::Unknown` mutant at `mod.rs:78` and confirm that test fails, then restore the tree.
- **Done when**: That mutant is killed and the three `classify_seek_penalty` assertions still match `hdd`, `ssd`, and `unknown`.
- **Priority**: Major

### Fix 2: Lock `workerCount` to the process parallelism

- **Root cause**: The only assertion is `worker_count >= 1`. Returning `1` always left all 12 bench tests green. The spec outcome is the available parallelism of the process, or 1 only when that number cannot be obtained.
- **Fix task**: In the same report test, assert `worker_count` equals `thread::available_parallelism().map(|n| n.get() as u32).unwrap_or(1).max(1)`.
- **Verify**: Re-apply the `available_workers` → `1` mutant and confirm the test fails on a machine whose parallelism is greater than 1, then restore the tree.
- **Done when**: That mutant is killed.
- **Priority**: Major

---

## Requirement Traceability Update

`spec.md` was not edited. This verifier writes only `validation.md` and the lessons files.

| Requirement | Previous Status | New Status |
| ----------- | --------------- | ---------- |
| PERF-01 | Done | ✅ Verified |
| PERF-02 | Done | ✅ Verified |
| PERF-03 | Done | ❌ Needs Fix |
| PERF-04 | Done | ✅ Verified |

---

## Summary

**Overall**: ❌ Not Ready

**Spec-anchored check**: 19/21 ACs matched the spec outcome. 0 spec-precision gaps.
**Sensor**: 1/3 mutations killed. 2 survived.
**Gate**: 87 passed, 0 failed. Frontend build and the 3 health view tests also passed.

**What works**: Unsafe roots return the stable error codes and leave markers in place. The SHA-256 vectors for indexes 0, 1, and 63, and the 64-file `contentDigest`, match the spec hex. Cleanup, setup failure, unreadable files, and the outside junction behave as specified. The four official baseline JSON files match `ok`, `filesHashed`, `bytesPerItem`, and `failureCount`.

**Issues found**: The report can claim `unknown` no matter what `IncursSeekPenalty` returned, and it can claim a worker count of 1 no matter the process parallelism. Both faults passed the current tests.

**Next steps**: Apply Fix 1 and Fix 2, then re-verify. Maximum 3 fix→re-verify iterations.
