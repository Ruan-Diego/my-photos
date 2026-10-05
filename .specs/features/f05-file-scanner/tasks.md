# F05 File Scanner Tasks

## Execution Protocol

The `tlc-spec-driven` skill was not available in the cloud session that executed this feature (2026-10-05). The stakeholder approved running it by hand with the same flow: one task at a time, gate after each, then an independent Verifier sub-agent (author ≠ verifier) writing `validation.md`.

---

**Design**: `.specs/features/f05-file-scanner/design.md`
**Status**: In progress

---

## Test Coverage Matrix

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| `iso_utc` | unit | Epoch, a leap day, milliseconds, pre-1970 → None | `src-tauri/src/scan/time.rs` tests | `cargo test --manifest-path src-tauri/Cargo.toml scan::` |
| Walk (`walk`, `RealFs`) | integration (temp dir) | Folders, media, ignored files, `.myphotos`, links, invalid names, unreadable dir | `src-tauri/src/scan/walk.rs` tests | same |
| Scan (`scan_with`) | integration (temp dir + temp SQLite v1) | 1:1 to every SCAN-01..06 AC and edge case | `src-tauri/src/scan/tests.rs` | same |
| `ScanError` | unit | Every code and message | `src-tauri/src/scan/mod.rs` tests | same |
| `f05-scan` | none | Build gate; run locally on `E:` (SCAN-07) | `src-tauri/src/bin/f05-scan.rs` | `cargo build` |

## Parallelism Assessment

| Test Type | Parallel-Safe? | Isolation Model |
| --------- | -------------- | --------------- |
| unit | Yes | Pure |
| integration | Yes | Each test has its own `Scratch` dir and its own SQLite file; never `E:\` |

## Gate Check Commands

| Gate Level | Command |
| ---------- | ------- |
| Quick | `cargo test --manifest-path src-tauri/Cargo.toml scan::` |
| Full | `cargo test --manifest-path src-tauri/Cargo.toml` (Windows/CI: all; Linux cloud: Windows-path tests in `bench`/`db` fail by design) |
| Build | Full + `npm run build` |

---

## Task Breakdown

### T1: ISO-8601 UTC formatting [ ]

**What**: `iso_utc(SystemTime) -> Option<String>` with milliseconds and `Z`.
**Where**: `src-tauri/src/scan/time.rs`
**Requirement**: SCAN-02 (dates)
**Done when**: epoch, 2024-02-29, millisecond truncation and pre-1970 tests pass.

### T2: Walk and `FileSystem` [ ]

**What**: `FileSystem` trait, `RealFs`, `walk(root, fs) -> Result<Walk, WalkAbort>`; skips `.myphotos` at the root, `Skip` entries and non-media files; records `unreadable_directory` and `invalid_name`; detects a vanished root. Makes `takeout::classify` and `is_system_dir` `pub(crate)`.
**Where**: `src-tauri/src/scan/walk.rs`, `src-tauri/src/takeout/`
**Depends on**: T1
**Requirement**: SCAN-02, SCAN-05, SCAN-06
**Done when**: walk tests over a temp tree pass.

### T3: Errors, root validation, library and run rows [ ]

**What**: `ScanError`; `scan_with` validates the root before any write, ensures the `libraries` row, inserts the `indexing_runs` row (`full`/`incremental`).
**Where**: `src-tauri/src/scan/mod.rs`
**Depends on**: T2
**Requirement**: SCAN-01
**Done when**: invalid-root tests show no rows in any table; first run is `full`, second is `incremental`.

### T4: Reconcile [ ]

**What**: One transaction applying the reconciliation table from the design (folders and media).
**Where**: `src-tauri/src/scan/mod.rs`
**Depends on**: T3
**Requirement**: SCAN-02, SCAN-04, SCAN-05 (unreadable dir protection)
**Done when**: new / unchanged / modified / missing / restored / case-only tests pass.

### T5: Hash phase and summary [ ]

**What**: Hash queue in path order, batches of 100, error rows, root-vanished abort, run finalisation and `ScanSummary`. `bench::hash::to_hex` becomes `pub(crate)`.
**Where**: `src-tauri/src/scan/mod.rs`
**Depends on**: T4
**Requirement**: SCAN-02, SCAN-04, SCAN-05, SCAN-06
**Done when**: known-hash, zero-byte, unreadable-file, retry, no-read-on-second-run, root-vanishes and read-only snapshot tests pass.

### T6: `f05-scan` binary [ ]

**What**: `scan_default_library()` and the CLI.
**Where**: `src-tauri/src/scan/mod.rs`, `src-tauri/src/bin/f05-scan.rs`
**Depends on**: T5
**Requirement**: SCAN-07
**Done when**: builds; the user runs it on `E:` (local validation).
