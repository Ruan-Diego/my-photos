# F16 Onboarding Tasks

**Design**: `.specs/features/f16-onboarding/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `library_folder`, `start_indexing`, contagem de exports | `cargo test --manifest-path src-tauri/Cargo.toml onboarding` |
| T2 | Tela, fluxo e cartões da lateral | `npm run build`; `node --experimental-strip-types --test src/onboarding/flow.test.ts` |
| T3 | Comparar com `Main`/`MainEscuro` | Capturas 1440×900 com o Tauri simulado |
| T4 | Primeiro uso real no Windows (banco novo) | manual |
