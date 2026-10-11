# F06 Takeout JSON Parser Tasks

## Execution Protocol

Same as F05: the `tlc-spec-driven` skill is not available in the cloud session, so tasks run by hand one at a time with a gate after each, then an independent Verifier sub-agent writes `validation.md`.

---

**Design**: `.specs/features/f06-takeout-json-parser/design.md`
**Status**: Done — T1–T3

---

## Test Coverage Matrix

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| `time::iso_from_unix_seconds` | unit | 0, negative, both range ends, one past each end | `src-tauri/src/time.rs` tests | `cargo test --manifest-path src-tauri/Cargo.toml time::` |
| `takeout::parse` | unit (bytes in memory) | 1:1 to JSON-01..05 ACs and edge cases; F02 TD tests kept | `src-tauri/src/takeout/parse.rs` tests | `cargo test --manifest-path src-tauri/Cargo.toml takeout::` |
| `read_json_capped` + `discover` | integration (temp dir) | JSON-05 AC1 read bound, JSON-06 | `src-tauri/src/takeout/mod.rs` tests | same |

## Gate Check Commands

| Gate Level | Command |
| ---------- | ------- |
| Quick | `cargo test --manifest-path src-tauri/Cargo.toml -- time:: takeout:: scan::` |
| Full | `cargo test --manifest-path src-tauri/Cargo.toml` (Windows/CI) |

---

## Task Breakdown

### T1: Move ISO formatting to `crate::time` and add `iso_from_unix_seconds` [x]

**Where**: `src-tauri/src/time.rs`, `src-tauri/src/scan/*`
**Requirement**: JSON-01
**Done when**: `time::` tests (old + range/negative) and `scan::` pass.

### T2: Models and parser hardening [x]

**What**: `LocationSource`, new fields, `normalized_person_name`, `MAX_SIDECAR_BYTES`, BOM/UTF-8/root checks, coordinate and instant ranges, text/people cleaning, `raw_json`; F02 TD tests adjusted only where the rule changed.
**Where**: `src-tauri/src/takeout/model.rs`, `src-tauri/src/takeout/parse.rs`
**Depends on**: T1
**Requirement**: JSON-01..05
**Done when**: every AC and edge case has a passing test.

### T3: Capped reader in `discover` [x]

**Where**: `src-tauri/src/takeout/mod.rs`
**Depends on**: T2
**Requirement**: JSON-05 AC1, JSON-06
**Done when**: oversize and non-object sidecars become `invalid_json` with the neighbour still matched; reader reads at most 1 MiB + 1.
