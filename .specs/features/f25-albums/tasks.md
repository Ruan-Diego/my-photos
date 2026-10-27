# F25 Albums Tasks

**Design**: `.specs/features/f25-albums/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `albums`, reconstrução no F10, `albums_list`, filtro `album` | `cargo test --manifest-path src-tauri/Cargo.toml -- alb srch acl01` |
| T2 | Tela Álbuns, chip de álbum na busca | `npm run build`; `node --experimental-strip-types --test src/albums/albums.test.ts src/search/query.test.ts` |
| T3 | Comparar com `Albums` | Capturas 1440×900, claro e escuro, com o Tauri simulado |
| T4 | No `E:`: "Procurar arquivos novos" e conferir os álbuns | manual |
