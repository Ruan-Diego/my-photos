# F03 Performance Baseline Tasks

## Execution Protocol (MANDATORY -- do not skip)

Implement these tasks with the `tlc-spec-driven` skill: **activate it by name and follow its Execute flow and Critical Rules.** Do not search for skill files by filesystem path. The skill is the source of truth for the full flow (per-task cycle, sub-agent delegation, adequacy review, Verifier, discrimination sensor).

**If the skill cannot be activated, STOP and tell the user — do not proceed without it.**

---

**Design**: `.specs/features/f03-performance-baseline/design.md`
**Status**: In progress

---

## Test Coverage Matrix

> Generated from codebase, project guidelines, and spec — confirm before Execute. Guidelines found: none — strong defaults applied, bounded by `.specs/features/f03-performance-baseline/spec.md`. No `AGENTS.md`, `CONTRIBUTING.md`, or coverage config exists. Existing tests are the floor: `#[cfg(test)]` modules inside the source file (`src-tauri/src/db/mod.rs`, `src-tauri/src/commands/health.rs`, `src-tauri/src/takeout/**`). The CI command is `.github/workflows/ci.yml`.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| Report model | none | Build gate only. The types are the artifact | `src-tauri/src/bench/model.rs` | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Domain (`file_bytes`, `relative_path`, `validate`, `classify_seek_penalty`, `volume_root_of`, `hash_reader`, `content_digest`) | unit | All branches; 1:1 to the acceptance criteria those functions decide; every listed edge case of that function has a test | `src-tauri/src/bench/**/*.rs` tests in the same module | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Volume query | integration | The Windows call returns `Some` or `None` without panicking, and `classify_seek_penalty` maps that value | `src-tauri/src/bench/volume.rs` | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Baseline orchestration | integration | Happy path for 64 files, cleanup, outside marker, junction, unreadable file, setup failure, cleanup failure, empty root, non-empty root | `src-tauri/src/bench/mod.rs` tests, each with its own temp directory | `cargo test --manifest-path src-tauri/Cargo.toml` |
| CLI | integration | Missing arguments, a validation error, and one successful count-1 run that writes `--output` | `src-tauri/src/bin/f03-bench.rs` and `src-tauri/tests/f03_bench.rs` | `cargo test --manifest-path src-tauri/Cargo.toml` |

## Parallelism Assessment

> Generated from codebase — confirm before Execute.

| Test Type | Parallel-Safe? | Isolation Model | Evidence |
| --------- | -------------- | --------------- | -------- |
| unit (Rust) | Yes | Pure functions or a temp directory unique to the test. No shared store | `src-tauri/src/takeout/classify.rs` and `src-tauri/src/db/mod.rs` keep state inside the test |
| integration (Rust, temp dir or one volume read) | Yes | Each test creates its directory under the process temp dir with a unique name. The volume query only reads. Tests must not create a `*.db` in the crate directory, because `db` tests list those names | `src-tauri/src/db/mod.rs` filters `*.db` in the process cwd |
| none | Yes | No tests | Model compile only |

## Gate Check Commands

> Generated from codebase — confirm before Execute. There is no lint script. `npm run build` does not compile the Rust library; `cargo test` does.

| Gate Level | When to Use | Command |
| ---------- | ----------- | ------- |
| Quick | After a task whose tests are unit tests | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Full | After an integration task | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Build | After the model-only task and after the last task | `cargo test --manifest-path src-tauri/Cargo.toml` and `npm run build` and `node --experimental-strip-types --test src/health/view.test.ts` |

---

## Execution Plan

### Phase 1: Foundation (Sequential)

```
T1
```

### Phase 2: Domain (two chains)

```
T1 complete, then:
  T2 [P] ──→ T4 [P]
         └─→ T5 [P]
  T3 [P] ──→ T6 [P]
```

`T2` and `T3` start together. After `T2`, `T4` and `T5` start together. After `T3`, `T6` starts. `T4`, `T5`, and `T6` do not wait for each other.

### Phase 3: Baseline (Sequential, then the CLI)

```
T4 ──┐
T5 ──┼──→ T7 ──→ T8 [P] ──┐
T6 ──┘         └─→ T9 [P] ──┴──→ T10
```

