# F01 Project Setup Validation

**Date**: 2026-09-27
**Spec**: `.specs/features/f01-project-setup/spec.md`
**Diff range**: `d947678..HEAD` (`9bbe8b6`)
**Verifier**: independent sub-agent (author ≠ verifier)
**Iteration**: 2 (re-verification after the previous FAIL)

---

## Task Completion

`tasks.md` Done-when boxes are still unchecked. Status below is taken from the commit range.

| Task | Status | Notes |
| ---- | ------ | ----- |
| T1 | ✅ Done | `2d1b7eb` scaffold |
| T2 | ✅ Done | `65a13f1` in-memory SQLite, then moved into `src-tauri/src/db/` by `9bbe8b6` |
| T3 | ✅ Done | `826dfd8` `app_health` mapping; JSON keys locked in `a07e8a2` |
| T4 | ✅ Done | `e84c1f2` proof shell; build starts the motion-dependency check in `a07e8a2` |
| T5 | ✅ Done | `e1c202f` README |
| T6 | ✅ Done | `9a3dc6d` CI |

Follow-up commits `a07e8a2` and `9bbe8b6` close the three gaps from iteration 1. None blocked. No partial commits.

---

## Spec-Anchored Acceptance Criteria

### P1: Comprovante da stack no Windows

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| WHEN `npm run tauri dev` runs on Windows 10/11 THEN the window title is exactly `My Photos` | Window title `My Photos` | `src-tauri/tauri.conf.json:16` — `"title": "My Photos"`; `package.json:10` — `"tauri": "tauri"`. Desktop window was not launched | ✅ PASS (config). Launch not observed |
| WHEN the window finishes loading THEN the UI invokes `app_health` with no arguments and renders exactly `My Photos está pronto.` | Exact string `My Photos está pronto.` after `invoke("app_health")` | `src/health/view.test.ts:8` — `assert.equal(screen.ready, "My Photos está pronto.")`; call site `src/App.tsx:26` — `invoke<AppHealth>("app_health")` with no argument object | ✅ PASS |
| WHEN `app_health` opens in-memory SQLite, runs `SELECT 1`, and gets `1` THEN it returns `{ "appName": "My Photos", "sqliteOk": true }` and the UI renders exactly `SQLite: ok` | JSON object keys `appName` and `sqliteOk`, values `"My Photos"` and `true`, plus `SQLite: ok` | `src-tauri/src/commands/health.rs:37-40` — `assert_eq!(serde_json::to_string(&health)..., r#"{"appName":"My Photos","sqliteOk":true}"#)` ; `src/health/view.test.ts:9` — `assert.equal(screen.sqlite, "SQLite: ok")` | ✅ PASS |
| WHEN the connection or `SELECT 1` fails THEN `app_health` returns an error and the UI renders `Falha: ` plus the message, and does not render the success strings | Error, prefix `Falha: `, success strings absent | `src-tauri/src/commands/health.rs:52` — `assert_eq!(message, "disco")`; `src/health/view.test.ts:17-20` — `assert.equal(screen.failure, "Falha: boom")` and success fields `null` | ✅ PASS |
| WHEN `app_health` runs THEN no file outside the project and the process temp is created, changed, or deleted, and no library `.db` is created | No library `.db` | `src-tauri/src/db/mod.rs:22-23` and `:31-33` — `assert_eq!(select_one()..., 1)` and `assert_eq!(database_file_names(&directory), before)` for top-level `*.db` in the process cwd. Opener is `Connection::open_in_memory` at `db/mod.rs:4` | ✅ PASS |

### P1: Bibliotecas de movimento entram no build

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| WHEN `package.json` is read THEN `dependencies` includes `three`, `gsap`, `animejs`, and `motion` | Those four dependency names | `package.json:15-20` — `"animejs"`, `"gsap"`, `"motion"`, `"three"` | ✅ PASS |
| WHEN the Vite entry imports each package and exposes the names THEN the window renders exactly `three, gsap, animejs, motion` | Exact list string | `src/health/view.test.ts:10` — `assert.equal(screen.libraries, "three, gsap, animejs, motion")`; imports `src/motion/libraries.ts:1-4` | ✅ PASS |
| WHEN `npm run build` runs THEN it succeeds, including TypeScript. WHEN any of the four packages is removed from `package.json` THEN `npm run build` fails | Exit 0 with the packages present; non-zero after removing one dependency entry, even if `node_modules` still has the package | Happy path: `npm run build` exit 0 (`node scripts/require-motion-deps.mjs && tsc && vite build`). Negative path, scratch copy of `package.json`, restored afterward: deleted only the `"motion"` dependency while `node_modules/motion/package.json` stayed present. `scripts/require-motion-deps.mjs:8-10` printed `dependências ausentes: motion` and `npm run build` exited 1. `package.json` SHA-256 matched the pre-edit hash after restore | ✅ PASS |
| WHEN the proof window is visible THEN the system does not create a WebGL renderer or start an animation loop | No `WebGLRenderer`, no animation call | `src/motion/libraries.ts:7-17` reads `REVISION`, `gsap`, `animate`, and `motion` and returns the spec line. No `WebGLRenderer` construction and no `animate()` / `gsap.to` call. No test locks that negative | ✅ PASS (code inspection) |

