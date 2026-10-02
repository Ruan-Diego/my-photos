# F04 Database Schema Tasks

## Execution Protocol (MANDATORY -- do not skip)

Implement these tasks with the `tlc-spec-driven` skill: **activate it by name and follow its Execute flow and Critical Rules.** Do not search for skill files by filesystem path. The skill is the source of truth for the full flow (per-task cycle, sub-agent delegation, adequacy review, Verifier, discrimination sensor).

**If the skill cannot be activated, STOP and tell the user — do not proceed without it.**

---

**Design**: `.specs/features/f04-database-schema/design.md`
**Status**: Approved

---

## Test Coverage Matrix

> Generated from codebase, project guidelines, and spec. Guidelines found: none — strong defaults applied, bounded by `spec.md`. No `AGENTS.md`, `CONTRIBUTING.md` or coverage config. Floor: `#[cfg(test)]` modules in the source file (`src-tauri/src/bench/**`, `src-tauri/src/takeout/**`, `src-tauri/src/commands/health.rs`) and `src/health/view.test.ts`. CI: `.github/workflows/ci.yml`.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| Error/migration types (`DbError`, `Migration`) | unit | Every `as_code()` and `message()` string from the design table | `src-tauri/src/db/mod.rs` tests | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Opening (`open_with`) | integration | 1:1 to DB-01, DB-02 and DB-03 ACs and every spec edge case; each test in its own `%TEMP%` dir | `src-tauri/src/db/mod.rs` tests | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Schema v1 (`0001_initial.sql`) | integration | 1:1 to DB-04, DB-05, DB-06 ACs with real INSERT/UPDATE/DELETE | `src-tauri/src/db/schema_tests.rs` | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Health command (`health_from_open`) | unit | Success and every error path to the pt-BR message | `src-tauri/src/commands/health.rs` tests | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Health view (TS) | unit | Success, invoke error, invalid `schemaVersion` | `src/health/view.test.ts` | `node --experimental-strip-types --test src/health/view.test.ts` |
| Tauri wiring (`lib.rs` setup/state) | none | Build gate; exercised by running the app | `src-tauri/src/lib.rs` | `cargo test` + `npm run build` |

## Parallelism Assessment

| Test Type | Parallel-Safe? | Isolation Model | Evidence |
| --------- | -------------- | --------------- | -------- |
| unit (Rust) | Yes | Pure functions | `commands/health.rs` tests |
| integration (Rust, SQLite file) | Yes | Each test opens its own file in a unique `%TEMP%` dir (`Scratch` pattern); no shared connection; never `E:\` | `src-tauri/src/bench/mod.rs` `Scratch` |
| unit (TS) | Yes | Pure function | `src/health/view.test.ts` |

## Gate Check Commands

| Gate Level | When to Use | Command |
| ---------- | ----------- | ------- |
| Quick | Rust-only tasks | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Full | Tasks touching the frontend | Quick + `node --experimental-strip-types --test src/health/view.test.ts` |
| Build | Last task / wiring | Full + `npm run build` |

---

## Execution Plan

### Phase 1: Opening and migrations (Sequential)

```
T1 → T2 → T3 → T4
```

### Phase 2: Schema and health (Parallel OK)

```
        ┌→ T5 [P]
T4 ─────┤
        └→ T6 [P] ──→ T7
