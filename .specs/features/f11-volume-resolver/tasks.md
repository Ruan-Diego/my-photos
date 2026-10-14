# F11 Volume Resolver Tasks

**Design**: `.specs/features/f11-volume-resolver/design.md`
**Status**: T1–T2 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `volume/` + testes VOL-01..04 | `cargo test --manifest-path src-tauri/Cargo.toml volume::` |
| T2 | `start_run` revincula e registra; `db::open_located`; entradas padrão sem `E:\` fixo; mensagem nova | `cargo test --manifest-path src-tauri/Cargo.toml` |
