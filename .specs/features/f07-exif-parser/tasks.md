# F07 EXIF Parser Tasks

## Execution Protocol

Same as F05/F06: tasks by hand, gate after each, then an independent Verifier sub-agent writes `validation.md`.

---

**Design**: `.specs/features/f07-exif-parser/design.md`
**Status**: Done — T1–T3

---

## Test Coverage Matrix

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| `time` additions | unit | `days_from_civil` round-trips `civil_from_days`; millis formatting and range | `src-tauri/src/time.rs` | `cargo test --manifest-path src-tauri/Cargo.toml time::` |
| `photo_meta::date` | unit | Every EXIF-01/02 parsing rule and edge case | `src-tauri/src/photo_meta/date.rs` | `cargo test --manifest-path src-tauri/Cargo.toml photo_meta::` |
| `read_embedded` | integration (generated JPEG/PNG in temp dirs) | 1:1 to EXIF-01..06 ACs | `src-tauri/src/photo_meta/tests.rs` | same |

## Gate Check Commands

| Gate Level | Command |
| ---------- | ------- |
| Quick | `cargo test --manifest-path src-tauri/Cargo.toml -- time:: photo_meta:: takeout:: scan::` |
| Full | `cargo test --manifest-path src-tauri/Cargo.toml` (Windows/CI) |

---

## Task Breakdown

### T1: `time` helpers [x]

**What**: `days_from_civil`, `iso_from_unix_millis`.
**Requirement**: EXIF-01, EXIF-02
**Done when**: round-trip and range tests pass; F05/F06 tests unchanged.

### T2: Date, sub-second and offset parsing [x]

**Where**: `src-tauri/src/photo_meta/date.rs`
**Depends on**: T1
**Requirement**: EXIF-01, EXIF-02
**Done when**: unit tests for every format rule pass.

### T3: `read_embedded` [x]

**What**: dependencies `kamadak-exif` 0.6.1 and `imagesize` 0.15.0; dimensions, EXIF fields, GPS, orientation, raw JSON, corrupt/absent handling.
**Where**: `src-tauri/src/photo_meta/mod.rs`, `src-tauri/Cargo.toml`
**Depends on**: T2
**Requirement**: EXIF-01..06
**Done when**: generated-image tests for every AC pass.
