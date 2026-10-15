# F15 App Shell Tasks

**Design**: `.specs/features/f15-app-shell/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `library_summary` (Rust) | `cargo test --manifest-path src-tauri/Cargo.toml commands::library` |
| T2 | Tokens, tema, fonte, barra lateral, rotas, Aparência | `npm run build`; `node --experimental-strip-types --test src/shell/shell.test.ts` |
| T3 | Comparar com as artboards, claro e escuro | Capturas 1440×900 (Chromium) |
| T4 | Abrir o app no Windows com o `E:` | manual |
