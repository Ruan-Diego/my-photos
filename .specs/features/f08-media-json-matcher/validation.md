# F08 — Validation

## MATCH-01 (testes)

| AC | Teste |
| -- | ----- |
| 1 | `matching::tests::td04_supplemental_metadata_is_exact_and_its_truncation_heuristic`, `takeout::tests::td04_supplemental_metadata_is_exact` |
| 2 | `matching::tests::untruncated_long_supplemental_metadata_still_matches` |
| 3 | `td04_supplemental_metadata_is_exact_and_its_truncation_heuristic` (2ª parte) |
| 4 | `takeout::tests::sidecar_in_another_part_of_the_same_export_matches_with_physical_paths` |
| 5 | `td06_duplicate_marker_sidecar_is_heuristic`, testes de `-edited` (sem mudança) |

Local (Linux, 2026-10-13): `cargo test --lib takeout` 71 passam; 1 falha só fora do Windows (`unreadable_subdirectory…`, usa `icacls`). CI Windows é o gate.

## MATCH-02 (`E:`)

| Data | Código | google_photos | matched | % | exact | heuristic |
| ---- | ------ | ------------- | ------- | - | ----- | --------- |
| 2026-10-13 | `main` antes da F08 | 4 247 | 4 247 | 100,00 | 0 | 4 247 |
| — | após a F08 | pendente | | | | |
