# F12 — Validation

| Requisito | Teste |
| --------- | ----- |
| THUMB-01 | `thumb01_each_size_fits_its_edge_and_keeps_the_aspect`, `thumb01_small_originals_are_never_upscaled`, `thumb01_exif_orientation_is_baked_in`, `thumb01_transparency_is_flattened_on_white` |
| THUMB-02 | `thumb02_unreadable_originals_say_why` |
| THUMB-03 | `thumb03_requests_first_then_newest_first`, `thumb03_a_request_moves_a_queued_job_up_once`, `thumb03_a_full_cache_drops_only_background_jobs` |
| THUMB-04 | `thumb04_pending_lists_photos_without_an_up_to_date_thumbnail_newest_first`, `thumb04_pending_ignores_other_libraries`, `thumb04_fingerprint_and_paths` |
| THUMB-05 | `thumb05_generate_library_makes_each_missing_thumbnail_once` |
| THUMB-06 | `thumb06_requests_are_served_and_marked_used` |
| THUMB-07 | `thumb07_eviction_drops_least_recently_used_large_and_medium_first`, `thumb07_cache_limit_comes_from_settings`, `thumb07_a_full_cache_stops_the_background_fill_and_keeps_what_it_made` |
| Migração v2 | `db::schema_tests::v1_has_exactly_the_spec_tables_and_indexes` (Windows) |

Local (Linux, 2026-10-14): `thumbs::tests` 16/16 (4 execuções seguidas); suíte 214 passam, as mesmas 42 falhas só-Windows de antes. JPEG cortado no meio dos dados decodifica (resto cinza) e vira miniatura; só cortado no cabeçalho vira `corrupt_media`.

Manual pendente: `cargo run --release --bin f12-thumbs` no `E:` (anotar tempo, `generated`, `unsupported`, `cacheBytes`) e rodar de novo (deve dar `queued: 0`).
