# F05 File Scanner Validation

**Date**: 2026-10-06
**Spec**: `.specs/features/f05-file-scanner/spec.md`
**Verifier**: autor (auto-verificação). **Não** é o Verifier independente do processo: a skill `tlc-spec-driven` não está disponível na sessão na nuvem. Repetir com um Verifier independente numa sessão local.

**Verdict**: ⚠️ PASS provisório no harness Linux. Falta (1) o CI `windows-latest` verde no commit, que roda os testes só-Windows, e (2) a corrida manual SCAN-07 no `E:`.

---

## Como foi testado

Num harness em Linux (crate sem Tauri que copia `db`, `takeout`, `scan`, `bench/hash` e troca `symlink_dir`/`file_attributes` dos testes antigos por equivalentes Unix):

- `cargo test --lib scan::` → 27 passed, 0 failed, 1 ignored (`real_library_scan`)
- `cargo test --lib takeout::` → 52 passed, 1 failed: `unreadable_subdirectory_is_a_problem_and_keeps_the_neighbor`, que chama `icacls` (só existe no Windows). Falha esperada fora do Windows.

Testes `#[cfg(windows)]` que **só o CI roda**: `hidden_system_folder_is_skipped_below_the_root`, `unreadable_folder_keeps_the_status_of_its_media`. `linked_folder_is_not_followed` e `invalid_name_is_an_error_outside_media_items` têm variante Unix (rodaram) e Windows (junção `mklink /J`, surrogate UTF-16 solto). `names_that_differ_only_in_case_keep_the_first` só roda fora do Windows (o NTFS não deixa criar o par).

## Acceptance Criteria → testes (`src-tauri/src/scan/tests.rs`)

| AC | Teste | Resultado |
| -- | ----- | --------- |
| SCAN-01 AC1 mídias, tipos, pastas; `.txt` ignorado | `first_scan_records_media_and_folders_and_ignores_the_rest` | ✅ |
| SCAN-01 AC2 `.myphotos` / `.MyPhotos` na raiz | `app_folder_at_the_root_is_skipped_in_any_case` | ✅ |
| SCAN-01 AC3 symlink/junção não seguido | `linked_folder_is_not_followed` | ✅ Linux; junção no CI |
| SCAN-01 pasta oculto+sistema abaixo da raiz (edge) | `hidden_system_folder_is_skipped_below_the_root` | ⏳ CI |
| SCAN-02 AC4 campos e SHA-256 | `media_row_has_the_file_fields_and_the_sha256` | ✅ |
| SCAN-02 AC5 casamento JSON | `takeout_json_match_is_recorded` | ✅ |
| SCAN-02 AC6 biblioteca, run `full` `completed`, `last_indexed_at` | `first_scan_is_a_completed_full_run_of_one_library` | ✅ |
| SCAN-02 AC7 bytes intactos | `scan_leaves_every_library_byte_as_it_was` | ✅ |
| SCAN-03 AC1 sem mudança: 0 hashes, linhas idênticas, `incremental` | `second_scan_of_an_unchanged_disk_rehashes_nothing` | ✅ |
| SCAN-03 AC2 novo arquivo, ids mantidos | `new_file_is_inserted_and_known_ids_stay` | ✅ |
| SCAN-03 AC3 tamanho ou mtime mudou | `changed_content_is_rehashed_under_the_same_id` | ✅ |
| SCAN-03 AC4 só o JSON mudou | `json_change_updates_the_match_without_rehash` | ✅ |
| SCAN-04 AC1–AC2 removida → `missing_file` com favorito e nota; volta a `ok` | `removed_file_is_missing_with_its_favorite_and_note_and_comes_back` | ✅ |
| SCAN-04 AC3 movida mantém `id` | `file_moved_outside_the_app_keeps_its_id` | ✅ |
| SCAN-04 AC4 duas gêmeas ausentes não adotam | `two_missing_twins_do_not_adopt_a_new_copy` | ✅ |
| SCAN-04 AC5 pasta ilegível preserva status | `unreadable_folder_keeps_the_status_of_its_media` | ⏳ CI |
| SCAN-04 AC6 raiz some no meio | `root_lost_mid_scan_fails_the_run_and_marks_nothing_missing` | ✅ |
| SCAN-04 AC7 pasta sumida sai de `folders` | `vanished_folder_leaves_the_folder_table` | ✅ |
| SCAN-05 AC1, SCAN-06 AC4 arquivo ilegível | `unreadable_file_is_an_error_row_and_its_neighbors_are_written` | ✅ |
| SCAN-05 AC2 JSON inválido | `invalid_json_leaves_the_media_unmatched_with_an_error` | ✅ |
| SCAN-05 AC3 nome inválido | `invalid_name_is_an_error_outside_media_items` | ✅ Linux; UTF-16 no CI |
| SCAN-06 AC5 run `running` antiga → `failed` | `run_left_running_by_a_previous_process_becomes_failed` | ✅ |
| SCAN-06 AC6 raiz ausente | `missing_root_is_refused_without_writing` | ✅ |
| Edge: arquivo de 0 bytes | `empty_file_has_the_sha256_of_no_bytes` | ✅ |
| Edge: renomeado só na caixa | `case_only_rename_keeps_the_id`, `folder_case_rename_updates_the_paths_and_keeps_the_id` | ✅ |
| Edge: dois nomes que diferem só na caixa | `names_that_differ_only_in_case_keep_the_first` | ✅ (só fora do Windows) |
| SCAN-07 `E:` real | `real_library_scan` (`#[ignore]`) | ⏳ manual |

## Discrimination Sensor (auto, harness Linux)

Cada mutação aplicada à cópia do harness, `cargo test --lib scan::`, cópia restaurada. Nada aplicado ao repositório.

| # | Mutação | Resultado |
| - | ------- | --------- |
| M1 | "inalterado" ignora o mtime | ✅ Killed |
| M2 | gêmeas ausentes adotam a cópia nova | ✅ Killed |
| M3 | `UPDATE` compara o caminho sem `COLLATE BINARY` | ✅ Killed (depois de acrescentar o teste de pasta renomeada só na caixa; na 1ª rodada sobreviveu) |
| M4 | `.myphotos` não é pulada | ✅ Killed |
| M5 | run `running` antiga não vira `failed` | ✅ Killed |
| M6 | falha de hash com a raiz sumida vira erro de arquivo | ✅ Killed |
| M7 | ausente não vira `missing_file` | ✅ Killed |
| M8 | pasta sumida nunca sai de `folders` | ✅ Killed |
| M9 | sempre recalcula o hash | ✅ Killed |
| M10 | `UPDATE` sempre escreve | ✅ Killed |
| M11 | vídeo gravado como foto | ✅ Killed |
| M12 | `last_indexed_at` não gravado | ✅ Killed |
| M13 | upsert de pasta ignora mudança de caixa | ✅ Killed |

**Result**: 13/13 killed. Não cobertos pelo sensor: o caminho de pasta ilegível (só Windows).

## Pendências

- CI `windows-latest` verde no commit da F05.
- `real_library_scan` no `E:` (SCAN-07): imprimir contagens; segunda passada com `hashed=0`.
- Verifier independente numa sessão com a skill.
