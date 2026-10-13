# F08 Media↔JSON Matcher Tasks

**Design**: `.specs/features/f08-media-json-matcher/design.md`
**Status**: Done — T1–T3 (survey no `E:` 100%, 4 247 `exact`)

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | Supplemental inteiro → `Exact`; truncado → `Heuristic`; testes td04, nome longo e multi-parte atualizados | `cargo test --manifest-path src-tauri/Cargo.toml takeout::` |
| T2 | `real_takeout_survey` mede só Google Fotos e exige >98%; `google_photos_scope_is_by_folder_name` | idem |
| T3 | Rodar no `E:` (só leitura) e registrar em `validation.md` | `TAKEOUT_ROOT=E:\ cargo test --manifest-path src-tauri/Cargo.toml real_takeout_survey -- --ignored --nocapture` |
