# F33 — Validation

| Requisito | Teste |
| --------- | ----- |
| A11Y-01 | `a11y.test.ts` A11Y-01 (30 pares, claro e escuro) |
| A11Y-02 | `a11y.test.ts` A11Y-02; Playwright: Enter numa miniatura põe o foco no visualizador, 40 Tabs e 10 Shift+Tabs não saem dele, Esc devolve o foco à miniatura |
| A11Y-03 | `a11y.test.ts` A11Y-03; Playwright: a miniatura focada diz "Foto de 13 de maio de 2024 às 12:00, favorita" |
| A11Y-04 | Playwright: o primeiro Tab foca "Pular para o conteúdo" e Enter foca o `main` (`skip-link-*.png`) |
| A11Y-05 | Playwright: "Nova pasta" foca o campo e Esc devolve o foco ao botão (`folder-dialog-*.png`); "Limpar miniaturas" vai para "Cancelar" e volta |
| A11Y-06 | axe 4.10 nas 12 telas (Fotos, visualizador, Pastas, Álbuns, Pessoas, Locais, Favoritos, Duplicatas, Problemas, Lixeira, Buscar, Configurações), claro e escuro: zero violações. Antes da F33, só os pinos de Locais no escuro (`locations-pins-*.png`) |

As capturas estão em `/mnt/project-files/f33-accessibility/`. Fica pendente no Windows testar com o Narrador: abrir uma foto, navegar com as setas e fechar.
