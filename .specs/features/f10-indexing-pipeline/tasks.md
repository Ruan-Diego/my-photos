# F10 Indexing Pipeline Tasks

**Design**: `.specs/features/f10-indexing-pipeline/design.md`
**Status**: Done — T1–T3

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `scan_run` devolve ids; partes do F05 `pub(crate)` | `cargo test --manifest-path src-tauri/Cargo.toml scan::` |
| T2 | `index/` + `index/tests.rs` (IDX-01..06) + `bin/f10-index.rs` | `cargo test --manifest-path src-tauri/Cargo.toml index::` |
| T3 | `cargo run --release --bin f10-index` no `E:` (escreve só `E:\.myphotos\library.db`), duas vezes | ver `validation.md` |
