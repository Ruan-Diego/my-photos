# F14 Search Engine Tasks

**Design**: `.specs/features/f14-search-engine/design.md`
**Status**: T1–T2 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | Migração v3 (FTS5 + gatilhos); testes de schema | `cargo test --manifest-path src-tauri/Cargo.toml db::` (Windows) |
| T2 | `search/` + SRCH-01..09 | `cargo test --manifest-path src-tauri/Cargo.toml search::` |
| T3 | Conferir no `E:` | manual |
