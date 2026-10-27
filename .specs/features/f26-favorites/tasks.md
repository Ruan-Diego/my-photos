# F26 Favorites Tasks

**Design**: `.specs/features/f26-favorites/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `favorites`, importação do Takeout, `favorite_set` | `cargo test --manifest-path src-tauri/Cargo.toml -- fav acl01` |
| T2 | Coração no visualizador, tela Favoritos | `npm run build`; `node --experimental-strip-types --test src/favorites/favorites.test.ts` |
| T3 | Comparar com `Favorites` | Capturas 1440×900, claro e escuro, com o Tauri simulado |
| T4 | No `E:`: abrir o app e conferir as favoritas do Google Fotos; marcar e desmarcar | manual |
