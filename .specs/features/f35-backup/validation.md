# F35 — Validation

| Requisito | Teste |
| --------- | ----- |
| BAK-01 | `backup::tests::bak01_*`: 10 cópias viram 7 e a cópia abre como banco |
| BAK-02 | `backup::tests::bak02_*`: uma por dia |
| BAK-03, BAK-04 | `backup::tests::bak03_*`: num índice novo, a foto movida volta a ser favorita pelo SHA, o favorito do Takeout desmarcado é desmarcado, a nota volta e aparece na busca, e o que não existe mais é contado |
| BAK-05 | `backup::tests::bak04_*`: caminhos com `..`, `/`, `\`, outros nomes, JSON quebrado e app errado são recusados |
| Comandos | `commands::tests::acl01_*` |
| Tela | `settings/backup.test.ts` BKUI-01..03. Playwright: exportar acrescenta o arquivo à lista e importar chama `appdata_import` com o nome escolhido e mostra o resumo. Capturas `settings-backup-*` (claro, escuro, 900 px, sem rolagem horizontal) em `/mnt/project-files/f35-backup/`. O axe não acusa nada em Configurações |

Pendente no Windows: abrir o app e ver `E:\.myphotos\backups\library-*.db` (cerca de 185 MB cada, até 7 cópias, cerca de 1,3 GB no HD), exportar e importar.
