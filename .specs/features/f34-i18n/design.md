# F34 — Design

- `src/i18n/pt-BR.ts`: o objeto `ptBR` com as chaves (`as const`), de onde sai o tipo `Key`. Uma mensagem é um texto ou `{ one, other }`. `one` vale só para exatamente 1, que mantém o "0 itens" que o app sempre mostrou; uma língua com mais formas de plural amplia esse tipo. Listas (meses, dias) são textos separados por `|`.
- `src/i18n/i18n.ts`: `LOCALES`, `setLocale`, `currentLocale`, `t`, `list`, `formatCount`, `numberFormat`, `collator`, `dateText` e `placeholders` (usado no teste). A chave é tipada, então uma chave errada não compila.
- `MONTHS` e `WEEKDAYS` (timeline) e `SECTIONS` (barra lateral) são lidos do idioma quando o app abre; trocar de idioma em tempo real fica para quando existir um segundo idioma.
- `main.tsx` põe `lang` no `<html>`.
