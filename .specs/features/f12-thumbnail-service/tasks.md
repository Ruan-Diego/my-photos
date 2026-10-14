# F12 Thumbnail Service Tasks

**Design**: `.specs/features/f12-thumbnail-service/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | Migração v2 `thumbnails` + `thumbnail_failures`; testes de schema atualizados | `cargo test --manifest-path src-tauri/Cargo.toml db::` (Windows) |
| T2 | `thumbs/` render, fila, pendentes, LRU, serviço + testes THUMB-01..07 | `cargo test --manifest-path src-tauri/Cargo.toml thumbs::` |
| T3 | `bin/f12-thumbs` | `cargo build --manifest-path src-tauri/Cargo.toml` |
| T4 | Rodar no `E:` e registrar os números | manual |
