# F34 — Validation

| Requisito | Teste |
| --------- | ----- |
| I18N-01 | `i18n.test.ts` I18N-01 |
| I18N-02 | `i18n.test.ts` I18N-02 |
| I18N-03 | `i18n.test.ts` I18N-03 (procura `Intl.*("xx")` e `toLocale*String("xx")` fora de `src/i18n`) |
| I18N-04 | `tsc` (chaves tipadas); testes de timeline, busca, pastas, álbuns, favoritos, pessoas, locais e indexação continuam passando com os textos vindos do módulo |
| I18N-05 | Playwright: texto visível, `aria-label`, `title` e `lang` das 12 telas na `main` (0196b3b) e neste branch: idênticos (982 linhas) |