`T8` and `T9` start after `T7` and do not wait for each other. `T10` waits for both.

---

## Task Breakdown

### T1: Baseline report types

**What**: Add the report types, the stable error codes, and the empty `bench` module on the crate.
**Where**: `src-tauri/src/bench/model.rs`, `src-tauri/src/bench/mod.rs`, `src-tauri/src/bench/corpus.rs`, `src-tauri/src/bench/hash.rs`, `src-tauri/src/bench/volume.rs`, `src-tauri/src/lib.rs`
**Depends on**: None
**Reuses**: module layout of `src-tauri/src/db/mod.rs` and `serde` derive already in `src-tauri/Cargo.toml`
**Requirement**: shapes consumed by PERF-01, PERF-02, PERF-03, PERF-04

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] `BaselineReport`, `HashFailure`, and `VolumeClass` match the design and derive `Serialize` with camelCase JSON names (`itemCount`, `bytesPerItem`, `volumeClass`, `volumeRoot`, `workerCount`, `filesHashed`, `totalBytes`, `failureCount`, `failures`, `contentDigest`, `walkElapsedMs`, `hashElapsedMs`, `ok`)
- [x] `VolumeClass` serializes as `hdd`, `ssd`, or `unknown`. `BenchError::as_code` returns `count_out_of_range`, `root_not_absolute`, `root_is_volume`, `parent_missing`, `root_not_empty`, `setup_failed`, and `cleanup_failed`
- [x] `lib.rs` declares `pub mod bench`. `bench/mod.rs` declares `model`, `corpus`, `hash`, and `volume`. The three domain files exist and are empty of logic
- [x] `invoke_handler` still lists only `app_health`
- [x] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0, and `npm run build` exits 0, and `node --experimental-strip-types --test src/health/view.test.ts` exits 0
- [x] The 54 existing tests still pass. This task adds no test

**Tests**: none
**Gate**: build

**Commit**: `feat(bench): add the baseline report types`

---

### T2: Synthetic file bytes and paths [P]

**What**: Implement `file_bytes` and `relative_path` from the spec rules.
**Where**: `src-tauri/src/bench/corpus.rs`
**Depends on**: T1
**Reuses**: nothing
**Requirement**: PERF-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] `file_bytes(i)` returns 4096 bytes and byte `j` is `(i + j) % 251`
- [x] `relative_path` uses `/` and the spec formula. Index 0 is `f03/2000/01/item-0000000.bin`. Index 100 is month `02`. Index 1199 is still year 2000. Index 1200 is `f03/2001/01/item-0001200.bin`
- [x] Two unit tests pass, one for the byte rule and one for those four indexes
- [x] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [x] Test count: 2 new tests in `corpus`. No existing test deleted

**Tests**: unit
**Gate**: quick

**Commit**: `feat(bench): define synthetic file bytes and paths`

---

### T3: Classify seek penalty [P]

