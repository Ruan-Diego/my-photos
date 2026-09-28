# F02 Takeout Discovery Tasks

## Execution Protocol (MANDATORY -- do not skip)

Implement these tasks with the `tlc-spec-driven` skill: **activate it by name and follow its Execute flow and Critical Rules.** Do not search for skill files by filesystem path. The skill is the source of truth for the full flow (per-task cycle, sub-agent delegation, adequacy review, Verifier, discrimination sensor).

**If the skill cannot be activated, STOP and tell the user — do not proceed without it.**

---

**Design**: `.specs/features/f02-takeout-discovery/design.md`
**Status**: In progress

---

## Test Coverage Matrix

> Generated from codebase, project guidelines, and spec — confirm before Execute. Guidelines found: none — strong defaults applied, bounded by `.specs/features/f02-takeout-discovery/spec.md`. No `AGENTS.md`, `CONTRIBUTING.md`, or coverage config exists. Existing tests are the floor: `#[cfg(test)]` modules inside the source file (`src-tauri/src/db/mod.rs`, `src-tauri/src/commands/health.rs`). The CI command is `.github/workflows/ci.yml`.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| Report model | none | Build gate only. The types are the artifact | `src-tauri/src/takeout/model.rs` | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Domain (`classify`, `match_directory`, `parse_media_json`, `parse_album_json`) | unit | All branches; 1:1 to the acceptance criteria those functions decide; every listed edge case of that function has a test | `src-tauri/src/takeout/**/*.rs` tests in the same module | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Discovery walk | integration | Every discovery acceptance criterion: happy path, parent-folder miss, ordering, idempotency, invalid JSON, unreadable, ignored files, orphan, byte snapshot | `src-tauri/src/takeout/mod.rs` tests, fixtures in `src-tauri/fixtures/takeout/` | `cargo test --manifest-path src-tauri/Cargo.toml` |

## Parallelism Assessment

> Generated from codebase — confirm before Execute.

| Test Type | Parallel-Safe? | Isolation Model | Evidence |
| --------- | -------------- | --------------- | -------- |
| unit (Rust) | Yes | Pure functions. No shared store, no file write | `src-tauri/src/db/mod.rs` and `src-tauri/src/commands/health.rs` keep state inside the test |
| integration (Rust, fixture read) | Yes | Fixtures are read-only under `CARGO_MANIFEST_DIR`. The `unreadable` case uses an injected reader, not an OS lock. Tests must not create a `*.db` in the crate directory, because `db` tests list those names | `src-tauri/src/db/mod.rs` filters `*.db` in the process cwd |
| none | Yes | No tests | Model compile only |

## Gate Check Commands

> Generated from codebase — confirm before Execute. There is no lint script. `npm run build` does not compile the Rust library; `cargo test` does.

| Gate Level | When to Use | Command |
| ---------- | ----------- | ------- |
| Quick | After a task whose tests are `cargo test` | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Full | After the discovery integration tests | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Build | After the model-only task and after the last phase | `cargo test --manifest-path src-tauri/Cargo.toml` and `npm run build` and `node --experimental-strip-types --test src/health/view.test.ts` |

---

## Execution Plan

### Phase 1: Foundation (Sequential)

```
T1
```

### Phase 2: Domain (two chains)

```
T1 complete, then:
  T2 [P] ──→ T3 ──→ T4
  T5 [P] ──→ T6
```

`T2` and `T5` start together. `T6` does not wait for `T3`. `T4` waits for `T3`.

### Phase 3: Discovery (Sequential)

```
T4 ──┐
T6 ──┴──→ T7
```

---

## Task Breakdown

### T1: Report types

**What**: Add the discovery report types and mount the `takeout` module on the crate.
**Where**: `src-tauri/src/takeout/model.rs`, `src-tauri/src/takeout/mod.rs`, `src-tauri/src/lib.rs`
**Depends on**: None
**Reuses**: module layout of `src-tauri/src/db/mod.rs`
**Requirement**: shapes consumed by TD-01, TD-11, TD-19, TD-23

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] `model.rs` defines `Confidence`, `UnmatchedReason`, `MediaMetadata`, `MediaHit`, `AlbumMetadata`, `AlbumHit`, `ProblemKind`, `Problem`, `DiscoveryReport` as in the design
- [x] `lib.rs` declares `mod takeout` and `takeout/mod.rs` declares `mod model`
- [x] No `Serialize` on these types
- [x] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0, and `npm run build` exits 0, and `node --experimental-strip-types --test src/health/view.test.ts` exits 0
- [x] The five existing tests still pass. This task adds no test

**Tests**: none
**Gate**: build

**Commit**: `feat(takeout): add the discovery report types`

---

### T2: Classify a file name [P]

