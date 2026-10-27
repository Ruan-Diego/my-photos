# F31 — Validation

| Requisito | Teste |
| --------- | ----- |
| SET-01 | `settings::tests::set01_*`; `settings.test.ts` SETUI-01..02 |
| SET-02 | `settings::tests::set02_*`; `settings.test.ts` SETUI-03; Playwright: as chaves chamam `settings_set_flag` |
| SET-03 | `settings::tests::set03_*`; Playwright: escolher 2 GB chama `settings_set_cache_limit` com 2147483648 |
| SET-04 | `settings::tests::set04_*`; Playwright: confirmar mostra "Apagar 1,2 GB de miniaturas" e depois o aviso |
| SET-05 | `settings.test.ts` SETUI-04; Playwright: "Sempre reduzidas" põe `data-motion="reduced"` |
| Comandos | `commands::tests::acl01_*` |
| Tela | `settings-*`, `settings-confirm-*`, `settings-narrow-*` (claro e escuro) em `/mnt/project-files/f31-settings/`, sem rolagem horizontal em 900 px |

Pendente no Windows: ligar a busca ao conectar e reabrir o app; limpar e refazer as miniaturas só se quiser (cerca de 40 min no `E:`).
