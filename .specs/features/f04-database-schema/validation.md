# F04 Database Schema Validation

**Date**: 2026-10-02
**Spec**: `.specs/features/f04-database-schema/spec.md`
**Diff range**: `1347065..83ede4d` (8 task commits: `dbc1ab5`, `6c128d5`, `c127afe`, `1cae356`, `c5d5d1b`, `ec876b9`, `001392b`, `83ede4d`); iteration 2 adds `83ede4d..2f7fa3e` (`0ce7af8`, `2f7fa3e`)
**Verifier**: independent sub-agent (author ≠ verifier)

**Final verdict (iteration 2)**: ✅ PASS. All 20 ACs now have evidence that targets the spec outcome. All 27 mutants were killed. See [Iteration 2](#iteration-2-re-verification). Two items stay outside the automated gate: the WAL-recovery edge case, and the manual DB-01 AC7 / DB-07 run against the real `E:`.

**Iteration 1 verdict**: ❌ FAIL. The code matches the spec and the design when read. The SQL file is byte-identical to the design DDL, and the error codes match the design table. The FAIL is about the tests: 6 of 24 behavior-level mutants survived. Four of them hit spec ACs (DB-03 AC3, DB-03 AC4, DB-04 AC1 columns, DB-04 AC9).

---

## Task Completion

| Task | Commit | Status | Notes |
| ---- | ------ | ------ | ----- |
| T1 | `dbc1ab5` | ✅ Done | 8 variants, codes and messages match the design table (`mod.rs:24`–`45`); `latest_version_of` 0 / 2 asserted |
| T2 | `6c128d5` | ✅ Done | Relative, missing volume, missing grandparent, hidden creation and visible-folder cases all asserted |
| T3 | `c127afe` | ✅ Done | Garbage and v99 files are refused and stay byte-identical, with no `-wal`/`-shm`; the three PRAGMAs are asserted |
| T4 | `1cae356` | ⚠️ Done, weak tests | Runner is correct when read. The test leaves no WAL frames before the backup and runs no multi-step failure, so mutants M13 and M23 survive |
| T5 | `c5d5d1b` | ⚠️ Done, weak tests | SQL equals the design DDL exactly. The tests check table and index names only, not columns or indexed columns (M14, M15 survive) |
| T6 | `ec876b9` | ✅ Done | JSON `{"appName":"My Photos","schemaVersion":1}` and every error message asserted |
| T7 | `001392b` | ✅ Done | `Banco: esquema v1`, `Falha: disco E: não encontrado`, invalid 0 / 1.5 asserted |
| T8 | `83ede4d` | ✅ Done | Build gate green. `open_library` appears only in its definition (`db/mod.rs:71`) and `lib.rs:17`; no test calls it |

All T1–T8 commits are present. The working-tree `tasks.md` header says "Done (T1–T8 committed; Verifier pending)". The per-task "Done when" checkboxes are still `[ ]`. That is bookkeeping only.

---

## Spec-Anchored Acceptance Criteria

### P1: Banco aberto e migrado ao iniciar (DB-01, DB-02)

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| AC1: parent exists and file absent → create, apply all, `user_version` = latest (1) | File created; `user_version` equals the last version of the list; v1 for the real list | `src-tauri/src/db/mod.rs:480` — `assert_eq!(plain_version(&path), 2)` (list `TWO`, latest 2); `mod.rs:481` — `assert!(has_table(&path, "t") && has_table(&path, "u"))`; real list: `src-tauri/src/db/schema_tests.rs:51` — `assert_eq!(latest_version(), 1)` plus `schema_tests.rs:71` (all v1 tables exist after `open`) | ✅ PASS (by composition; no test reads `PRAGMA user_version` of a DB opened with `MIGRATIONS`) |
| AC2: `.myphotos` absent, volume root exists → create it hidden, DB inside | Folder exists with `FILE_ATTRIBUTE_HIDDEN`; DB file exists | `mod.rs:342` — `assert!(folder.is_dir())`; `mod.rs:343` — `assert!(is_hidden(&folder))`; `mod.rs:344` — `assert!(path.is_file())` | ✅ PASS |
| AC3: volume root missing → `volume_missing`, nothing created anywhere | Code `volume_missing`; no folder or file | `mod.rs:321` — `assert_eq!(error.as_code(), "volume_missing")`; `mod.rs:322` — `assert!(!Path::new(&format!(r"{letter}:\")).exists())` | ✅ PASS |
| AC4: file is not SQLite → `db_not_sqlite`, bytes identical | Code `db_not_sqlite`; bytes unchanged | `mod.rs:376` — `assert_eq!(error.as_code(), "db_not_sqlite")`; `mod.rs:377` — `assert_eq!(fs::read(&path).unwrap(), bytes)`; `mod.rs:378` — `assert!(sidecars_absent(&path))` | ✅ PASS |
| AC5: `user_version` > latest → `db_newer_than_app`, bytes identical | Code `db_newer_than_app`; bytes unchanged | `mod.rs:391` — `assert_eq!(error.as_code(), "db_newer_than_app")`; `mod.rs:392` — `assert_eq!(fs::read(&path).unwrap(), bytes)`; `mod.rs:393` — `assert!(sidecars_absent(&path))` | ✅ PASS (fixture is a rollback-journal file; a WAL-mode newer file, the realistic case, is not exercised — note) |
| AC6: connection PRAGMAs | `foreign_keys` 1, `journal_mode` `wal`, `synchronous` 2 | `mod.rs:410` — `assert_eq!(pragma("foreign_keys"), "1")`; `mod.rs:411` — `assert_eq!(pragma("journal_mode"), "wal")`; `mod.rs:412` — `assert_eq!(pragma("synchronous"), "2")` | ✅ PASS (`busy_timeout = 5000` from the Assumptions table is not asserted — M12 survived; minor) |
| AC7: app start shows `Banco: esquema v1`; failure shows `Falha: ` + message (e.g. `disco E: não encontrado`) | Exact strings | Backend: `src-tauri/src/commands/health.rs:52`–`53` — `assert_eq!(serde_json::to_string(&health)…, r#"{"appName":"My Photos","schemaVersion":1}"#)`; `health.rs:59`–`62` — `VolumeMissing` → `"disco E: não encontrado"`; view: `src/health/view.test.ts:9` — `assert.equal(screen.schema, "Banco: esquema v1")`; `view.test.ts:17` — `assert.equal(screen.failure, "Falha: disco E: não encontrado")` | ✅ PASS for the components. The `lib.rs:17` → `app_health` → `App.tsx:26`–`32` wiring is declared "none / build gate" in the coverage matrix. **Manual / out of automated scope**: running the app with the real `E:` connected or disconnected (Success Criteria 1–2) was not done, per the no-`E:` rule |

### P1: Migrações seguras e idempotentes (DB-03)

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| AC1: reopen a migrated DB → no migration, no `.bak`, `sqlite_schema` identical | Schema equal; no `.bak-v*`; version unchanged | `mod.rs:491` — `assert_eq!(schema_of(&again), before)`; `mod.rs:493` — `assert_eq!(plain_version(&path), 2)`; `mod.rs:494` — `assert!(backups_in(&scratch.path).is_empty())` | ✅ PASS |
| AC2: migration fails midway → rolled back, `user_version` stays, `migration_failed` | Code `migration_failed`; version 1; v2 objects absent | `mod.rs:520` — `assert_eq!(error.as_code(), "migration_failed")`; `mod.rs:521` — `assert_eq!(plain_version(&path), 1)`; `mod.rs:522`–`523` — `has_table t` / `!has_table u` | ✅ PASS |
| AC3: existing DB at N ≥ 1 → byte-for-byte copy at `library.db.bak-vN` before the first change | `.bak-v1` holds the full committed v1 state | `mod.rs:508` — `assert_eq!(backups_in(…), vec!["library.db.bak-v1"])`; `mod.rs:509` — `assert_eq!(plain_version(&backup), 1)`; `mod.rs:510`–`511` — `has_table(&backup, "t")`, `!has_table(&backup, "u")` | ❌ GAP. The copy happens before the migration (M22 killed). But the v1 connection is closed before the copy, so the WAL is already folded into the main file. Removing the `wal_checkpoint(TRUNCATE)` (M23) still passes. The design names this exact risk ("`fs::copy` … perde páginas que estão no `-wal`"). There is also no byte comparison |
| AC4: pending N+1..M → ascending order, each in its own transaction | Ascending order; a failure in step k keeps steps < k committed | `mod.rs:545` — `assert_eq!(value, 2)` (unordered list, v2 depends on v1); `mod.rs:547` — `assert_eq!(plain_version(&path), 2)` | ❌ GAP. Order is covered (M7 killed). "Each in its own transaction" has no evidence: running all pending migrations in one transaction (M13) passes every test |

### P1: Esquema v1 completo (DB-04, DB-05, DB-06)

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| AC1: exactly the 16 tables, with the PRD 8.3 + design columns | Table set = 16 names; column sets = design DDL | `schema_tests.rs:71` — `assert_eq!(names(&connection, "table"), expected)` (16 names, `sqlite_%` excluded) | ❌ GAP for the column clause. No test checks columns. Dropping `media_items.perceptual_hash` (M14) passes. A static check shows `0001_initial.sql` is byte-identical to the design DDL block (LF-normalized `diff`), so the code is correct today. The tests just would not catch a regression |
| AC2: duplicate `(library_id, relative_path)` → UNIQUE violation, media and folders | Constraint violation (UNIQUE) | `schema_tests.rs:83` — `assert_eq!(code(error), ErrorCode::ConstraintViolation)` (media); `schema_tests.rs:88` — same (folders) | ✅ PASS (asserts the primary code, not `SQLITE_CONSTRAINT_UNIQUE`. The only constraint that can fire here is UNIQUE, so it is acceptable) |
| AC3: values outside the 4 media domains → CHECK violation | Constraint violation for `audio`, `maybe`, `guess`, `trashed` | `schema_tests.rs:107` — `assert_eq!(code(error), ErrorCode::ConstraintViolation, "{column}")` over the 4 columns; `schema_tests.rs:115`–`120` — one legal value per column accepted | ✅ PASS (by `UPDATE` rather than `INSERT`, same CHECK semantics). Note: only one legal value per domain is accepted, so dropping `'orphan_json'` from the CHECK (M24) survives. Minor |
| AC4: update `relative_path` → same `id`, favorite/notes/people/albums still linked | Same id; `is_favorite` 1; each child count 1 | `schema_tests.rs:185` — `assert_eq!(moved, id)`; `schema_tests.rs:186` — `is_favorite == 1`; `schema_tests.rs:188` — `assert_eq!(count(…), 1, "{sql}")` over 6 child tables | ✅ PASS |
| AC5: delete highest id, insert → new id greater (never reused) | `next > highest` | `schema_tests.rs:201` — `assert!(next > highest, …)` | ✅ PASS |
| AC6: delete media → 6 child tables emptied; `trash_entries.media_id` NULL, paths kept | Six counts 0; `media_id` NULL; original and trash paths unchanged | `schema_tests.rs:215` — `assert_eq!(count(…), 0, "{sql}")`; `schema_tests.rs:224` — `assert_eq!(media_id, None)`; `schema_tests.rs:225`–`226` — paths equal | ✅ PASS |
| AC7: unknown `library_id` → FK violation | Constraint violation (FK) | `schema_tests.rs:134` — `assert_eq!(code(error), ErrorCode::ConstraintViolation)` | ✅ PASS (primary code; FK is the only constraint that can fire with these values) |
| AC8: `trashed_at '2026-10-02T12:00:00Z'` → `purge_after '2026-11-01T12:00:00Z'` | Exact string | `schema_tests.rs:242` — `assert_eq!(purge_after, "2026-11-01T12:00:00Z")` | ✅ PASS |
| AC9: indexes `idx_media_taken_at`, `idx_media_hash`, `idx_media_type`, and an index on `trash_entries(purge_after)` | Those names; the trash index covers `purge_after` | `schema_tests.rs:72`–`75` — `assert_eq!(names(&connection, "index"), vec!["idx_media_hash", "idx_media_taken_at", "idx_media_type", "idx_trash_purge"])` | ❌ GAP. Only names are asserted. Moving `idx_trash_purge` to `trashed_at` (M15) passes, so the spec clause "um índice de `trash_entries(purge_after)`" has no evidence |

**Status**: ❌ Gaps present. 16/20 ACs match the spec outcome. 4 ACs have no evidence for one clause (DB-03 AC3, DB-03 AC4, DB-04 AC1 columns, DB-04 AC9 indexed column). 0 spec-precision gaps: every AC in this spec gives an exact outcome.

### DB-07 (health screen)

Covered by DB-01 AC7 above. `health.rs:73` — `assert_eq!(health_from_open(Err(error)).unwrap_err(), error.message())` for all 8 variants; `view.test.ts:27` — `assert.equal(screen.failure, "Falha: schemaVersion inválido")` for 0 and 1.5. ✅ PASS for the components; the end-to-end run is manual.

---

## Design Conformance

- **DDL**: `src-tauri/src/db/migrations/0001_initial.sql` was diffed against the ```` ```sql ```` block in `design.md` with CR removed. They are identical, with every `-- F04` comment present. `AUTOINCREMENT` is on every table that has an `id`. `ON DELETE CASCADE` is on the 6 media child tables, `media_people.person_id`, `media_albums.album_id`, `duplicate_group_members.group_id` and `indexing_errors.run_id`. `SET NULL` is on `albums.cover_media_id`, `duplicate_groups.representative_media_id` and `trash_entries.media_id`. `purge_after` is `GENERATED ALWAYS … STORED`.
- **Error table**: The 8 `DbError` variants, `as_code()` strings (`mod.rs:24`–`31`) and `message()` strings (`mod.rs:37`–`45`) match the design table exactly. `mod.rs:248`–`270` asserts all 16 strings.
- **Interfaces**: `LIBRARY_DB_PATH`, `open_library`, `open`, `open_with` (pub(crate)), `Migration`, `MIGRATIONS`, `latest_version`, `DbState`, `AppHealth { app_name, schema_version }` (camelCase), `health_from_open`, and `app_health(State<DbState>)` are all present as designed. Two small helpers were added: `latest_version_of` and `schema_version(&Connection)`.
- **Open order** (`mod.rs:87`–`103`): absolute check → volume → parent → `Connection::open` → read `user_version` → newer check → `configure` → `migrate`. This matches the flowchart. M4 confirms that writing PRAGMAs before the version check breaks byte identity.

---

## Discrimination Sensor

**Method**: Each mutation was applied to the real working-tree file. Then `cargo test --manifest-path src-tauri/Cargo.toml --lib` ran (or `node --test` for the TS mutant). The file was then restored from the `HEAD` blob (`git show HEAD:<file> > <file>`), checked with `cmp` against the blob, and its index stat refreshed. Nothing was staged or committed, and HEAD stayed `83ede4d`.

**Restore incident**: The first five mutants were restored with `git checkout --`. With `core.autocrlf=true`, that rewrote `db/mod.rs` and `0001_initial.sql` with CRLF endings, while the originals were LF. Both files were then rewritten from the LF blobs, and the harness was switched to blob restore. `git status` was clean (apart from the pre-existing `tasks.md` change and untracked files) after every mutant. Every touched file is byte-identical to `HEAD`. The final full gate was re-run on the restored tree.

| # | File:line | Mutation | Killed? |
| - | --------- | -------- | ------- |
| 1 | `0001_initial.sql:22` | `media_items.id` without `AUTOINCREMENT` | ✅ Killed. `deleted_highest_id_is_never_reused` (`schema_tests.rs:201`) |
| 2 | `0001_initial.sql:135` | `'+30 days'` → `'+29 days'` | ✅ Killed. `trash_entry_is_purged_thirty_days_after_it_was_trashed` (`schema_tests.rs:242`) |
| 3 | `mod.rs:120` | Backup copy skipped | ✅ Killed. `existing_database_is_copied_before_it_is_migrated` (the stale `.bak` is not a DB) |
| 4 | `mod.rs:96`–`100` | `configure()` moved before the version read and check | ✅ Killed. `file_that_is_not_sqlite…` (code mismatch at `mod.rs:376`) and `database_newer_than_the_app…` (bytes changed, `mod.rs:392`) |
| 5 | `0001_initial.sql:128` | `trash_entries.media_id` `ON DELETE SET NULL` → `CASCADE` | ✅ Killed. `deleting_a_media_cascades…` (trash row gone, `schema_tests.rs:223`) |
| 6 | `0001_initial.sql:118` | `user_notes.media_id` without `ON DELETE CASCADE` | ✅ Killed. `deleting_a_media_cascades…` (FK error on delete, `schema_tests.rs:212`) |
| 7 | `mod.rs:118` | Pending migrations not sorted | ✅ Killed. `pending_migrations_run_in_ascending_version_order` |
| 8 | `mod.rs:127`–`133` | SQL failure ignored until after `user_version` is set and committed, then `migration_failed` returned | ✅ Killed. `failed_migration_rolls_back…` (version 2 ≠ 1) |
| 9 | `mod.rs:175` | `foreign_keys = OFF` | ✅ Killed. `opened_connection_has_the_spec_pragmas`, `unknown_library_violates_the_foreign_key`, `deleting_a_media_cascades…` |
| 10 | `mod.rs:230` | `hide()` made a no-op | ✅ Killed. `missing_folder_is_created_hidden…` (`mod.rs:343`) |
| 11 | `health.rs:13` | JSON field `schemaVersion` renamed to `version` | ✅ Killed. `open_database_reports_my_photos_and_the_schema_version` (`health.rs:52`) |
| 12 | `mod.rs:172` | `busy_timeout` 5000 → 0 | ❌ Survived (Assumptions-table PRAGMA, not an AC; minor) |
| 13 | `mod.rs:122`–`134` | All pending migrations in one transaction | ❌ Survived. DB-03 AC4 "cada uma em sua transação" |
| 14 | `0001_initial.sql:30` | Column `media_items.perceptual_hash` removed | ❌ Survived. DB-04 AC1 columns |
| 15 | `0001_initial.sql:137` | `idx_trash_purge` on `trashed_at` instead of `purge_after` | ❌ Survived. DB-04 AC9 |
| 16 | `mod.rs:185` | `synchronous = NORMAL` | ✅ Killed. `opened_connection_has_the_spec_pragmas` (`mod.rs:412`) |
| 17 | `src/health/view.ts:35` | `schemaVersion < 1` → `< 0` | ✅ Killed. `view.test.ts` (the 0 case; 2 pass, 1 fail) |
| 18 | `mod.rs:89`–`91` | Volume-exists check removed | ✅ Killed. `missing_volume_is_rejected…` (`parent_missing` ≠ `volume_missing`) |
| 19 | `mod.rs:165` | `NotADatabase` mapped to `OpenFailed` | ✅ Killed. `file_that_is_not_sqlite…` |
| 20 | `mod.rs:97` | Newer check `>` → `>=` | ✅ Killed. 3 tests (reopen and folder tests) |
| 21 | `mod.rs:119` | Backup also for version 0 (new file) | ✅ Killed. `new_file_reaches_the_latest_version_without_a_backup`, `reopening…`, `existing_database_is_copied…` |
| 22 | `mod.rs:119`–`121` | Backup moved after the migration loop | ✅ Killed. `existing_database_is_copied…` (`.bak` at v2, `mod.rs:509`) |
| 23 | `mod.rs:141`–`143` | `PRAGMA wal_checkpoint(TRUNCATE)` before the copy removed | ❌ Survived. DB-03 AC3 (WAL pages missing from `.bak`) |
| 24 | `0001_initial.sql:48` | `'orphan_json'` removed from the `indexing_status` CHECK | ❌ Survived (legal domain value no longer accepted; minor) |

**Sensor depth**: expanded (persistence / data-integrity path). 24 manual behavior-level mutations; no mutation tool is installed.
**Result**: 18/24 killed, 6 survived. ❌ FAIL

---

## Interactive UAT Results

Not performed. The only user-facing outcome is the health line, and it needs the real `E:` (Success Criteria 1–2). The verifier is forbidden to touch `E:` or run the app. **Manual check still owed by the user**: `npm run tauri dev` with `E:` connected should show `Banco: esquema v1` and create `E:\.myphotos\library.db`, hidden. With `E:` disconnected it should show `Falha: disco E: não encontrado` and create nothing.

---

## Code Quality

| Principle | Status |
| --------- | ------ |
| Minimum code | ✅ About 150 lines of runner, no new dependency (approach A) |
| Surgical changes | ✅ Only the 8 files in the design were touched |
| No scope creep | ✅ No repository layer, no FTS, no purge logic |
| Matches patterns | ✅ `DbError` follows the `BenchError` shape; `Scratch` follows `bench::Scratch` |
| Spec-anchored outcome check (asserted values match spec) | ❌ DB-03 AC3, DB-03 AC4, DB-04 AC1 columns, DB-04 AC9 |
| Per-layer Coverage Expectation met | ❌ Schema layer "1:1 to DB-04 … ACs" misses the column and indexed-column clauses; opening layer misses the per-transaction clause |
| Every test maps to a spec requirement — no unclaimed tests | ✅ All 25 new tests map to an AC, edge case or T1 done-when (`every_error_has_the_design_code_and_message`, `latest_version_is_the_highest_listed_or_zero`) |
| Documented guidelines followed: none — strong defaults applied (`tasks.md` coverage matrix) | ✅ |

Notes:
- `as_code()` and `latest_version()` carry `#[allow(dead_code)]` because only tests use them today. This is acceptable and commented.
- The design's Risk row 6 says "um teste garante que `open` num caminho temporário não cria nada fora dele". The closest test is `relative_path_is_rejected_without_creating_anything` (`mod.rs:310` — `!Path::new("f04-relative").exists()`). No test checks that a successful open creates nothing outside its temp dir. Minor; the code has no such write path.
- No `// SPEC_DEVIATION` markers in the diff.

---

## Edge Cases

- [x] Relative path → `path_not_absolute`, nothing created (`mod.rs:309` — `assert_eq!(error.as_code(), "path_not_absolute")`; `mod.rs:310` — `assert!(!Path::new("f04-relative").exists())`).
- [x] Existing `.myphotos` without the hidden attribute is used as is (`mod.rs:354` — `assert!(!is_hidden(&folder))`; `mod.rs:355` — `assert!(path.is_file())`).
- [ ] Leftover `library.db-wal` is recovered on open. The spec says this is default SQLite behavior with no extra code. **No automated evidence**; low risk. It is also the scenario where M23 matters: recovered frames stay in the `-wal` until a checkpoint, so the backup depends on that checkpoint.
- [x] Existing `.bak-vN` is overwritten (`mod.rs:503` writes `b"stale backup"`; `mod.rs:509` — `assert_eq!(plain_version(&backup), 1)`; `mod.rs:510` — `assert!(has_table(&backup, "t"))`).
- [x] Missing grandparent → `parent_missing`, nothing created (design flowchart; `mod.rs:331`–`332`).

---

## Gate Check

- **Gate command** (Build level from `tasks.md`): `"$USERPROFILE/.cargo/bin/cargo.exe" test --manifest-path src-tauri/Cargo.toml` + `node --experimental-strip-types --test src/health/view.test.ts` + `npm run build`. Also `npx tsc --noEmit`.
- **Result**: cargo exit 0. Library: 108 passed, 0 failed, 1 ignored. Integration (`tests/f03_bench.rs`): 3 passed. TS: 3 passed, 0 failed (exit 0). `npx tsc --noEmit` exit 0. `npm run build` exit 0. Re-run on the restored tree after the sensor gave the same result.
- **Test count before feature** (`#[test]` at `1347065`): 92 (89 library including 1 ignored, plus 3 integration).
- **Test count after feature** (`83ede4d`): 112 (109 library including 1 ignored, plus 3 integration).
- **Delta**: +20 (25 added, 5 removed). The removed tests are `db::select_one_returns_1`, `db::second_call_returns_1_without_creating_a_db_file` and the three `health_from_probe` tests. Their code under test (`select_one`, `health_from_probe`) was deleted by design (T1, Risk row 6) and replaced by `health_from_open` tests. No surviving pre-feature test was weakened.
- **Skipped tests**: `takeout::tests::real_takeout_survey`, which is pre-existing (F02), `#[ignore]` and needs `TAKEOUT_ROOT`. It must not run here because of the no-`E:` rule.
- **Failures**: none.

---

## Fix Plans

### Fix 1: Exercise the WAL checkpoint before the backup (DB-03 AC3) — Major

- **Root cause**: `existing_database_is_copied_before_it_is_migrated` drops the v1 connection before migrating. Closing the last connection checkpoints the WAL, so `fs::copy` always sees a complete main file and the checkpoint in `back_up` is never needed (M23 survives).
- **Fix task**: In `src-tauri/src/db/mod.rs` tests, open v1 with `open_with(&path, ONE)`, insert a row into `t`, and keep that connection alive. Then call `open_with(&path, TWO)` and assert that `.bak-v1` opens at version 1, has table `t` **and the inserted row**, and has no `u`. Optionally snapshot the main file after a checkpoint and compare bytes with the `.bak`.
- **Verify**: Re-apply M23 (`let busy: i64 = 0;` instead of the checkpoint query) and confirm the new test fails.
- **Done when**: M23 is killed.

### Fix 2: Prove each migration commits on its own (DB-03 AC4) — Major

- **Fix task**: Open a **new** file with `BROKEN_TWO`. Assert `migration_failed`, `plain_version == 1`, `has_table(t)` and `!has_table(u)`.
- **Verify**: Re-apply M13 (one transaction for all pending migrations); the test must fail (version 0, no `t`).
- **Done when**: M13 is killed.

### Fix 3: Lock the v1 columns (DB-04 AC1) — Major

- **Fix task**: In `schema_tests.rs`, for each of the 16 tables, compare `SELECT name FROM pragma_table_info(?1) ORDER BY cid` with the column list of the design DDL.
- **Verify**: Re-apply M14 (drop `perceptual_hash`); the test must fail.
- **Done when**: M14 is killed.

### Fix 4: Lock the indexed columns (DB-04 AC9) — Major

- **Fix task**: Assert `pragma_index_info` for `idx_trash_purge` = `["purge_after"]`, `idx_media_taken_at` = `["library_id","taken_at"]`, `idx_media_hash` = `["file_hash_sha256"]` and `idx_media_type` = `["library_id","media_type"]`.
- **Verify**: Re-apply M15; the test must fail.
- **Done when**: M15 is killed.

### Fix 5: Accept every legal domain value (DB-05 AC3 / DB-04) — Minor

- **Fix task**: In `media_domains_reject_values_outside_the_spec`, loop over every legal value: `photo`, `video`; `exact`, `heuristic`, `none`; `takeout_json`, `exif`, `filesystem`; `ok`, `error`, `orphan_json`, `missing_file`.
- **Done when**: M24 is killed.

### Fix 6: Assert `busy_timeout` (Assumptions table) — Minor

- **Fix task**: In `opened_connection_has_the_spec_pragmas`, add `assert_eq!(pragma("busy_timeout"), "5000")`.
- **Done when**: M12 is killed.

---

## Requirement Traceability Update

`spec.md` was not edited. This verifier writes only `validation.md`.

| Requirement | Previous Status | New Status |
| ----------- | --------------- | ---------- |
| DB-01 | Pending | ✅ Verified (automated). AC7 end-to-end with real `E:` is a manual check |
| DB-02 | Pending | ✅ Verified |
| DB-03 | Pending | ❌ Needs Fix (AC3 WAL backup, AC4 per-migration transaction) |
| DB-04 | Pending | ❌ Needs Fix (AC1 columns, AC9 indexed column) |
| DB-05 | Pending | ✅ Verified (minor: Fix 5) |
| DB-06 | Pending | ✅ Verified |
| DB-07 | Pending | ✅ Verified (components). The on-screen run is manual |

---

## Summary

**Overall**: ❌ Not Ready (test discrimination; the implementation itself matches spec and design when read)

**Spec-anchored check**: 16/20 ACs matched the spec outcome; 4 ACs missing evidence for one clause; 0 spec-precision gaps.
**Sensor**: 18/24 mutations killed, 6 survived (4 tied to ACs, 2 minor).
**Gate**: Rust 108 passed + 1 ignored (library) and 3 passed (integration). TS 3 passed. `tsc` and `npm run build` exit 0.

**What works**: Path validation, the hidden `.myphotos` folder, refusing foreign and newer files byte-for-byte, the PRAGMAs, idempotent reopen, rollback of a failed step, backup ordering and overwrite, ascending order, the full v1 DDL, UNIQUE/CHECK/FK, stable ids, cascades with SET NULL on the trash, the 30-day `purge_after`, and the health JSON and screen strings.

**Issues found**: The tests cannot detect a missing WAL checkpoint before the backup, a single transaction across migrations, a dropped column, or a mis-targeted trash index.

**Next steps**: Apply Fixes 1–4 (Fixes 5–6 optional), re-verify (at most 3 fix→re-verify rounds), then do the manual `E:` run for DB-01 AC7.

---

## Iteration 2 (re-verification)

**Date**: 2026-10-02 · **Range**: `83ede4d..2f7fa3e`. The two commits are test-only: `0ce7af8` touches `src-tauri/src/db/mod.rs` tests and `2f7fa3e` touches `src-tauri/src/db/schema_tests.rs`. No production code or SQL changed (`git diff 83ede4d..2f7fa3e --stat`: 2 files, +129/−0, all inside test modules).

### Gate

- `cargo test --manifest-path src-tauri/Cargo.toml`: exit 0. Library: 113 passed, 0 failed, 1 ignored (`real_takeout_survey`, as before). Integration: 3 passed.
- `node --experimental-strip-types --test src/health/view.test.ts`: 3 passed, 0 failed.
- `npx tsc --noEmit`: exit 0. `npm run build`: exit 0.
- Test count: 112 → 117 (+5 tests, plus 1 new assertion in an existing test). Nothing was removed or weakened. The gate was re-run on the restored tree after the sensor, with the same result.

### Re-checked ACs

| AC | Spec-defined outcome | New `file:line` + assertion | Targets the outcome? | Result |
| -- | -------------------- | --------------------------- | -------------------- | ------ |
| DB-03 AC3 | `.bak-vN` is a full copy of the committed vN state, taken before the first change | `src-tauri/src/db/mod.rs:523` — `assert!(fs::metadata(&wal).unwrap().len() > 0, "row must sit in the WAL")` (precondition: a committed row exists only in the `-wal` while the writer stays open); `mod.rs:533` — `assert_eq!(value, "in-wal")` read from `library.db.bak-v1`. Earlier evidence still holds: `mod.rs:508`–`511` (name, version 1, `t` present, `u` absent) | Yes. It asserts the committed row is in the backup. The precondition makes the test fail loudly instead of passing vacuously if SQLite ever checkpoints first | ✅ PASS |
| DB-03 AC4 | Ascending order, each migration in its own transaction: a failure in v2 leaves v1 committed | `mod.rs:542` — `assert_eq!(error.as_code(), "migration_failed")`; `mod.rs:543` — `assert_eq!(plain_version(&path), 1)`; `mod.rs:544` — `assert!(has_table(&path, "t"))`; `mod.rs:545` — `assert!(!has_table(&path, "u"))` (new file, `BROKEN_TWO`). Order (unchanged test, line moved): `mod.rs:579` — `assert_eq!(value, 2)` | Yes. Version 1 and v1's table survive a v2 failure on a fresh file, which only per-step commits allow | ✅ PASS |
| DB-04 AC1 | The 16 tables, with the PRD 8.3 + design columns | `src-tauri/src/db/schema_tests.rs:135` — `assert_eq!(pragma_names(&connection, "table_xinfo", table), *columns, "{table}")` over `COLUMNS` (`schema_tests.rs:90`), which holds all 16 tables with exact, ordered column lists. `table_xinfo` includes the generated `purge_after`. Table set still at `schema_tests.rs:71` | Yes. Exact ordered equality. I compared `COLUMNS` with the design DDL: same 16 tables and same columns in order. The SQL is unchanged and still byte-identical to the design block | ✅ PASS |
| DB-04 AC9 | The three PRD indexes, plus an index on `trash_entries(purge_after)` | `schema_tests.rs:148` — `assert_eq!(pragma_names(&connection, "index_info", index), columns, "{index}")` for `idx_media_taken_at` = `[library_id, taken_at]`, `idx_media_hash` = `[file_hash_sha256]`, `idx_media_type` = `[library_id, media_type]`, `idx_trash_purge` = `[purge_after]`. Names: `schema_tests.rs:72`–`75` | Yes. Exact column lists | ✅ PASS |
| DB-05 AC3 | Values outside the 4 domains fail CHECK, and the legal domains are exactly the spec lists | Reject side unchanged: `schema_tests.rs:202` — `assert_eq!(code(error), ErrorCode::ConstraintViolation, "{column}")`. Accept side: `schema_tests.rs:168` — `.unwrap_or_else(\|error\| panic!("{column} = {value}: {error}"))` for all 12 legal values (`photo`, `video`; `exact`, `heuristic`, `none`; `takeout_json`, `exif`, `filesystem`; `ok`, `error`, `orphan_json`, `missing_file`) | Yes. Together the two tests pin the domain exactly | ✅ PASS |
| Assumption: `busy_timeout = 5000` | 5000 ms | `mod.rs:413` — `assert_eq!(pragma("busy_timeout"), "5000")` | Yes | ✅ PASS |

The other 15 ACs are unchanged from iteration 1 and still pass. **Spec-anchored total: 20/20 ACs matched; 0 spec-precision gaps.**

### Sensor (iteration 2)

Same method as iteration 1. Each file was restored from the `HEAD` blob and checked with `cmp`; I did not use `git checkout --`. After the run, `db/mod.rs`, `db/schema_tests.rs` and `0001_initial.sql` are byte-identical to `HEAD`, and nothing is staged.

| # | File:line | Mutation | Killed? |
| - | --------- | -------- | ------- |
| 12 | `mod.rs:172` | `busy_timeout` 5000 → 0 | ✅ Killed. `opened_connection_has_the_spec_pragmas` (`mod.rs:413`) |
| 13 | `mod.rs:122`–`134` | All pending migrations in one transaction | ✅ Killed. `each_migration_commits_on_its_own` (version 0 ≠ 1) |
| 14 | `0001_initial.sql:30` | `media_items.perceptual_hash` removed | ✅ Killed. `every_table_has_the_design_columns_in_order` (`schema_tests.rs:135`) |
| 15 | `0001_initial.sql:137` | `idx_trash_purge` on `trashed_at` | ✅ Killed. `every_index_covers_the_design_columns` (`schema_tests.rs:148`) |
| 23 | `mod.rs:141`–`143` | `wal_checkpoint(TRUNCATE)` removed before the copy | ✅ Killed. `backup_holds_rows_that_were_still_in_the_wal` (the `.bak` lacks `t`/the row; panic at the query) |
| 24 | `0001_initial.sql:48` | `'orphan_json'` removed from the `indexing_status` CHECK | ✅ Killed. `every_legal_media_domain_value_is_accepted` (`schema_tests.rs:168`) |
| 25 (new) | `mod.rs:141`–`147` | `fs::copy` moved **before** the checkpoint (checkpoint still runs, too late) | ✅ Killed. `backup_holds_rows_that_were_still_in_the_wal` |
| 26 (new) | `0001_initial.sql:131`–`132` | `json_original_relative_path` and `json_trash_relative_path` swapped (same names, wrong order) | ✅ Killed. `every_table_has_the_design_columns_in_order` |
| 27 (new) | `0001_initial.sql:56` | `idx_media_type` on `(media_type)` instead of `(library_id, media_type)` | ✅ Killed. `every_index_covers_the_design_columns` |

**Result**: 9/9 killed in this round. Across both iterations: 27 mutants, 27 killed (the 18 killed in iteration 1 cover code that iteration 2 did not change). ✅ PASS

### Still outside the automated gate (unchanged)

- **WAL-recovery edge case** ("existe `library.db-wal` de uma sessão interrompida → o SQLite recupera"): there is still no automated evidence. The spec says this is default SQLite behavior with no extra code, so the risk is low. The checkpoint-before-backup path that depends on it is now covered (M23, M25).
- **DB-01 AC7 / DB-07 end-to-end and Success Criteria 1–2**: these are manual. With `E:` connected, `npm run tauri dev` should show `Banco: esquema v1` and create a hidden `E:\.myphotos\library.db`. With `E:` disconnected, it should show `Falha: disco E: não encontrado` and create nothing. The verifier did not run them (no-`E:` rule). The components are covered by `health.rs:52`–`62` and `view.test.ts:9`, `:17`.

### Requirement Traceability (final)

| Requirement | Iteration 1 | Iteration 2 |
| ----------- | ----------- | ----------- |
| DB-01 | ✅ Verified (AC7 manual) | ✅ Verified (AC7 end-to-end manual) |
| DB-02 | ✅ Verified | ✅ Verified |
| DB-03 | ❌ Needs Fix | ✅ Verified |
| DB-04 | ❌ Needs Fix | ✅ Verified |
| DB-05 | ✅ Verified (minor) | ✅ Verified |
| DB-06 | ✅ Verified | ✅ Verified |
| DB-07 | ✅ Verified (components) | ✅ Verified (on-screen run manual) |

### Summary (final)

**Overall**: ✅ Ready, pending only the manual `E:` run.
**Spec-anchored check**: 20/20 ACs matched the spec outcome; 0 spec-precision gaps.
**Sensor**: iteration 2, 9/9 killed; cumulative 27/27.
**Gate**: Rust 113 passed + 1 ignored (library) and 3 passed (integration); TS 3 passed; `tsc` and `npm run build` exit 0.
**Fix→re-verify iterations used**: 2 of 3.

---

## Lessons (to record)

The lessons below come from iteration 1. They stay valid as candidates: the fixes confirm each of them. Iteration 2 found no new signal.

`scripts/lessons.py` could not run (Python is not installed). Record these with `lessons.py add` when possible. Style follows `.specs/LESSONS.md`.

- **surviving_mutant** · scope `db` · evidence `mutant drop wal_checkpoint src-tauri/src/db/mod.rs:141 (DB-03 AC3)` — A backup test that closes the source connection first cannot detect a missing WAL checkpoint; keep committed frames in the `-wal` when the copy is taken.
- **surviving_mutant** · scope `db` · evidence `mutant single transaction src-tauri/src/db/mod.rs:122 (DB-03 AC4)` — Per-step transactions are only proven by a failure after an earlier pending step has succeeded; a single-pending-step failure test cannot tell one transaction from many.
- **ac_gap** · scope `db` · evidence `DB-04 AC1 src-tauri/src/db/schema_tests.rs:71 (mutant M14)` — When an AC names columns, assert `pragma_table_info` per table, not just the table names.
- **ac_gap** · scope `db` · evidence `DB-04 AC9 src-tauri/src/db/schema_tests.rs:72 (mutant M15)` — When an AC names the column an index covers, assert `pragma_index_info`, not just the index name.
- **surviving_mutant** · scope `db` · evidence `mutant drop 'orphan_json' src-tauri/src/db/migrations/0001_initial.sql:48 (DB-05 AC3)` — CHECK-domain tests should accept every legal value, not one sample, or a narrowed domain goes unnoticed.
- **process** · scope `verifier` · evidence `core.autocrlf=true restore of src-tauri/src/db/mod.rs` — On this repo, restoring a mutated file with `git checkout --` rewrites LF files as CRLF; restore from the blob (`git show HEAD:<file> > <file>`) and compare with `cmp`.