### P1: Layout do repositório

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| WHEN the repo is inspected THEN `src-tauri/` is the Tauri 2 Rust backend and `src/` is the React + TypeScript + Vite frontend | Both trees exist | `src-tauri/Cargo.toml:21` `tauri` 2; `src/main.tsx:1-8`; `package.json:7` `tsc && vite build` | ✅ PASS |
| WHEN the backend is inspected THEN `src-tauri/src/commands/` contains `app_health` and `src-tauri/src/db/` contains the in-memory open | Directory `src-tauri/src/db/` plus `commands/` | `src-tauri/src/commands/health.rs:23-26` defines `app_health`. `src-tauri/src/db/mod.rs:4` calls `Connection::open_in_memory`. `src-tauri/src/db.rs` is absent on disk and absent from `HEAD` | ✅ PASS |
| WHEN `src-tauri/Cargo.toml` is read THEN `tauri` is 2.x and `rusqlite` uses feature `bundled` | Tauri 2 and bundled `rusqlite` | `src-tauri/Cargo.toml:21` — `tauri = { version = "2" ... }`; `Cargo.toml:25` — `rusqlite = { version = "0.40.2", features = ["bundled"] }` | ✅ PASS |
| WHEN the root README is read THEN it documents Windows 10/11, Rust stable, Node.js Active LTS, WebView2, and `npm run tauri dev` | Those four prerequisites and that command | `README.md:5-8` and `README.md:17` | ✅ PASS |

### P1: CI básico

| Criterion (WHEN X THEN Y) | Spec-defined outcome | `file:line` + assertion | Result |
| ------------------------- | -------------------- | ----------------------- | ------ |
| WHEN there is a `push` or `pull_request` THEN `.github/workflows/ci.yml` runs `npm run build` and `cargo test` on `src-tauri` | Both commands on both events | `.github/workflows/ci.yml:4-5`, `:19`, `:21` | ✅ PASS |
| WHEN `npm run build` or `cargo test` exits non-zero THEN the workflow fails | Job failure | `.github/workflows/ci.yml:19-21` are plain `run` steps. No `continue-on-error` | ✅ PASS |
| WHEN `cargo test` runs THEN at least one test opens in-memory SQLite with `rusqlite`, runs `SELECT 1`, and asserts the value is `1` | Asserted value `1` | `src-tauri/src/db/mod.rs:22` — `assert_eq!(select_one().expect("SELECT 1"), 1)` | ✅ PASS |
| WHEN the workflow runs THEN it does not launch the desktop window | No `tauri dev` | `.github/workflows/ci.yml` steps are `npm ci`, `npm run build`, the Node view test, and `cargo test` | ✅ PASS |

**Status**: ✅ All ACs covered (17/17 matched, 0 gaps, 0 spec-precision gaps)

---

## Discrimination Sensor

Each mutant was applied to a backup copy of one file, tests were run, and the file was restored from that backup. SHA-256 of each file matched the pre-mutation hash. `git diff` after the sensor was empty.

| Mutation | File:line | Description | Killed? |
| -------- | --------- | ----------- | ------- |
| 1 | `src-tauri/src/commands/health.rs:6` | Removed `#[serde(rename_all = "camelCase")]` | ✅ Killed — `cargo test` `probe_of_one_reports_my_photos_ready` exit 101; left `"{\"app_name\":\"My Photos\",\"sqlite_ok\":true}"`, right `"{\"appName\":\"My Photos\",\"sqliteOk\":true}"` |
| 2 | `src/health/view.ts:38` | Success `ready` string changed to `My Photos pronto.` | ✅ Killed — Node test exit 1; `view.test.ts:8` expected `My Photos está pronto.` |
| 3 | `src-tauri/src/db/mod.rs:6` | Query changed from `SELECT 1` to `SELECT 2` | ✅ Killed — `cargo test` `db::tests` exit 101; both db tests failed (`left: 2`, `right: 1`) |

**Sensor depth**: lightweight (3 behavior-level mutations)
**Result**: 3/3 killed — ✅ PASS

The iteration-1 surviving serializer mutant is now killed by the JSON assertion.

---

## Interactive UAT Results (if performed)

Not performed. This pass did not launch `npm run tauri dev`. Window title evidence is the Tauri config, not a live window.

| # | Test | Result | Details |
| - | ---- | ------ | ------- |
| 1 | Open the window and read the title and success strings | ⏭️ Skip | No desktop session in this verifier pass |

---

## Code Quality

