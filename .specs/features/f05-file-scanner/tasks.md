# F05 File Scanner Tasks

**Design**: `.specs/features/f05-file-scanner/design.md`
**Status**: Done (T1–T3 committed). CI `windows-latest` is the gate; independent Verifier pending.

> Executado numa sessão na nuvem (Linux) **sem** a skill `tlc-spec-driven`, com autorização do stakeholder ("pode prosseguir", 2026-10-06). O crate não compila no Linux (Tauri pede GTK). Os testes rodaram num harness local sem Tauri, que copia `db`, `takeout`, `scan` e `bench/hash` e troca os helpers só-Windows dos testes antigos. O gate oficial é o CI.

---

## Test Coverage Matrix

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| Tree walk refactor (`takeout`) | integration | F02 tests unchanged and green; `.myphotos` skipped at the root | `src-tauri/src/takeout/mod.rs` tests + `scan/tests.rs` | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Scanner (`scan`) | integration | 1:1 to SCAN-01..06 ACs and every spec edge case; temp dirs + `open_in_memory()` | `src-tauri/src/scan/tests.rs` | same |
| Real library (SCAN-07) | manual | `#[ignore]` `real_library_scan` on the stakeholder's machine | `src-tauri/src/scan/tests.rs` | `cargo test --manifest-path src-tauri/Cargo.toml real_library_scan -- --ignored --nocapture` |

## Task Breakdown

### T1: Share one tree walk between the matcher and the scanner

**What**: `takeout::read_library_tree` (records folders, skips `.myphotos` at the root) and `takeout::discover_tree`; `discover_with` composes them.
**Where**: `src-tauri/src/takeout/mod.rs`
**Requirement**: SCAN-01 AC1, AC2
**Commit**: `refactor(takeout): read the tree once and skip the app folder`

- [x] F02 tests unchanged
- [x] Folders listed, root excluded

### T2: Scanner

**What**: `scan`/`scan_with`, `ScanError`, `ScanSummary`; batches; incremental reuse; missing, moved, folders; runs and errors. `bench::hash` becomes `pub(crate)`; `db::open_in_memory()` for tests.
**Where**: `src-tauri/src/scan/mod.rs`, `src-tauri/src/scan/tests.rs`, `src-tauri/src/bench/{mod,hash}.rs`, `src-tauri/src/db/mod.rs`, `src-tauri/src/lib.rs`
**Requirement**: SCAN-01..07
**Commit**: `feat(scan): scan the library into folders and media_items`

- [x] One test per AC and edge case (see `validation.md`)
- [x] `real_library_scan` ignored

### T3: Docs and state

**Where**: `.specs/features/f05-file-scanner/*`, `.specs/STATE.md`
**Commit**: `docs(f05): design, tasks and validation of the file scanner`
