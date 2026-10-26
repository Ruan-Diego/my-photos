# F21 Details Panel Tasks

**Design**: `.specs/features/f21-details-panel/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `media_panel_details` e permissão | `cargo test --manifest-path src-tauri/Cargo.toml details acl01` |
| T2 | Painel, botão ⓘ na foto e no vídeo | `npm run build`; `node --experimental-strip-types --test src/viewer/panel.test.ts` |
| T3 | Comparar com `Viewer` | Capturas 1440×900 com o Tauri simulado |
| T4 | Abrir fotos reais do `E:` com o painel (com GPS, pessoas, sem JSON) | manual |
