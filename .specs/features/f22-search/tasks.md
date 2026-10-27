# F22 Search & Filters Tasks

**Design**: `.specs/features/f22-search/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `search_ids`, `search_options`, filtros de pessoas e pasta, comandos e permissões | `cargo test --manifest-path src-tauri/Cargo.toml srch acl01` |
| T2 | Tela de busca, Ctrl F, visualizador dentro dos resultados | `npm run build`; `node --experimental-strip-types --test src/search/query.test.ts` |
| T3 | Comparar com `Search` | Capturas 1440×900, claro e escuro, com o Tauri simulado |
| T4 | Buscar na biblioteca real do `E:` (nome, pessoa, pasta, ano) | manual |
