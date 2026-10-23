# F18 Timeline View Tasks

**Design**: `.specs/features/f18-timeline/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | Comando `timeline`, protocolo `thumb`, permissão | `cargo test --manifest-path src-tauri/Cargo.toml timeline acl01` |
| T2 | Agrupamento, linhas, virtualização, tela, filtros, tamanho, trilho | `npm run build`; `node --experimental-strip-types --test src/timeline/timeline.test.ts` |
| T3 | Comparar com `Timeline`/`TimelineEscuro` | Capturas 1440×900 com o Tauri simulado (2.400 mídias) |
| T4 | Rolar a biblioteca real do `E:` no Windows | manual |
