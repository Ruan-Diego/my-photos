# F17 Indexing Progress Tasks

**Design**: `.specs/features/f17-indexing-progress/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | Progresso do hash, contadores, console, pausa, hora do fim, miniaturas na execução, exports um nível abaixo | `cargo test --manifest-path src-tauri/Cargo.toml` (index, onboarding, thumbs) |
| T2 | Tela de progresso, cartões da lateral, pausar/continuar | `npm run build`; `node --experimental-strip-types --test src/indexing/progress.test.ts` |
| T3 | Comparar com `Indexing`/`IndexingEscuro` | Capturas 1440×900 com o Tauri simulado |
| T4 | Indexação real no Windows com a tela nova | manual |
