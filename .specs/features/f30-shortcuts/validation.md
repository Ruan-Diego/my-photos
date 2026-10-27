# F30 — Validation

| Requisito | Teste |
| --------- | ----- |
| KEY-01 | `shortcuts.test.ts` KEY-01; Playwright: tamanho 3 → 5 com duas roladas para cima, → 4 com uma para baixo; roda sem Ctrl rola 400 px e mantém o 4 |
| KEY-02 | `shortcuts.test.ts` KEY-02; Playwright: F abre a tela cheia no `.viewer`, F de novo fecha, → vai para a próxima |
| KEY-03 | `shortcuts.test.ts` KEY-03; Playwright: Esc fora da tela cheia volta a `#/fotos` |
| KEY-04 | `shortcuts.test.ts` KEY-04 |
| Tela | Capturas `timeline-ctrl-wheel-light.png` e `viewer-fullscreen-light.png` em `/mnt/project-files/f30-shortcuts/` |

Pendente no Windows: conferir que Ctrl + roda não dá zoom na WebView2.
