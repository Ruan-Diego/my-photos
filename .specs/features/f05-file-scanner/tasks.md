# F05 File Scanner Tasks

**Design**: `.specs/features/f05-file-scanner/design.md`
**Status**: Implemented — aguarda `cargo test` no Windows (CI) e confirmação das premissas

| Task | What | Where | Requirement | Status |
| ---- | ---- | ----- | ----------- | ------ |
| T1 | `discover` pula `.myphotos` na raiz e lista todas as pastas | `takeout/mod.rs`, `takeout/model.rs` | SC-01 | Done |
| T2 | `bench::hash` visível ao crate; `db::open_unchecked` para testes | `bench/`, `db/mod.rs` | — | Done |
| T3 | `scanner::scan` com plano (stat/hash) e escrita transacional | `scanner/mod.rs`, `scanner/time.rs` | SC-01..SC-08 | Done |
| T4 | Testes SC-01..SC-08 em pastas temporárias | `scanner/tests.rs`, `takeout/mod.rs` | SC-01..SC-08 | Done (17 testes passam no Linux) |

Gate: `cargo test --manifest-path src-tauri/Cargo.toml` (Windows, CI).
