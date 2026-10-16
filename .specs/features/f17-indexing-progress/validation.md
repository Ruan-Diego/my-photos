# F17 — Validation

| Requisito | Teste |
| --------- | ----- |
| PROG-01 | `progress.test.ts` "PROG-01 …" |
| PROG-02 | `progress.test.ts` "PROG-02 …" |
| PROG-03 | `progress.test.ts` "PROG-03 …"; `index::tests::prog03_hashing_reports_each_file_with_its_bytes` |
| PROG-04 | `progress.test.ts` "PROG-04 …" |
| PROG-05 | `progress.test.ts` "PROG-05 …"; `index::tests::prog06_counters_and_log_lines_follow_the_library`; `commands::onboarding::tests::prog01_*`, `prog02_*` |
| PROG-06 | `progress.test.ts` "PROG-06 …"; captura `sidebar-light.png` |
| PROG-07 | `index::tests::prog05_pause_keeps_what_was_done_and_the_next_run_picks_up` |
| PROG-08 | `index::tests::prog04_the_run_ends_at_the_time_it_ends` |
| PROG-09 | Capturas 1440×900 em `/mnt/project-files/f17-indexing-progress/` comparadas com `Indexing`/`IndexingEscuro` |
| ONB-01 (exports) | `commands::onboarding::tests::onb02_exports_one_folder_down_count_too` |

Local (Linux, 2026-10-15):
- Rust: 244 testes passaram; as 42 falhas são as mesmas de antes e só passam no Windows.
- Front: 19/19 testes em `node --test`; `npm run build` ok.

Pendente: indexação real no Windows com a tela nova.