**What**: Implement `classify` and test every role the spec names.
**Where**: `src-tauri/src/takeout/classify.rs`
**Depends on**: T1
**Reuses**: `eq_ignore_ascii_case`
**Requirement**: file-role rules used by TD-19, TD-25, TD-26

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Order is `archive_browser.html` → ignored, `metadata.json` → album, known extension → media, other `.json` → sidecar, anything else → ignored
- [x] Comparison is ASCII case-insensitive. Extension on the media role is lowercase without a dot
- [x] Six unit tests pass: one known image, one known video, the album name in different case, a media sidecar, `archive_browser.html`, `notas.txt`
- [x] One of those tests asserts every extension in the spec list is media
- [x] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [x] Test count: 6 new tests in `classify`. No existing test deleted

**Tests**: unit
**Gate**: quick

**Commit**: `feat(takeout): classify takeout file names`

---

### T3: Match exact, supplemental, and truncated names

**What**: Implement truncation and cascade steps 1–3.
**Where**: `src-tauri/src/takeout/matching.rs`
**Depends on**: T2
**Reuses**: `classify` to drop `metadata.json` and non-JSON
**Requirement**: TD-01, TD-02, TD-03, TD-04, TD-05

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] `truncate_sidecar_name` returns the input when it is at most 51 UTF-8 bytes, the 46-byte prefix plus `.json` when it is longer, and `None` when byte 46 splits a character
- [ ] Step 1 is `Exact` for `{file}.json` and for `{stem}.json`. Both present is a tie
- [ ] Step 2 matches a full or truncated `supplemental-metadata` name as `Heuristic`
- [ ] Step 3 matches the truncated `{file}.json` only when step 1 missed, as `Heuristic`
- [ ] Two logical names that truncate to one file are one candidate
- [ ] Six unit tests pass for TD-01, TD-02, TD-03, TD-04, TD-05, and the UTF-8 boundary that returns `None`
- [ ] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] Test count: 6 tests in `matching`. The 6 `classify` tests still pass

**Tests**: unit
**Gate**: quick

**Commit**: `feat(takeout): match exact and truncated sidecar names`

---

### T4: Match duplicate markers, edited files, and ties

**What**: Add cascade steps 4 and 5 and the post-pass that refuses a shared JSON.
**Where**: `src-tauri/src/takeout/matching.rs`
**Depends on**: T3
**Reuses**: truncation from T3
**Requirement**: TD-06, TD-07, TD-08, edge `-edited` sem original

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] `(N)` matches only `\(([1-9][0-9]*)\)$` at the end of the stem. The marker is appended after truncation and may exceed 51 bytes
- [ ] `camphoto_1271212614(1).jpg` matches `camphoto_1271212614.jpg.supplemental-metadata(1).json` as `Heuristic`
- [ ] `foto-edited.jpg` is `Heuristic` on the JSON already held by `foto.jpg`. Its confidence is never `Exact`
- [ ] Two non-edited media that select the same JSON become `None` with `Ambiguous`, and an edited file that only shared it becomes `None` without that reason
- [ ] An edited file whose original is missing or `None` stays `None`
- [ ] Four new unit tests pass, one per bullet above that names a file
- [ ] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] Test count: 10 tests in `matching` (6 from T3 plus 4). No test deleted

**Tests**: unit
**Gate**: quick

**Commit**: `feat(takeout): match edited files and duplicate markers`

---

### T5: Parse media JSON [P]

**What**: Implement `parse_media_json` with per-field absence instead of failing the document.
**Where**: `src-tauri/src/takeout/parse.rs`
**Depends on**: T1
**Reuses**: `serde_json::Value`
**Requirement**: TD-11, TD-12, TD-13, TD-14, TD-15, TD-16, TD-17, TD-18, edges `people` vazio e campo extra

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Bytes that are not JSON return `Err(())`. A bad field does not
- [ ] `taken_at_unix` comes only from `photoTakenTime.timestamp`. `creationTime.timestamp` goes to `creation_time_unix`
- [ ] `geoData` `0.0/0.0` clears latitude, longitude, and altitude. `geoDataExif` is used only then. A real `geoData` wins over `geoDataExif`
- [ ] `people` keeps non-empty names in order. Missing `people` yields an empty list
- [ ] Empty `title`, `description`, and `url` are absent. A non-empty `url` is copied. No network API is called
- [ ] An unknown JSON field is ignored
- [ ] Ten unit tests pass: TD-11 through TD-18, empty `people`, and an extra field
- [ ] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] Test count: 10 new tests in `parse`. No existing test deleted

**Tests**: unit
**Gate**: quick

**Commit**: `feat(takeout): parse media sidecar JSON`

---

### T6: Parse album metadata.json

