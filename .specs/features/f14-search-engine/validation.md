# F14 — Validation

| Requisito | Teste |
| --------- | ----- |
| SRCH-01 | `srch01_the_box_splits_words_dates_and_types` |
| SRCH-02 | `srch02_words_match_names_folders_and_text_without_accents` |
| SRCH-03 | `srch03_people_follow_their_links_and_renames` |
| SRCH-04 | `srch04_place_and_note_are_searchable` |
| SRCH-05 | `srch05_dates_types_people_location_and_favorites_filter` |
| SRCH-06 | `srch06_newest_first_with_paging_and_a_total` |
| SRCH-07 | `srch07_moves_deletes_and_rebuild_keep_the_index_true` |
| SRCH-08 | `srch08_upgrading_an_indexed_library_fills_the_index` |
| SRCH-09 | `srch09_twenty_thousand_media_answer_quickly` (~25 ms em debug no Linux) |
| Migração v3 | `db::schema_tests::v1_has_exactly_the_spec_tables_and_indexes` (Windows); lista de tabelas e índices conferida com o sqlite3 do Python |

Local (Linux, 2026-10-15): `search::tests` 9/9; suíte 232 passam, as mesmas 42 falhas só-Windows de antes. `index::` sem regressão com os gatilhos ligados.
