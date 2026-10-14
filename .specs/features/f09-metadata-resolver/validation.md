# F09 — Validation

| Requisito | Teste |
| --------- | ----- |
| RES-01.1 | `res01_json_date_wins_over_exif_and_filesystem` |
| RES-01.2 | `res01_json_without_a_date_falls_to_exif_never_to_its_creation_time` |
| RES-02.1 | `res01_json_without_a_date_falls_to_exif_never_to_its_creation_time` |
| RES-02.2 | `res02_exif_without_zone_uses_the_fallback_offset`, `res02_fallback_offset_crosses_midnight_and_year_end` |
| RES-02.3 | `res02_unreadable_local_time_falls_to_the_filesystem` |
| RES-03 | `res03_filesystem_uses_the_earlier_time` |
| RES-04 | `res04_json_location_wins_and_keeps_its_source`, `res04_exif_location_fills_in_when_json_has_none` |
| RES-05 | `res05_dimensions_come_from_the_file_and_text_from_the_json` |

Local (Linux, 2026-10-13): `cargo test --lib resolve` 10/10; `cargo clippy --all-targets` sem avisos no módulo. CI Windows é o gate.
