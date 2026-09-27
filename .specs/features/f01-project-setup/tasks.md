# F01 Project Setup Tasks

## Execution Protocol (MANDATORY -- do not skip)

Implement these tasks with the `tlc-spec-driven` skill: **activate it by name and follow its Execute flow and Critical Rules.** Do not search for skill files by filesystem path. The skill is the source of truth for the full flow (per-task cycle, sub-agent delegation, adequacy review, Verifier, discrimination sensor).

**If the skill cannot be activated, STOP and tell the user — do not proceed without it.**

---

**Design**: `.specs/features/f01-project-setup/design.md`
**Status**: Done

---

## Test Coverage Matrix

> Generated from codebase, project guidelines, and spec — confirm before Execute. Guidelines found: none — strong defaults applied, bounded by `.specs/features/f01-project-setup/spec.md`. No `AGENTS.md`, `CONTRIBUTING.md`, or test-runner config exists. The spec forbids launching the desktop window in CI (SETUP-17), so there is no e2e layer.

| Code Layer | Required Test Type | Coverage Expectation | Location Pattern | Run Command |
| ---------- | ------------------ | -------------------- | ---------------- | ----------- |
| Domain (`db`, `health_from_probe`) | unit | All branches; 1:1 to SETUP-03, SETUP-04, SETUP-05 and the double-call edge | `src-tauri/src/**/*.rs` tests in the same module | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Health view | unit | Happy path, invoke error, `sqliteOk !== true` | `src/health/view.test.ts` | `node --experimental-strip-types --test src/health/view.test.ts` |
| Tauri command registration, window title, capability | none | Build plus the domain tests that feed the command | `src-tauri/tauri.conf.json`, `src-tauri/capabilities/` | `npm run build` and `cargo test --manifest-path src-tauri/Cargo.toml` |
| Motion imports | none | Build fails if a package is missing (SETUP-08) | `src/motion/libraries.ts` | `npm run build` |
| README, CI workflow, folder layout | none | Build gate. The files are the artifact | `README.md`, `.github/workflows/ci.yml` | `npm run build` |

## Parallelism Assessment

> Generated from codebase — confirm before Execute.

| Test Type | Parallel-Safe? | Isolation Model | Evidence |
| --------- | -------------- | --------------- | -------- |
| unit (Rust) | Yes | Each test opens its own `:memory:` connection. No shared file | Design: `Connection::open_in_memory` |
| unit (Node) | Yes | Pure function, no shared store | `src/health/view.ts` has no I/O |
| none | Yes | No tests | Config and build only |

## Gate Check Commands

> Generated from codebase — confirm before Execute. Commands are the ones the spec names, plus the Node test that covers the view. There is no lint script in the repo yet; the build runs `tsc`.

| Gate Level | When to Use | Command |
| ---------- | ----------- | ------- |
| Quick | After Rust unit tests only | `cargo test --manifest-path src-tauri/Cargo.toml` |
| Full | After the view unit tests | `node --experimental-strip-types --test src/health/view.test.ts` and `cargo test --manifest-path src-tauri/Cargo.toml` |
| Build | After phase completion or config-only tasks | `npm run build` and `cargo test --manifest-path src-tauri/Cargo.toml` and `node --experimental-strip-types --test src/health/view.test.ts` |

---

## Execution Plan

Three phases, so execution stays in this session. No phase sub-agents.

### Phase 1: Foundation (Sequential)

```
T1
```

### Phase 2: Core (Sequential)

```
T1 → T2 → T3
```

### Phase 3: Shell and CI (Sequential)

```
T3 → T4 → T5 → T6
```

---

## Task Breakdown

### T1: Scaffold the Tauri React app