**What**: Implement the pure `classify_seek_penalty` and `volume_root_of`.
**Where**: `src-tauri/src/bench/volume.rs`
**Depends on**: T1
**Reuses**: `VolumeClass` from T1
**Requirement**: PERF-03

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] `Some(true)` is `hdd`, `Some(false)` is `ssd`, and `None` is `unknown`
- [x] `volume_root_of` turns a drive path into an uppercase letter plus `:\`, including a lowercase drive letter. It does not call Windows
- [x] Four unit tests pass: one per `classify_seek_penalty` input, plus one for the volume-root formatting
- [x] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [x] Test count: 4 new tests in `volume`. No existing test deleted

**Tests**: unit
**Gate**: quick

**Commit**: `feat(bench): classify volume seek penalty`

---

### T4: Reject unsafe benchmark roots [P]

**What**: Implement `validate` so a bad count or a bad directory fails before any file is created.
**Where**: `src-tauri/src/bench/corpus.rs`
**Depends on**: T2
**Reuses**: `BenchError` from T1
**Requirement**: PERF-01

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] Count `0` and `150001` return `count_out_of_range`. Count `1` and `150000` are accepted and create no file
- [x] A relative path and a UNC path return `root_not_absolute`
- [x] A path with `..` returns `root_not_absolute`
- [x] A volume root returns `root_is_volume`
- [x] A missing parent returns `parent_missing` and creates nothing
- [x] A non-empty directory returns `root_not_empty` and the marker file inside it is unchanged. An absent root whose parent exists returns `Ok` and the root still does not exist
- [x] Six unit tests pass: count bounds, relative/UNC, `..`, volume root, missing parent, and the empty-or-absent directory pair
- [x] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [x] Test count: 8 tests in `corpus` (2 from T2 plus 6). No test deleted

**Tests**: unit
**Gate**: quick

**Commit**: `feat(bench): reject unsafe benchmark roots`

---

### T5: Hash files in order [P]

**What**: Implement streaming SHA-256 and the aggregate `content_digest`.
**Where**: `src-tauri/src/bench/hash.rs`, `src-tauri/Cargo.toml`
**Depends on**: T2
**Reuses**: `file_bytes` and `sha2` 0.11.0 (`Sha256`, `Digest`)
**Requirement**: PERF-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] `Cargo.toml` depends on `sha2` version `0.11.0`
- [x] `hash_reader` feeds `Sha256` in chunks of at most 65536 bytes. An I/O error returns `Err`
- [x] Digest of `file_bytes(0)` is `d67c656e01756650d77717b0839985a056ec28ffe174601d690fc407a2ceffca`. Index 1 is `29fa02d3381a0b093ff2d6d783c1c78176fd1aafce643c8d205631dc85df5e95`. Index 63 is `b21fe85ea1fba0db5690a657db55547f0cf8cb4a9ea81ac332e67de8de84be06`
- [x] `content_digest` of indexes 0 through 63 is `c067a4e6cde3d7306e1fef52a92ebee10c1e562368ad06488bb1398610067415`. No successes yields an empty string. A reader that yields the bytes in pieces smaller than 4096 still matches index 0
- [x] Five unit tests pass, covering the three known file digests, the 64-file aggregate, the empty aggregate, the chunked reader, and a reader error
- [x] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [x] Test count: 5 new tests in `hash`. No existing test deleted

**Tests**: unit
**Gate**: quick

**Commit**: `feat(bench): hash synthetic files with sha-256`

---

### T6: Query the Windows seek penalty [P]

**What**: Implement `query_seek_penalty` with `windows-sys` and map a failure to `None`.
**Where**: `src-tauri/src/bench/volume.rs`, `src-tauri/Cargo.toml`
**Depends on**: T3
**Reuses**: `classify_seek_penalty`, `STORAGE_PROPERTY_QUERY`, `DEVICE_SEEK_PENALTY_DESCRIPTOR`, `IOCTL_STORAGE_QUERY_PROPERTY`, `StorageDeviceSeekPenaltyProperty` from `windows-sys` 0.61.2
**Requirement**: PERF-03

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] On Windows, `Cargo.toml` depends on `windows-sys` version `0.61.2` with features `Win32_Foundation`, `Win32_Storage_FileSystem`, `Win32_System_IO`, and `Win32_System_Ioctl`
- [x] The query opens `\\.\X:` and sends `StorageDeviceSeekPenaltyProperty`. `CreateFileW` or `DeviceIoControl` failing returns `None`. Non-Windows builds return `None` and do not link those APIs
- [x] One integration test calls `query_seek_penalty` for the volume of the temp directory and checks that `classify_seek_penalty` of that answer is `hdd`, `ssd`, or `unknown`
- [x] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [x] Test count: 5 tests in `volume` (4 from T3 plus 1). No test deleted

**Tests**: integration
**Gate**: full

**Commit**: `feat(bench): query windows seek penalty`

---

### T7: Measure walk and hash, then delete the root

**What**: Implement `run_baseline` for a successful corpus, including the reparse-point skip and cleanup.
**Where**: `src-tauri/src/bench/mod.rs`
**Depends on**: T4, T5, T6
**Reuses**: `validate`, `file_bytes`, `relative_path`, `hash_reader`, `content_digest`, `query_seek_penalty`, `volume_root_of`
**Requirement**: PERF-01, PERF-02, PERF-03

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [x] A run of 64 files returns `ok` true, `filesHashed` 64, `totalBytes` 262144, `bytesPerItem` 4096, the known `contentDigest`, and integer `walkElapsedMs` and `hashElapsedMs` measured apart from corpus creation. The root no longer exists, whether the caller created it empty or the function created it
- [x] A second run on a new empty root returns the same `contentDigest`. `workerCount` is at least 1. On Windows, `volumeClass` is `hdd`, `ssd`, or `unknown` and `volumeRoot` ends with `:\`
- [x] A marker file outside the root is unchanged. A directory junction inside the corpus pointing outside is not walked, the outside sentinel is not hashed, and that sentinel still exists after cleanup
- [x] Serialized JSON uses the camelCase names from T1
- [x] Nothing reads or writes `E:\` unless that path is the explicit root
- [x] Seven integration tests pass, one for each bullet of behavior above that names a distinct scenario (created root, pre-existing empty root, second run, volume and workers, outside marker, junction, JSON names)
- [x] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [x] Test count: 7 new tests in `bench`. No test deleted

**Tests**: integration
**Gate**: full

**Commit**: `feat(bench): measure a walk and hash, then delete the root`

---

### T8: Keep partial failures inside the corpus [P]

**What**: Add `run_baseline_with` so one bad file, a failed setup, or a failed cleanup does not destroy anything outside the root.
**Where**: `src-tauri/src/bench/mod.rs`
**Depends on**: T7
**Reuses**: the success path from T7 and the `Hooks` from the design
**Requirement**: PERF-01, PERF-02

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] An injected read error on one file leaves `ok` false, one `failures` item with `reason` `unreadable` and a `/` relative path, `filesHashed` 63, and the root removed
- [ ] Every file failing the read leaves `filesHashed` 0, `contentDigest` empty, `ok` false, and the root removed
- [ ] `fail_setup_at` returns `setup_failed`, removes what the call created, and leaves a marker in the parent unchanged
- [ ] `fail_cleanup` returns `cleanup_failed` and leaves the root in place. The test removes that leftover itself
- [ ] `run_baseline` on a non-empty directory returns `root_not_empty` and does not change the marker file
- [ ] Five integration tests pass, one per bullet
- [ ] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] Test count: 12 tests in `bench` (7 from T7 plus 5). No test deleted

**Tests**: integration
**Gate**: full

**Commit**: `feat(bench): keep benchmark failures inside the corpus`

---

### T9: Add the f03-bench command [P]

**What**: Add a console binary that calls `run_baseline` and prints the JSON.
**Where**: `src-tauri/src/bin/f03-bench.rs`, `src-tauri/tests/f03_bench.rs`
**Depends on**: T7
**Reuses**: `run_baseline`. Do not copy `windows_subsystem` from `src-tauri/src/main.rs`
**Requirement**: PERF-04

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] `f03-bench` requires `--root` and `--count`. Missing arguments print `usage` to stderr and exit 2, creating nothing
- [ ] A validation error prints that `as_code()` to stderr and exits 1. `--count 0` does not create the root
- [ ] `--count 1` with `--output` exits 0, writes pretty JSON to that file and stdout, and removes the root
- [ ] One unit test covers missing arguments. Two integration tests cover the validation exit and the count-1 output file. They start the binary through `CARGO_BIN_EXE_f03_bench`
- [ ] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] Test count: 3 new tests. No test deleted

**Tests**: integration
**Gate**: full

**Commit**: `feat(bench): add the f03-bench command`

---

### T10: Record the 50000 and 150000 baselines

**What**: Run the official sizes on each present volume class and save the JSON reports.
**Where**: `.specs/features/f03-performance-baseline/baselines/`
**Depends on**: T8, T9
**Reuses**: the `f03-bench` command from T9
**Requirement**: PERF-04

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] For each of `hdd` and `ssd` that this machine can classify, two runs finish with `ok` true: `--count 50000` and `--count 150000`. Each root is an empty directory on that volume, not a volume root, and not a directory that already has files
- [ ] Each report is saved as `.specs/features/f03-performance-baseline/baselines/{volumeClass}-{itemCount}.json` with `filesHashed` equal to the count, `bytesPerItem` 4096, and `failureCount` 0. The root is gone afterward
- [ ] A class this machine does not have is stated as not measured in `baselines/README.md`. That absence does not fail the task
- [ ] No path under the real library is used unless it is a fresh empty subdirectory the operator created for the run. The run does not delete anything outside the chosen root
- [ ] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0, and `npm run build` exits 0, and `node --experimental-strip-types --test src/health/view.test.ts` exits 0
- [ ] Test count: 87 passing. That is the 54 tests already in the crate, plus 2 in `corpus` from T2, plus 6 from T4, plus 4 in `volume` from T3, plus 1 from T6, plus 5 in `hash` from T5, plus 7 in `bench` from T7, plus 5 from T8, plus 3 from T9. No test deleted

**Tests**: none
**Gate**: build

**Commit**: `feat(bench): record local 50k and 150k baselines`

---

## Parallel Execution Map

```
Phase 1 (Sequential):
  T1

