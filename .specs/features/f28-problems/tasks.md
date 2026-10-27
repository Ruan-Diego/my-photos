# F28 Error Report & Diagnostics Tasks

**Design**: `.specs/features/f28-problems/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `problems`, comandos e permissões | `cargo test --manifest-path src-tauri/Cargo.toml -- prb acl01` |
| T2 | Tela Problemas | `npm run build`; `node --experimental-strip-types --test src/problems/problems.test.ts` |
| T3 | Comparar com `Problems` | Capturas 1440×900, claro e escuro, com o Tauri simulado |
| T4 | No `E:`: conferir os problemas reais, exportar e verificar o banco | manual |