| Principle | Status |
| --------- | ------ |
| Minimum code | ✅ |
| Surgical changes | ✅ |
| No scope creep | ✅ |
| Matches patterns | ✅ |
| Spec-anchored outcome check (asserted values match spec) | ✅ Serialized health keys, success/failure strings, `SELECT 1`, and the motion-dependency build failure match the spec |
| Per-layer Coverage Expectation met (domain 1:1 ACs; routes happy+edge+error) | ✅ Domain tests cover the serialized payload, probe errors, `SELECT 1`, and the second call. The view covers happy, invoke error, and `sqliteOk !== true`. Motion-package removal fails `npm run build` |
| Every test maps to a spec requirement — no unclaimed tests | ✅ The three Node tests and five Rust lib tests map to SETUP-02, SETUP-03, SETUP-04, SETUP-05, SETUP-07, SETUP-16, the `sqliteOk` edge, or the T3 message `SELECT 1 retornou {n}` |
| Documented guidelines followed: none — strong defaults applied (`tasks.md` coverage matrix) | ✅ |

---

## Edge Cases

- [x] WebView2 missing: no WebView2 installer is in the scaffold. README lists WebView2 as a prerequisite (`README.md:8`).
- [x] Rust or Node missing: no toolchain downloader. README is the prerequisite reference (`README.md:5-8`).
- [x] `app_health` twice: `select_one` opens `Connection::open_in_memory` on each call (`db/mod.rs:4`). `db/mod.rs:31-33` asserts two calls each return `1` and that the top-level `*.db` set does not change.
- [x] `sqliteOk` other than `true`: `src/health/view.test.ts:26-29` asserts `Falha: sqliteOk inválido` and null success strings, including the motion line.

---

## Gate Check

- **Gate command**: Build gate from `tasks.md`: `npm run build` and `cargo test --manifest-path src-tauri/Cargo.toml` and `node --experimental-strip-types --test src/health/view.test.ts`. Cargo was invoked after `vcvarsx86_amd64.bat` with `%USERPROFILE%\.cargo\bin` on PATH.
- **Result**: 8 passed, 0 failed, 0 skipped. `npm run build` exit 0.
- **Test count before feature** (`d947678`): 0 (no `src/` tree and no test files)
- **Test count after feature**: 5 Rust lib tests + 3 Node tests
- **Delta**: +8
- **Skipped tests**: none. Cargo also ran the `main.rs` and doc-test harnesses with 0 tests each; those are empty harnesses, not skips.
- **Failures**: none

Node (`src/health/view.test.ts`): 3 passed. Rust lib: `probe_error_is_returned_unchanged`, `probe_of_one_reports_my_photos_ready`, `probe_other_than_one_is_an_error`, `select_one_returns_1`, `second_call_returns_1_without_creating_a_db_file`.

---

## Fix Plans (if issues found)

None. The three iteration-1 gaps were re-tested and hold:

1. Removing `motion` from `package.json` `dependencies` makes `npm run build` exit 1 while `node_modules/motion` remains.
2. Serializing the success health value is the JSON object `{"appName":"My Photos","sqliteOk":true}`.
3. The in-memory open lives in `src-tauri/src/db/mod.rs`, not `src-tauri/src/db.rs`.

---

## Requirement Traceability Update

`spec.md` was not edited. Recommended statuses only:

| Requirement | Previous Status | New Status |
| ----------- | --------------- | ---------- |
| SETUP-01 | Pending | ✅ Verified (config; window not launched) |
| SETUP-02 | Pending | ✅ Verified |
| SETUP-03 | Pending | ✅ Verified |
| SETUP-04 | Pending | ✅ Verified |
| SETUP-05 | Pending | ✅ Verified |
| SETUP-06 | Pending | ✅ Verified |
| SETUP-07 | Pending | ✅ Verified |
| SETUP-08 | Pending | ✅ Verified |
| SETUP-09 | Pending | ✅ Verified (inspection) |
| SETUP-10 | Pending | ✅ Verified |
| SETUP-11 | Pending | ✅ Verified |
| SETUP-12 | Pending | ✅ Verified |
| SETUP-13 | Pending | ✅ Verified |
| SETUP-14 | Pending | ✅ Verified |
| SETUP-15 | Pending | ✅ Verified |
| SETUP-16 | Pending | ✅ Verified |
| SETUP-17 | Pending | ✅ Verified |

---

## Summary

**Overall**: ✅ Ready

**Spec-anchored check**: 17/17 ACs matched spec outcome; 0 spec-precision gaps
**Sensor**: 3/3 mutations killed
**Gate**: 8 passed, 0 failed; `npm run build` exit 0

**What works**: The Tauri 2 + Vite scaffold builds, the window title is configured as `My Photos`, `select_one` returns `1` from `:memory:` in `src-tauri/src/db/` without adding a top-level `.db`, the success health value serializes to `appName` and `sqliteOk`, the success and failure screen strings match the spec, the four motion packages are dependencies, `npm run build` fails when `motion` is removed from `dependencies`, and CI runs the build and `cargo test` on push and pull request without `tauri dev`.

**Issues found**: none

**Next steps**: none for this feature
