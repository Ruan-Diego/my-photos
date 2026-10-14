# F11 — Validation

| Requisito | Teste |
| --------- | ----- |
| VOL-01 | `vol01_e_wins_when_it_has_the_library`, `vol01_the_library_is_found_on_another_letter`, `vol01_first_run_starts_on_e_and_nothing_else_is_guessed`, `vol01_two_libraries_without_e_is_ambiguous` |
| VOL-02 | `vol02_serial_and_drive_mask_are_formatted_like_windows`, `vol02_real_volume_has_a_serial` (Windows), `vol02_no_volume_off_windows` (Linux) |
| VOL-03 | `vol03_a_moved_disk_is_relinked_to_its_new_letter`, `vol03_another_disk_or_an_unknown_volume_is_never_relinked`, `vol03_a_root_already_used_by_another_library_is_not_taken_over` |
| VOL-04 | `vol04_the_run_records_serial_and_label` |

Local (Linux, 2026-10-14): `volume::tests` 10/10; suíte 198 passam, as mesmas 42 falhas só-Windows de antes. `scan::` e `index::` sem regressão. CI Windows é o gate.

Manual pendente: rodar `f10-index` no `E:` e ver `libraries.volume_id` preenchido; trocar a letra do HD e ver o app abrir a mesma biblioteca.
