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

## `E:` (2026-10-14, `05b8126`, `cargo run --release --bin f10-index`)

| Execução | media | resolved | json | exif | assumed | filesystem | no_date | location | errors | tempo |
| -------- | ----- | -------- | ---- | ---- | ------- | ---------- | ------- | -------- | ------ | ----- |
| 1 (run 3) | 6 132 | 6 132 | 4 247 | 850 | 370 | 1 035 | 0 | 323 | 5 `orphan_json` | ~2m46s (sem o build) |
| 2 (run 4) | 6 132 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 5 `orphan_json` | 1,4 s |

O banco já tinha a varredura F05 (SCAN-07), então a 1ª execução foi `incremental` com 0 hash; as 6 132 mídias entraram por `taken_at_source` vazio. Depois: `taken_at_source` = 4 247 `takeout_json`, 850 `exif`, 1 035 `filesystem`; 3 pessoas; 323 locais. `library.db` 3,6 MB → 15,8 MB. Backup antes da execução em `%TEMP%\library-before-f10.db` na máquina. Os 5 `orphan_json` são os mesmos da F08 (fotos ausentes no export) e se repetem a cada execução.

**Veredito: PASS** (todas as mídias com data; 2ª execução sem trabalho).
