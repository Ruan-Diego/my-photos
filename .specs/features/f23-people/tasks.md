# F23 People View Tasks

**Design**: `.specs/features/f23-people/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `people_list`, permissão e capability | `cargo test --manifest-path src-tauri/Cargo.toml -- ppl01 acl01` |
| T2 | Tela Pessoas e a busca aberta pela pessoa | `npm run build`; `node --experimental-strip-types --test src/people/people.test.ts` |
| T3 | Comparar com `People` | Capturas 1440×900, claro e escuro, com o Tauri simulado |
| T4 | Ver as pessoas reais do `E:` | manual |
