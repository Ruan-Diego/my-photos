# F34 — Validation

| Requisito | Teste |
| --------- | ----- |
| I18N-01 | `i18n.test.ts` I18N-01 |
| I18N-02 | `i18n.test.ts` I18N-02 |
| I18N-03 | `i18n.test.ts` I18N-03 (procura `Intl.*("xx")` e `toLocale*String("xx")` fora de `src/i18n`) |
| I18N-04 | `tsc` (chaves tipadas); testes de timeline, busca, pastas, álbuns, favoritos, pessoas, locais e indexação continuam passando com os textos vindos do módulo |
| I18N-06 | `i18n.test.ts` I18N-06 (nenhum texto de tela fora do catálogo em `src/`) e I18N-07 (o detector acha texto JSX, `aria-label`, template e "1 nota"; ignora comando, SVG, comparação e CSS). Playwright: texto visível, `aria-label`, `title`, `placeholder` e `aria-valuetext` das 12 telas, do visualizador, das três abas de Problemas e do onboarding, na `main` (a6afe54) e neste branch: idênticos (692 + 15 linhas, horários mascarados) |
| I18N-05 | Playwright: texto visível, `aria-label`, `title` e `lang` das 12 telas na `main` (0196b3b) e neste branch: idênticos (982 linhas) |