**What**: Create the official `react-ts` Tauri 2 app in this repository, with identifier `com.myphotos.desktop`, product name `My Photos`, and window title `My Photos`.
**Where**: repository root, `src/`, `src-tauri/`
**Depends on**: None
**Reuses**: `create-tauri-app` template `react-ts`
**Requirement**: SETUP-01, SETUP-10, SETUP-12

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] `src/` and `src-tauri/` exist, and `src-tauri/Cargo.toml` depends on Tauri 2.x
- [ ] `src-tauri/tauri.conf.json` has product name `My Photos`, identifier `com.myphotos.desktop`, and the main window title `My Photos`
- [ ] `npm run build` exits 0
- [ ] No path in the scaffold points at `E:\`

**Tests**: none
**Gate**: build

**Commit**: `build(setup): scaffold tauri react typescript app`

---

### T2: Open in-memory SQLite

**What**: Add `select_one()` and Rust tests that prove `SELECT 1` returns `1`, a second call returns `1`, and no `.db` file appears.
**Where**: `src-tauri/src/db.rs`, `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`
**Depends on**: T1
**Reuses**: template `lib.rs` module declarations
**Requirement**: SETUP-03, SETUP-05, SETUP-16

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] `rusqlite` 0.40.2 with feature `bundled` is a dependency
- [ ] `select_one()` uses `Connection::open_in_memory` and `SELECT 1`
- [ ] `cargo test` asserts the returned value is `1` on two successive calls
- [ ] `cargo test` asserts the crate directory contains no new file ending in `.db`
- [ ] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] Test count: 2 tests pass

**Tests**: unit
**Gate**: quick

**Commit**: `feat(setup): open in-memory sqlite for the health probe`

---

### T3: Map the health payload and expose the command

**What**: Add `health_from_probe` and the Tauri command `app_health`, with permission `allow-app-health` on the main window.
**Where**: `src-tauri/src/commands/health.rs`, `src-tauri/src/lib.rs`, `src-tauri/permissions/app-health.toml`, `src-tauri/capabilities/default.json`
**Depends on**: T2
**Reuses**: `db::select_one`
**Requirement**: SETUP-03, SETUP-04

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] `Ok(1)` becomes `app_name == "My Photos"` and `sqlite_ok == true`
- [ ] `Ok` other than `1` returns `Err` whose message is `SELECT 1 retornou {n}`
- [ ] `Err("disco")` returns `Err("disco")`
- [ ] The command is registered and the main capability allows `allow-app-health`
- [ ] Gate: `cargo test --manifest-path src-tauri/Cargo.toml` exits 0
- [ ] Test count: 5 tests pass (2 from T2 plus 3 probe cases)

**Tests**: unit
**Gate**: quick

**Commit**: `feat(setup): add app_health command payload mapping`

---

### T4: Render the proof shell

**What**: Render the spec strings from `healthView`, import the four motion libraries without animating, and invoke `app_health` when the window loads.
**Where**: `src/health/view.ts`, `src/health/view.test.ts`, `src/motion/libraries.ts`, `src/App.tsx`, `package.json`
**Depends on**: T3
**Reuses**: `src/main.tsx` from the template
**Requirement**: SETUP-02, SETUP-04, SETUP-06, SETUP-07, SETUP-08, SETUP-09

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] Success render is exactly `My Photos está pronto.`, `SQLite: ok`, and `three, gsap, animejs, motion`
- [ ] An invoke error `boom` renders exactly `Falha: boom` and none of the success strings
- [ ] `sqliteOk: false` renders exactly `Falha: sqliteOk inválido` and none of the success strings
- [ ] `package.json` dependencies include `three`, `gsap`, `animejs`, and `motion`
- [ ] `libraries.ts` imports those four packages and does not call an animation or create a WebGL renderer
- [ ] Gate: full gate exits 0
- [ ] Test count: 3 Node tests and 5 Rust tests pass

**Tests**: unit
**Gate**: full

**Commit**: `feat(setup): show the stack proof shell`

---

### T5: Document how to run the app

**What**: Replace the root README with Windows prerequisites and `npm run tauri dev`.
**Where**: `README.md`
**Depends on**: T4
**Reuses**: none
**Requirement**: SETUP-13

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] README names Windows 10/11, Rust stable, Node.js Active LTS, and WebView2
- [ ] README documents the command `npm run tauri dev`
- [ ] README does not tell the reader to scan `E:\`
- [ ] Gate: build gate exits 0

**Tests**: none
**Gate**: build

**Commit**: `docs(setup): document windows prerequisites and dev command`

---

### T6: Run the checks in GitHub Actions

**What**: Add a workflow that runs the build, the Node view tests, and `cargo test` on push and pull request, without launching the window.
**Where**: `.github/workflows/ci.yml`
**Depends on**: T4, T5
**Reuses**: the gate commands
**Requirement**: SETUP-14, SETUP-15, SETUP-17

**Tools**:

- MCP: NONE
- Skill: NONE

**Done when**:

- [ ] `.github/workflows/ci.yml` triggers on `push` and `pull_request`
- [ ] The workflow runs `npm run build`, `node --experimental-strip-types --test src/health/view.test.ts`, and `cargo test --manifest-path src-tauri/Cargo.toml`
- [ ] The workflow file does not contain `tauri dev`
- [ ] Gate: build gate exits 0
- [ ] Test count stays 5 Rust tests and 3 Node tests

**Tests**: none
**Gate**: build

**Commit**: `ci(setup): check build and tests on push and pull request`

---

## Parallel Execution Map

```
Phase 1 (Sequential):
  T1

Phase 2 (Sequential):
  T1 complete, then:
    T2 → T3

Phase 3 (Sequential):
  T3 complete, then:
    T4 → T5 → T6
```

No `[P]` tasks. T5 depends on the scaffold from T1 and is ordered after T4 so the README describes the shell that already exists.

---

## Task Granularity Check

| Task | Scope | Status |
| ---- | ----- | ------ |
| T1: Scaffold the Tauri React app | 1 CLI operation plus window identity in the generated config | ✅ Granular |
| T2: Open in-memory SQLite | 1 function plus its tests | ✅ Granular |
| T3: Map the health payload and expose the command | 1 command and its permission | ✅ Granular |
| T4: Render the proof shell | 1 screen: view, library link, and App | ✅ Granular — one UI surface |
| T5: Document how to run the app | 1 file | ✅ Granular |
| T6: Run the checks in GitHub Actions | 1 workflow file | ✅ Granular |

## Diagram-Definition Cross-Check

| Task | Depends On (task body) | Diagram Shows | Status |
| ---- | ---------------------- | ------------- | ------ |
| T1 | None | Phase 1 start | ✅ Match |
| T2 | T1 | T1 → T2 | ✅ Match |
| T3 | T2 | T2 → T3 | ✅ Match |
| T4 | T3 | T3 → T4 | ✅ Match |
| T5 | T4 | T4 → T5 | ✅ Match |
| T6 | T4, T5 | T4 → T5 → T6 | ✅ Match |

## Test Co-location Validation

| Task | Code Layer Created/Modified | Matrix Requires | Task Says | Status |
| ---- | --------------------------- | --------------- | --------- | ------ |
| T1: Scaffold | Tauri config / entity | none | none | ✅ OK |
| T2: SQLite | Domain | unit | unit | ✅ OK |
| T3: Health command | Domain | unit | unit | ✅ OK |
| T4: Proof shell | Health view (unit) and motion imports (none) | unit | unit | ✅ OK |
| T5: README | README | none | none | ✅ OK |
| T6: CI | CI workflow | none | none | ✅ OK |
