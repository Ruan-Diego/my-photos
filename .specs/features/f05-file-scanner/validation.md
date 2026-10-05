# F05 File Scanner Validation

**Date**: 2026-10-05
**Spec**: `.specs/features/f05-file-scanner/spec.md`
**Design**: `.specs/features/f05-file-scanner/design.md`
**Diff range**: `5b520e2..0338c74` (one commit, `0338c74`: spec, design, tasks, `scan/` module, `f05-scan` binary, four visibility changes)
**Verifier**: independent sub-agent (author ≠ verifier)
**Environment**: Linux cloud container, no `E:`. `cargo test --lib scan::` → 34 passed. Windows-only tests did not run.

**Verdict**: ❌ FAIL

The code does what the spec and design say when you read it. I found no bug that breaks an AC on NTFS. The FAIL is about the tests:

- 6 mutants that change spec-visible behaviour survive: M07, M08, M13/M14, M19, M31 and M36.
- The hidden+system folder clause of SCAN-02 AC6 has no test anywhere in the repo, not even on Windows CI.

One robustness finding outside the spec (R1, case-colliding names) is also recorded.

---

## Task Completion

| Task | Commit | Status | Notes |
| ---- | ------ | ------ | ----- |
| T1 ISO-8601 | `0338c74` | ✅ Done | `time.rs:38-65`: epoch, leap day, ms truncation, end of year, pre-1970 → `None` |
| T2 Walk + `FileSystem` | `0338c74` | ⚠️ Done, one gap | Skips `.myphotos` (root only, any case), `Skip` entries, non-media. Errors `unreadable_directory` / `invalid_name`. Root-vanish detected. **But** nothing tests that `RealFs` maps a hidden+system folder to `Skip` (see AC gap G1). `takeout::classify` / `is_system_dir` now `pub(crate)` |
| T3 Errors, root validation, library/run rows | `0338c74` | ⚠️ Done, weak test | Validation happens before any write. `full` → `incremental` is asserted. Library `name = 'E:'` is not asserted (M36) |
| T4 Reconcile | `0338c74` | ⚠️ Done, weak tests | Table rules are implemented (`scan/mod.rs:243-420`). Untested: size-only change (M07), hash NULL on modify (M08), folder case rename (M19), protected-prefix boundary (M31) |
| T5 Hash phase + summary | `0338c74` | ⚠️ Done, weak test | Path order, error rows, root-vanish abort and counters are pinned. The 100-file batch commit is not pinned (M13, M14) |
| T6 `f05-scan` | `0338c74` | ✅ Builds | `cargo build --bin f05-scan` OK. On Linux it prints `path_not_absolute`, exit 1 (expected: `E:\` is not absolute here). The real `E:` run is manual (SCAN-07) |

`tasks.md` still shows every task `[ ]` and status "In progress". That is bookkeeping only.

---

## Spec-Anchored Acceptance Criteria

All test lines are in `src-tauri/src/scan/tests.rs` unless another file is named.

### P1: First scan (SCAN-01, SCAN-02, SCAN-03)

| Criterion | Spec outcome | `file:line` + assertion | Result |
| --------- | ------------ | ----------------------- | ------ |
| AC1 relative / missing / file root | Codes `root_not_absolute`, `root_missing`, `root_not_directory`; no row in any table | `:264` `assert_eq!(error.as_code(), code)` over the 3 cases; `:274` `assert_eq!(library.count(table), 0)` for `libraries, indexing_runs, indexing_errors, folders, media_items` | ✅ PASS |
| AC2 first run → library row + `full`/`completed`/`finished_at` | `root_path` = root; run `full`, `completed`, `finished_at` set | `:293` `assert_eq!(stored_root, root_path)`; `:298-300` run type `full`, status `completed`, `finished == Some(T1)`; `:305` second run `incremental` | ✅ PASS. Note: the spec default `name = 'E:'` is never asserted (M36 survives) |
| AC3 one row per physical folder, incl. empty and `-1-001`/`-1-002`, no root | Exact folder set | `:333-343` exact `assert_eq!` on the sorted folder list (2 Takeout parts, `Vazia`, no root row) | ✅ PASS for the AC. A folder renamed only in case keeps its old case in the DB without any test noticing (M19). That goes against "caminho … como no disco" |
| AC4 media row facts + SHA-256 | path, filename, lowercase ext, type, size, ISO ms `Z` mtime, 64-hex SHA, `ok` | `:358-364` filename/ext/type/size/`HELLO_SHA256`/`ok`; `:371` mtime = `iso_utc(metadata.modified)`; `:373-374` length + `Z`; `:404-417` all 18 extensions, upper-case on disk → lower-case ext + type | ✅ PASS |
| AC5 non-media → no row, no error | 0 media rows, 0 errors | `:429-430` `count("media_items") == 0`, `count("indexing_errors") == 0` (json, zip, html, no extension) | ✅ PASS |
| AC6 `.myphotos` (any case), links/junctions, hidden+system → nothing inside | No folder/media rows | `.myphotos`: `:440-447` (`.MYPHOTOS` skipped, `sub/.myphotos` kept); symlinks: `:460-461` (Unix only); junction: `walk.rs:327` (`#[cfg(windows)]`, **not run here**) | ❌ GAP G1. **Hidden+system has no test.** `walk.rs:406` only checks that a scripted `EntryKind::Skip` is ignored. Nothing tests `RealFs::list` → `is_system_dir` → `Skip` (`walk.rs:62-66`). Design §Risks says the rule is "already tested in F02 (Windows)". A repo-wide grep finds no such test: `is_system_dir` is used only at `takeout/mod.rs:245` and is never asserted |
| AC7 library untouched | Same bytes, size, mtime; nothing created or deleted outside `.myphotos` | `:522` and `:526` `assert_eq!(snapshot(&root), before)`. The snapshot holds path, is_dir, len, mtime and bytes for every entry. The DB is inside `root/.myphotos` (production layout). Covers both a first and an incremental scan | ✅ PASS |

### P1: Incremental scans (SCAN-04)

| Criterion | Spec outcome | `file:line` + assertion | Result |
| --------- | ------------ | ----------------------- | ------ |
| AC1 no change → nothing opened, no row changes, `incremental` | 0 opens; identical rows | `:543` `probe.opened.is_empty()`; `:544` `dump() == before` (every media + folder column); `:545`, `:555` `incremental` | ✅ PASS |
| AC2 new media → new row with hash; old ids stay | | `:567` only `b.jpg` opened; `:568` old id equal; `:571` hash; `:573` `(new, unchanged) == (1, 1)` | ✅ PASS |
| AC3 content **or size** or mtime change → same id, new size/mtime/hash/`updated_at`; only that file read | | `:595` opened = `["date.jpg","size.jpg"]`; `:598-608` same id, new size, hash, `updated_at`, mtime differs | ❌ GAP G2. `size.jpg` is rewritten, so its mtime changes too. No case has a size change with the same mtime: comparing mtime only (M07) passes. Also, no test proves the hash is cleared on modify (M08). See the mutant log for why that matters to SCAN-06 |
| AC4 missing → `missing_file` same id; back → `ok` same id | | `:635-640` id, `missing_file`, hash kept, `missing == 2`; `:643` not counted twice; `:657-665` back: only the re-dated file read, ids equal, `ok`, `(restored, modified, unchanged) == (2,1,0)` | ✅ PASS (sibling-prefix protection is weak, see M31) |
| AC5 folder gone → row deleted; new folder → row | | `:686-689` exact set `["fica","nova"]` (`velha`, `velha/sub` gone); `:698` surviving id unchanged | ✅ PASS |
| AC6 NULL hash → read even if size/mtime equal | | `:712` `opened == ["a.jpg"]`; `:713` hash written | ✅ PASS |
| AC7 run counters + summary | `files_scanned` = media on disk, `files_indexed` = hashed, `files_errored` = error rows; summary counts | `:737-753` full `ScanSummary` equality (`new 1, modified 2, unchanged 1, missing 1, restored 0, hashed 2, errors 1, bytes 7`); `:759` `(4, 2, 1)` | ✅ PASS |

### P1: Failures (SCAN-05, SCAN-06)

| Criterion | Spec outcome | `file:line` + assertion | Result |
| --------- | ------------ | ----------------------- | ------ |
| SCAN-05 AC1 unreadable file → scan continues, `error`/`unreadable`/hash NULL, `indexing_errors` row | | `:794` both opened (continues); `:796-798` status, error, hash NULL; `:800` `errors == ["unreadable bad.jpg"]`; `:801` run `completed` | ✅ PASS |
| SCAN-05 AC2 readable again → hash, `ok`, error NULL | | `:805-811` only `bad.jpg` reopened, same id, `ok`, `None`, hash, no errors | ✅ PASS |
| SCAN-05 AC3 unlistable subfolder → `unreadable_directory`, known contents unchanged, rest scanned | | `:829-831` error row; `:832` nested media still `ok`; `:833` `missing == 0`; `:835-837` folder rows kept; `:838` new sibling hashed; `:841` every prior row identical | ✅ PASS (sibling with the same prefix not covered, M31) |
| SCAN-05 AC4 invalid Unicode name → skipped, `invalid_name` | | `:858-865` (Linux-only, `\xff`); `walk.rs:414-420` | ✅ PASS |
| SCAN-06 AC5 root vanishes → `root_missing`, run `failed` + `finished_at`, no `missing_file` from the disconnect, saved hashes kept, unhashed read next time | | Hash phase: `:890` `RootMissing`; `:896` `("failed", Some(T1))`; `:897` first hash kept; `:900` others `ok` + NULL; `:908` next run opens exactly `b.jpg, c.jpg`; `:910` `missing == 0`. Walk phase: `:934` `root_missing`; `:935` `dump() == before` (no `missing_file`); `:937` `["completed","failed"]` | ✅ PASS for the cases tested. Only new files are tested. A *modified* file interrupted before hashing is not (M08) |
| SCAN-07 `f05-scan` | Scans `E:`, JSON summary; second run `unchanged` = total; `volume_missing` without `E:` | Build gate only | ⏸ Not run (no `E:`). Manual on the real drive. Code path read: `bin/f05-scan.rs` → `scan_default_library` (`mod.rs:92-97`) → `db::open_library` returns `volume_missing` before any scan |

---

## Edge Cases

| Edge case | Evidence | Result |
| --------- | -------- | ------ |
| Empty root → `completed`, 0 folders, 0 media | `:312-315` | ✅ PASS |
| 0-byte media → `size_bytes = 0`, empty SHA | `:379-380` `Some(0)`, `EMPTY_SHA256` | ✅ PASS |
| `.myphotos` in a subfolder is scanned | `:440-447`, `walk.rs:302-304` | ✅ PASS |
| Case-only difference updates the existing row | `:771-779` no open, 1 row, same id, path + filename in disk case | ✅ PASS for media. Folders: no test (M19) |

---

## Implementation Review

What I confirmed by reading the code:

- **Root validation before any write.** `mod.rs:105-112` returns before `start_run`.
- **Reconciliation.** It follows the design table (`mod.rs:378-420`): unchanged / restored / modified (+restored) / NULL-hash requeue / case rename. `missing_file` is set only when the row is not already missing and not protected (`mod.rs:363-371`). The `is_protected` slice at `mod.rs:239` cannot panic, because the preceding byte check guarantees an ASCII `/` boundary.
- **Root vanish.** In the walk, `walk.rs:134-137` checks `root.is_dir()` and aborts before reconcile, so nothing is marked missing. In the hash phase, `mod.rs:461-479` commits what was hashed in the current batch, then returns `RootMissing`. The run is marked `failed` with `finished_at` (`mod.rs:116-121`).
- **Read-only guarantee.** The only file access is `File::open` (read-only, `walk.rs:83-85`) and `fs::read_dir` / `DirEntry::metadata` (no link follow). No writes, renames or deletes outside SQLite.
- **Counters.** They match the design (`mod.rs:195-206`).

Observations. None of these breaks an AC on NTFS:

| # | Severity | Where | Finding |
| - | -------- | ----- | ------- |
| R1 | Medium (robustness, outside spec) | `mod.rs:229-231`, `mod.rs:333-353`, schema `UNIQUE(library_id, relative_path COLLATE NOCASE)` | Two names that differ only in case in one folder (`A.jpg` + `a.jpg`) can exist on Linux, on NTFS dirs with per-directory case sensitivity, or with files written by ntfs-3g. The second one goes to the `INSERT` branch and hits the UNIQUE constraint, so `?` turns it into `DbWriteFailed` and the **whole scan fails on every run**. The same applies to folders. RF-014 ("falha num item não para a varredura") suggests skipping with a per-item error instead. The spec assumes NTFS is case-insensitive, so this is not counted as an AC failure |
| R2 | Low | `walk.rs:157-160` | An entry whose metadata cannot be read becomes an `unreadable` error and is protected, with no `media_items` row. This also happens for non-media files (e.g. a `.json`), which is in tension with SCAN-02 AC5 ("não-mídia … nem em `indexing_errors`"). It is rare in practice |
| R3 | Low | `mod.rs:445-480`, `mod.rs:116-121` | On a `DbWriteFailed` inside a batch or in reconcile, the transaction rolls back but the `summary` counters (`hashed`, `errors`, `new`, …) were already incremented. `finish_run` then writes overstated `files_indexed` / `files_errored` into the `failed` run. `bytes_hashed` also includes partial reads of files that failed |
| R4 | Info | `mod.rs:476-479` | If the DB lives on the vanished disk, the batch commit and `finish_run` both fail. The run row stays `running`, as the design accepts ("a F10 trata execuções órfãs") |

---

## Discrimination Sensor (mutation log)

Each mutant was applied alone to `src-tauri/src/scan/mod.rs` or `walk.rs`. Then `cargo test --lib scan::` ran, and the file was restored with `git checkout -- <file>`. Afterwards, `git status --short` showed no source changes and the baseline was 34/34 green again.

| ID | File | Mutation | Result | Killed by |
| -- | ---- | -------- | ------ | --------- |
| M01 | mod.rs:364 | drop `!is_protected` on media missing | ✅ Killed | `unreadable_folder_is_reported_and_its_known_contents_stay` |
| M02 | mod.rs:296 | drop `!is_protected` on folder delete | ✅ Killed | same |
| M03 | walk.rs:162 | never skip `.myphotos` | ✅ Killed | `app_folder_at_the_root_is_skipped_in_any_case`, walk test |
| M04 | walk.rs:162 | skip `.myphotos` at any depth | ✅ Killed | same |
| M05 | walk.rs:162 | `.myphotos` match case-sensitive | ✅ Killed | same |
| M06 | mod.rs:392 | compare size only | ✅ Killed | `modified_content_size_or_date…`, `removed_media…` |
| **M07** | mod.rs:392 | compare mtime only | ❌ **Survived** | — SCAN-04 AC3 "ou o tamanho" |
| **M08** | mod.rs:396 | modify does not set `file_hash_sha256 = NULL` | ❌ **Survived** | — If the hash phase is interrupted (SCAN-06) after a file was modified, the row keeps the **old hash with the new size/mtime**. The next scan sees it as unchanged and never rereads it: a permanently wrong SHA-256 (violates SCAN-04 AC3 / SCAN-06 "ficaram sem hash SHALL ser lidas") |
| M09 | mod.rs:411 | restored not counted (unchanged branch) | ✅ Killed | `removed_media…` |
| M10 | mod.rs:401 | restored not counted (modified branch) | ✅ Killed | `removed_media…` |
| M11 | walk.rs:135 | no `root.is_dir()` on list failure | ✅ Killed | `root_vanishing_while_listing…`, walk test |
| M12 | mod.rs:461 | no `root.is_dir()` on open failure | ✅ Killed | `root_vanishing_while_hashing…` |
| **M13** | mod.rs:27 | `HASH_BATCH = 1` | ❌ **Survived** | — |
| **M14** | mod.rs:27 | `HASH_BATCH = usize::MAX` (one transaction) | ❌ **Survived** | — Spec "Gravação" ("lotes de até 100 … uma queda perde no máximo um lote") and design "Commit a cada 100". The vanish test cannot tell the difference, because the code commits on vanish anyway. A crash or a concurrent WAL reader would see it |
| M15 | mod.rs:364 | invert `missing_file` guard | ✅ Killed | `removed_media…`, `run_counters…` |
| M16 | mod.rs:476 | on vanish, return before commit | ✅ Killed | `root_vanishing_while_hashing…` |
| M17 | mod.rs:230 | case-sensitive key | ✅ Killed | `case_only_rename…` |
| M18 | mod.rs:385 | skip media case-rename UPDATE | ✅ Killed | `case_only_rename…` |
| **M19** | mod.rs:280 | skip folder case-rename UPDATE | ❌ **Survived** | — SCAN-02 AC3 "como no disco" |
| M20 | mod.rs:413 | NULL-hash rows not requeued | ✅ Killed | `missing_hash…`, `root_vanishing_while_hashing…`, `unreadable_file…` |
| M21 | mod.rs:455 | hash success leaves `indexing_error` | ✅ Killed | `unreadable_file…` |
| M22 | mod.rs:471 | no `indexing_errors` row for unreadable file | ✅ Killed | `unreadable_file…`, `run_counters…` |
| M23 | mod.rs:171 | `run_type` always `full` | ✅ Killed | 3 tests |
| M24 | mod.rs:116 | failed run recorded `completed` | ✅ Killed | both vanish tests |
| M25 | mod.rs:203 | `files_indexed` from `errors` | ✅ Killed | `run_counters…` |
| M26 | walk.rs:158 | unreadable entry not protected | ✅ Killed | walk test |
| M27 | walk.rs:138 | unreadable dir not protected | ✅ Killed | walk + scan tests |
| M28 | walk.rs:150 | `invalid_name` not reported | ✅ Killed | walk + scan tests |
| M29 | walk.rs:61 | drop symlink guard | ⚪ Survived, **equivalent** | `DirEntry::file_type` of a link is neither `is_dir` nor `is_file`, so it still falls to `Skip` |
| M30 | mod.rs:396 | modify does not reset status to `ok` | ⚪ Survived, near-equivalent | A successful hash sets `ok` anyway. Visible only if a restored-and-modified file's hash is interrupted (it stays `missing_file`). Minor |
| **M31** | mod.rs:238 | `is_protected` without `/` boundary | ❌ **Survived** | — With `trancada` unreadable, a vanished file in sibling `trancada2/` would wrongly stay `ok` instead of `missing_file` (SCAN-04 AC4). No test has a prefix sibling |
| M32 | mod.rs:500 | `bytes_hashed` not counted | ✅ Killed | `media_rows…`, `run_counters…` |
| M33 | mod.rs:372 | queue not sorted | ⚪ Survived, **equivalent** | The walk already sorts files, and the queue is built in walk order |
| M34 | walk.rs:9 | drop `mov` from videos | ✅ Killed | `every_media_extension…`, `media_rows…` |
| M35 | mod.rs:209 | `last_indexed_at` not set | ✅ Killed | `first_run…` |
| **M36** | mod.rs:158 | library `name` not trimmed (`E:\` instead of `E:`) | ❌ **Survived** | — Spec default `name = 'E:'` (minor) |

**Totals**: 36 mutants. 26 killed, 3 equivalent or near-equivalent (M29, M30, M33), **7 spec-visible survivors (M07, M08, M13, M14, M19, M31, M36)**.

---

## Not Run

| Item | Reason |
| ---- | ------ |
| `walk::tests::junctions_are_not_followed` (`walk.rs:325`, `#[cfg(windows)]`) | Not run (Windows CI) |
| Hidden+system skip via `is_system_dir` | Not runnable on Linux (`is_system_dir` returns `false` here), and **no test exists at all** (G1) |
| `f05-scan` on the real `E:` (SCAN-07, Success Criteria 1–2) | No `E:` drive; manual by the stakeholder |
| Full `cargo test` | 42 pre-existing Windows-only failures in `bench::`, `db::`, `takeout::`; not F05's |

---

## Required to reach PASS

1. **G1 / SCAN-02 AC6.** Add a `#[cfg(windows)]` test that sets `FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM` on a folder holding a `.jpg` (`windows_sys` is already a dependency, `db/mod.rs:233`). Assert that `scan_with` / `RealFs` records neither the folder nor the media. Also fix the design §Risks claim that F02 already tests this.
2. **M07.** Add a case that changes the size while restoring the original mtime with `set_modified`. Assert that the file is reread and the row updated.
3. **M08.** Modify a known file, make the root vanish before its hash is written, reconnect, and assert that the file is reread and the hash equals the new content.
4. **M13/M14.** Hash more than 100 files. In an `on_open` hook at open #101, read from a second connection (WAL) and assert that exactly 100 hashes are committed. A second check at open #1 would kill `HASH_BATCH = 1`.
5. **M19.** Rename a folder in case only, and assert that `folders.relative_path` takes the disk case with the same id.
6. **M31.** In the unreadable-folder test, add a sibling `trancada2/x.jpg`, delete it, and assert that it becomes `missing_file`.
7. **M36.** Assert `libraries.name` against the trimmed root. A unit test of the name derivation for `E:\` → `E:` would do.
8. Optional: decide R1 (case-colliding names abort every scan). Skipping the second name with an error would match RF-014.

## Requirement Status

| Requirement | Status |
| ----------- | ------ |
| SCAN-01 | ⚠️ Verified (library `name` default not asserted, M36) |
| SCAN-02 | ❌ Needs fix: hidden+system untested (G1); folder case (M19) |
| SCAN-03 | ✅ Verified |
| SCAN-04 | ❌ Needs fix: M07, M08, M31 |
| SCAN-05 | ✅ Verified |
| SCAN-06 | ❌ Needs fix: modified-then-interrupted path (M08); batch commit (M13/M14) |
| SCAN-07 | ⏸ Builds; manual run on `E:` pending |

**Overall**: ❌ FAIL. The code matches the spec when read. Tests are missing for SCAN-02 AC6 (hidden+system) and 7 spec-visible mutants survive.
