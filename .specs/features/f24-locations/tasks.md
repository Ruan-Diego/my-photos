# F24 Locations View Tasks

**Design**: `.specs/features/f24-locations/design.md`
**Status**: T1–T4 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | Dados de cidades e mapa, gerados por script | `node scripts/places-data.mjs` |
| T2 | `places`, `place_label` no F10 e ao abrir, `locations_overview`, filtro `place` | `cargo test --manifest-path src-tauri/Cargo.toml -- plc loc01 srch acl01` |
| T3 | Tela Locais, filtro Local na busca | `npm run build`; `node --experimental-strip-types --test src/locations/locations.test.ts src/search/query.test.ts` |
| T4 | Comparar com `Locations` | Capturas 1440×900, claro e escuro, com o Tauri simulado |
| T5 | Abrir o app no `E:` (os lugares são preenchidos ao abrir) e conferir nomes | manual |
