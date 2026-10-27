# F26 — Validation

| Requisito | Teste |
| --------- | ----- |
| FAV-01 | `favorites::tests::fav01_*`; `commands::tests::acl01_*` |
| FAV-02 | `favorites::tests::fav02_*` (JSON inválido, `false`, sem campo; desmarcado não volta; item novo entra) |
| FAVUI-01..04 | `favorites.test.ts` |
| Tela | Capturas `favorites-*`, `favorites-viewer-*`, `favorites-mark-*` (claro e escuro) em `/mnt/project-files/f26-favorites/`: 219 favoritos por ano; desmarcar no visualizador chama `favorite_set(1, false)` e o subtítulo vai a 218; marcar na linha do tempo acende o coração |

Pendente: conferir as favoritas reais do `E:`.