**What**: Implement `parse_album_json` with the same timestamp, text, and zero-coordinate rules.
**Where**: `src-tauri/src/takeout/parse.rs`
**Depends on**: T5
**Reuses**: the timestamp and coordinate helpers from `parse_media_json`
**Requirement**: TD-20, TD-22

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] The section 9.3 example yields title `Viagem Europa 2022`, absent description, `date_unix` `1660000000`, and no coordinates
- [ ] Invalid bytes return `Err(())` and do not panic
- [ ] Two new unit tests pass, one per bullet
- [ ] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] Test count: 12 tests in `parse` (10 from T5 plus 2). No test deleted

**Tests**: unit
**Gate**: quick

**Commit**: `feat(takeout): parse album metadata.json`

---

### T7: Discover a Takeout tree

**What**: Walk a directory tree, join the matcher and the parsers, and prove the report against fixtures.
**Where**: `src-tauri/src/takeout/mod.rs`, `src-tauri/fixtures/takeout/`
**Depends on**: T4, T6
**Reuses**: `classify`, `match_directory`, `parse_media_json`, `parse_album_json`
**Requirement**: TD-01–TD-10, TD-19–TD-27, edges de `none` sem erro, `orphan_json`, e `-edited` sem original

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] `discover` returns `Err` only when the root is not a readable directory. Symlinks are not followed
- [ ] Report paths use `/` and the three lists are lexicographically ordered by that path
- [ ] A second `discover` on the same tree returns the equal value
- [ ] JSON outside the media's directory does not match. `metadata.json` is never the media sidecar. `Photos from 2022` without that file produces no album
- [ ] Invalid JSON and an injected read error fail only that file. A neighbor with a valid sidecar stays `Exact`
- [ ] `archive_browser.html` and `notas.txt` are absent from media, albums, sidecars, and problems
- [ ] An unmatched `.json` is `OrphanJson` and is not read. A media file with no JSON is `None` with no `InvalidJson` problem
- [ ] Paths, sizes, and bytes of the fixture tree are unchanged after `discover`
- [ ] Fixtures contain empty media files and small JSON. Nothing reads `E:\`
- [ ] Twenty-one integration tests pass, one for each requirement and edge named in this task's Requirement field
- [ ] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0, then `npm run build` exits 0, and `node --experimental-strip-types --test src/health/view.test.ts` exits 0
- [ ] Test count: 54 passing (5 existing + 6 classify + 10 matching + 12 parse + 21 discover). No test deleted

**Tests**: integration
**Gate**: full

**Commit**: `feat(takeout): discover a Takeout tree without writing to it`

---

## Parallel Execution Map

```
Phase 1 (Sequential):
  T1

Phase 2 (two chains):
  T1 complete, then:
    ├── T2 [P] ──→ T3 ──→ T4
    └── T5 [P] ──→ T6

Phase 3 (Sequential):
  T4 and T6 complete, then:
    T7
```

---

## Task Granularity Check

| Task | Scope | Status |
| ---- | ----- | ------ |
| T1: Report types | 1 module of types | ✅ Granular |
| T2: Classify a file name | 1 function | ✅ Granular |
| T3: Match exact, supplemental, and truncated names | 1 module, truncation plus steps 1–3 | ✅ Granular |
| T4: Match duplicate markers, edited files, and ties | 1 increment of the same matcher | ✅ Granular |
| T5: Parse media JSON | 1 function | ✅ Granular |
| T6: Parse album metadata.json | 1 function | ✅ Granular |
| T7: Discover a Takeout tree | 1 function plus the fixtures it reads | ✅ Granular |

---

## Diagram-Definition Cross-Check

| Task | Depends On (task body) | Diagram Shows | Status |
| ---- | ---------------------- | ------------- | ------ |
| T1 | None | Phase 1 start | ✅ Match |
| T2 | T1 | T1 → T2 | ✅ Match |
| T3 | T2 | T2 → T3 | ✅ Match |
| T4 | T3 | T3 → T4 | ✅ Match |
| T5 | T1 | T1 → T5 | ✅ Match |
| T6 | T5 | T5 → T6 | ✅ Match |
| T7 | T4, T6 | T4 → T7 and T6 → T7 | ✅ Match |

T2 and T5 do not depend on each other. T6 does not depend on T3 or T4. The `[P]` marks match that.

---

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| ---- | --------------------------- | --------------- | --------- | ------ |
| T1: Report types | Report model | none | none | ✅ OK |
| T2: Classify a file name | Domain | unit | unit | ✅ OK |
| T3: Match exact, supplemental, and truncated names | Domain | unit | unit | ✅ OK |
| T4: Match duplicate markers, edited files, and ties | Domain | unit | unit | ✅ OK |
| T5: Parse media JSON | Domain | unit | unit | ✅ OK |
| T6: Parse album metadata.json | Domain | unit | unit | ✅ OK |
| T7: Discover a Takeout tree | Discovery walk | integration | integration | ✅ OK |
