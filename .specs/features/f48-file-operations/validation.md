# F48 — Validation

| Requisito | Teste |
| --------- | ----- |
| FOP-01 | `fileops::tests::fop01_*`; `folders.test.ts` FOLUI-01..02, FOLUI-04 |
| FOP-02 | `fileops::tests::fop02_*` |
| FOP-03 | `fileops::tests::fop03_*` (JSON junto, conflito de nome, busca acompanha) |
| FOP-04 | `fileops::tests::fop04_*` |
| FOP-05 | `fileops::tests::fop05_*` |
| FOP-06 | `fileops::tests::fop06_*` |
| FOP-07 | `fileops::tests::fop07_*` |
| FOP-08 | `fileops::tests::fop08_*` |
| FOP-09 | `commands/fileops.rs` `write` (checagem do `IndexingState`); botões desabilitados sem disco ou durante a indexação |
| FOP-10 | Playwright: o link vai para `#/pastas/<pasta da cópia>` |
| Seleção | `folders.test.ts` FOLUI-03; Playwright: clique, clique e Shift+clique dão 5 selecionadas |
| Comandos | `commands::tests::acl01_*` |
| Telas | `folders-*`, `folders-move-*`, `folders-trash-folder-*`, `folders-narrow-*`, `trash-*`, `trash-empty-*` (claro e escuro) em `/mnt/project-files/f48-file-operations/` |

Pendente no Windows: mover uma foto de teste entre duas pastas no `E:`, conferir que o JSON foi junto e que a data continua certa; mandar para a lixeira e restaurar.
