# F29 Duplicate Detection Tasks

**Design**: `.specs/features/f29-duplicates/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `duplicates`, hash na indexação, `duplicates_list` | `cargo test --manifest-path src-tauri/Cargo.toml -- dup acl01` |
| T2 | Tela Duplicatas | `npm run build`; `node --experimental-strip-types --test src/duplicates/duplicates.test.ts` |
| T3 | Comparar com `Duplicates` | Capturas 1440×900, claro e escuro, com o Tauri simulado |
| T4 | No `E:`: "Procurar arquivos novos" e conferir os grupos | manual |