Phase 2 (two chains):
  T1 complete, then:
    ├── T2 [P] ──→ T4 [P]
    │          └─→ T5 [P]
    └── T3 [P] ──→ T6 [P]

Phase 3 (Sequential, then the CLI):
  T4, T5, and T6 complete, then:
    T7 ──→ T8 [P] ──┐
       └─→ T9 [P] ──┴──→ T10
```

---

## Task Granularity Check

| Task | Scope | Status |
| ---- | ----- | ------ |
| T1: Baseline report types | 1 type module, plus empty files so later tasks do not edit `mod.rs` together | ✅ Granular |
| T2: Synthetic file bytes and paths | 2 pure functions in one file | ✅ Granular |
| T3: Classify seek penalty | 2 pure functions in one file | ✅ Granular |
| T4: Reject unsafe benchmark roots | 1 function | ✅ Granular |
| T5: Hash files in order | 2 functions in one file | ✅ Granular |
| T6: Query the Windows seek penalty | 1 function | ✅ Granular |
| T7: Measure walk and hash, then delete the root | 1 function | ✅ Granular |
| T8: Keep partial failures inside the corpus | 1 increment of the same orchestrator | ✅ Granular |
| T9: Add the f03-bench command | 1 binary | ✅ Granular |
| T10: Record the 50000 and 150000 baselines | 1 evidence directory | ✅ Granular |

---

## Diagram-Definition Cross-Check

| Task | Depends On (task body) | Diagram Shows | Status |
| ---- | ---------------------- | ------------- | ------ |
| T1 | None | Phase 1 start | ✅ Match |
| T2 | T1 | T1 → T2 | ✅ Match |
| T3 | T1 | T1 → T3 | ✅ Match |
| T4 | T2 | T2 → T4 | ✅ Match |
| T5 | T2 | T2 → T5 | ✅ Match |
| T6 | T3 | T3 → T6 | ✅ Match |
| T7 | T4, T5, T6 | T4 → T7, T5 → T7, T6 → T7 | ✅ Match |
| T8 | T7 | T7 → T8 | ✅ Match |
| T9 | T7 | T7 → T9 | ✅ Match |
| T10 | T8, T9 | T8 → T10, T9 → T10 | ✅ Match |

`T2` and `T3` do not depend on each other. `T4`, `T5`, and `T6` do not depend on each other. `T8` and `T9` do not depend on each other. The `[P]` marks match that.

---

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| ---- | --------------------------- | --------------- | --------- | ------ |
| T1: Baseline report types | Report model | none | none | ✅ OK |
| T2: Synthetic file bytes and paths | Domain | unit | unit | ✅ OK |
| T3: Classify seek penalty | Domain | unit | unit | ✅ OK |
| T4: Reject unsafe benchmark roots | Domain | unit | unit | ✅ OK |
| T5: Hash files in order | Domain | unit | unit | ✅ OK |
| T6: Query the Windows seek penalty | Volume query | integration | integration | ✅ OK |
| T7: Measure walk and hash, then delete the root | Baseline orchestration | integration | integration | ✅ OK |
| T8: Keep partial failures inside the corpus | Baseline orchestration | integration | integration | ✅ OK |
| T9: Add the f03-bench command | CLI | integration | integration | ✅ OK |
| T10: Record the 50000 and 150000 baselines | Evidence files, no code layer | none | none | ✅ OK |
