# F10 — Validation

| Requisito | Teste |
| --------- | ----- |
| IDX-01 | `idx01_first_run_writes_every_source_to_the_database` |
| IDX-02.1 | `idx02_second_run_without_changes_reads_nothing` |
| IDX-02.2 | `idx02_a_sidecar_that_appears_later_re_resolves_only_its_media` |
| IDX-02.3 | `idx02_a_changed_file_is_read_again` |
| IDX-03 | `idx03_a_rows_people_location_and_raw_json_are_replaced_not_appended` |
| IDX-04 | `idx04_sidecar_problems_are_recorded_but_not_inside_the_app_folder` |
| IDX-05 | `idx05_corrupt_or_unreadable_exif_never_stops_the_run` |
| IDX-06 | `idx06_missing_media_is_left_alone` |

Local (Linux, 2026-10-14): `index::` 8/8, `scan::` e `resolve::` sem regressão; as 42 falhas restantes da suíte são as mesmas só-Windows de antes (`bench::`, `db::`, `icacls`). CI Windows é o gate.

## `E:` (pendente)

| Execução | media | resolved | json | exif | assumed | filesystem | no_date | location | errors | tempo |
| -------- | ----- | -------- | ---- | ---- | ------- | ---------- | ------- | -------- | ------ | ----- |