```

### Phase 3: Wiring (Sequential)

```
T5, T7 ──→ T8
```

---

## Task Breakdown

### T1: `DbError` and `Migration` types

**What**: Enum `DbError` with the 8 variants, `as_code()` and `message()` from the design; struct `Migration { version, sql }`; `latest_version()` over a migration list. Replaces `db::select_one` and its two F01 tests (the in-memory probe is gone; design Risk row 6).
**Where**: `src-tauri/src/db/mod.rs`
**Depends on**: None
**Reuses**: `bench::model::BenchError` shape
**Requirement**: DB-01, DB-02, DB-03 (codes)
**Tools**: MCP NONE · Skill NONE

**Done when**:
- [ ] Every code and pt-BR message matches the design table, asserted by a test
- [ ] `latest_version(&[])` is 0 and of `[1, 2]` is 2
- [ ] Quick gate passes

**Tests**: unit · **Gate**: quick
**Commit**: `feat(db): add the database error codes and migration type`

---

### T2: Path validation and `.myphotos` creation

**What**: First half of `open_with`: `path_not_absolute`, `volume_missing`, `parent_missing`, creation of the missing parent with `FILE_ATTRIBUTE_HIDDEN` when the grandparent exists, then `Connection::open`.
**Where**: `src-tauri/src/db/mod.rs`
**Depends on**: T1
**Reuses**: `Component::Prefix` pattern from `bench/corpus.rs`; `Scratch` from `bench/mod.rs`
**Requirement**: DB-01 AC2, AC3; edge cases relative path and existing `.myphotos`
**Tools**: MCP NONE · Skill NONE

**Done when**:
- [ ] Relative path → `path_not_absolute`, nothing created
- [ ] Unused drive letter (first letter A–Z whose root does not exist) → `volume_missing`, nothing created
- [ ] Missing grandparent → `parent_missing`, nothing created
- [ ] Missing parent with existing grandparent → parent created and hidden attribute set
- [ ] Existing parent without hidden attribute → attribute unchanged
- [ ] Quick gate passes

**Tests**: integration · **Gate**: quick
**Commit**: `feat(db): validate the database path and create the hidden folder`

---

### T3: Version check and connection PRAGMAs

**What**: Read `user_version` before any write; `SQLITE_NOTADB` → `db_not_sqlite`; version above latest → `db_newer_than_app`; then `busy_timeout = 5000`, `foreign_keys = ON`, `journal_mode = WAL` (checked), `synchronous = FULL`.
**Where**: `src-tauri/src/db/mod.rs`
**Depends on**: T2
**Requirement**: DB-01 AC6, DB-02 (AC4, AC5)
**Tools**: MCP NONE · Skill NONE

**Done when**:
- [ ] Garbage file → `db_not_sqlite`, bytes identical, no `-wal`/`-shm` created
- [ ] SQLite file with `user_version` 99 → `db_newer_than_app`, bytes identical
- [ ] Opened connection reports `foreign_keys` 1, `journal_mode` `wal`, `synchronous` 2
- [ ] Quick gate passes

**Tests**: integration · **Gate**: quick
**Commit**: `feat(db): refuse foreign or newer files and set the connection pragmas`

---

### T4: Migration runner

**What**: Apply pending migrations in ascending order, one transaction each with `user_version` set inside it; before migrating an existing DB at N ≥ 1, `wal_checkpoint(TRUNCATE)` and copy to `library.db.bak-vN` (overwrite); failure → `migration_failed` / `backup_failed`.
**Where**: `src-tauri/src/db/mod.rs`
**Depends on**: T3
**Requirement**: DB-01 AC1, DB-03
**Tools**: MCP NONE · Skill NONE

**Done when**:
- [ ] New file → `user_version` equals the list's latest, no `.bak`
- [ ] Re-open → no migration, no `.bak`, `sqlite_schema` identical
- [ ] DB at v1 opened with a v2 list → `.bak-v1` exists and opens at version 1; DB at v2
- [ ] Broken v2 → `migration_failed`, version 1, v2's objects absent
- [ ] Existing `.bak-v1` is overwritten
- [ ] Quick gate passes

**Tests**: integration · **Gate**: quick
**Commit**: `feat(db): apply migrations in transactions with a backup first`

---

### T5: Schema v1 [P]

**What**: `migrations/0001_initial.sql` exactly as in the design, registered as `MIGRATIONS[0]`; schema tests.
**Where**: `src-tauri/src/db/migrations/0001_initial.sql`, `src-tauri/src/db/schema_tests.rs`
**Depends on**: T4
**Requirement**: DB-04, DB-05, DB-06
**Tools**: MCP NONE · Skill NONE

**Done when**:
- [ ] Table set equals the 16 names of DB-04 AC1 (ignoring `sqlite_sequence`); the 4 indexes exist
- [ ] UNIQUE on media and folders, CHECK on the 4 media domains, FK on `library_id` (AC2, AC3, AC7)
- [ ] Path update keeps id and links; deleted max id not reused (AC4, AC5)
- [ ] Media delete cascades the 6 child tables and nulls `trash_entries.media_id` keeping paths (AC6)
- [ ] `trashed_at '2026-10-02T12:00:00Z'` → `purge_after '2026-11-01T12:00:00Z'` (AC8)
- [ ] Quick gate passes

**Tests**: integration · **Gate**: quick
**Commit**: `feat(db): add schema v1`

---

### T6: Health command reports the schema version [P]

**What**: `AppHealth { app_name, schema_version }`, `health_from_open(Result<u32, DbError>)`, `DbState`, `app_health(State<DbState>)`.
**Where**: `src-tauri/src/commands/health.rs`
**Depends on**: T4
**Requirement**: DB-07 (backend)
**Tools**: MCP NONE · Skill NONE

**Done when**:
- [ ] `Ok(1)` → JSON `{"appName":"My Photos","schemaVersion":1}`
- [ ] Each `DbError` → `Err(message())`; `VolumeMissing` → `disco E: não encontrado`
- [ ] Quick gate passes

**Tests**: unit · **Gate**: quick
**Commit**: `feat(health): report the schema version`

---

### T7: Health view shows the schema line

**What**: `AppHealth.schemaVersion`, `HealthScreen.schema = "Banco: esquema v{n}"`, invalid version message; `App.tsx` renders `schema` instead of `sqlite`.
**Where**: `src/health/view.ts`, `src/health/view.test.ts`, `src/App.tsx`
**Depends on**: T6
**Requirement**: DB-07
**Tools**: MCP NONE · Skill NONE

**Done when**:
- [ ] `{schemaVersion: 1}` → `Banco: esquema v1`
- [ ] invoke error `disco E: não encontrado` → `Falha: disco E: não encontrado`
- [ ] `schemaVersion` 0 or 1.5 → `Falha: schemaVersion inválido`
- [ ] Full gate passes

**Tests**: unit · **Gate**: full
**Commit**: `feat(health): show the schema version on screen`

---

### T8: Open the library at startup

**What**: `lib.rs` `setup` calls `db::open_library()` and manages `DbState`; registers `app_health` with the state.
**Where**: `src-tauri/src/lib.rs`
**Depends on**: T5, T7
**Requirement**: DB-01 AC7, DB-07
**Tools**: MCP NONE · Skill NONE

**Done when**:
- [ ] Build gate passes (`cargo test`, TS test, `npm run build`)
- [ ] No test calls `open_library` (grep)

**Tests**: none (wiring) · **Gate**: build
**Commit**: `feat(app): open the library database at startup`

---

## Task Granularity Check

| Task | Scope | Status |
| ---- | ----- | ------ |
| T1 | 2 types in one file | ✅ |
| T2 | 1 function stage | ✅ |
| T3 | 1 function stage | ✅ |
| T4 | 1 function stage | ✅ |
| T5 | 1 SQL file + its tests | ✅ |
| T6 | 1 command module | ✅ |
| T7 | 1 view + render line | ⚠️ cohesive (view and its only consumer) |
| T8 | 1 wiring change | ✅ |

## Diagram-Definition Cross-Check

| Task | Depends On (body) | Diagram Shows | Status |
| ---- | ----------------- | ------------- | ------ |
| T1 | None | start | ✅ |
| T2 | T1 | T1 → T2 | ✅ |
| T3 | T2 | T2 → T3 | ✅ |
| T4 | T3 | T3 → T4 | ✅ |
| T5 | T4 | T4 → T5 | ✅ |
| T6 | T4 | T4 → T6 | ✅ |
| T7 | T6 | T6 → T7 | ✅ |
| T8 | T5, T7 | T5, T7 → T8 | ✅ |

T5 and T6 are `[P]` and do not depend on each other. ✅

## Test Co-location Validation

| Task | Code Layer | Matrix Requires | Task Says | Status |
| ---- | ---------- | --------------- | --------- | ------ |
| T1 | Error/migration types | unit | unit | ✅ |
| T2 | Opening | integration | integration | ✅ |
| T3 | Opening | integration | integration | ✅ |
| T4 | Opening | integration | integration | ✅ |
| T5 | Schema v1 | integration | integration | ✅ |
| T6 | Health command | unit | unit | ✅ |
| T7 | Health view | unit | unit | ✅ |
| T8 | Tauri wiring | none | none | ✅ |
