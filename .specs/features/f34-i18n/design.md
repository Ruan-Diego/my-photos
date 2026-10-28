# F34 — Design

- `src/i18n/pt-BR.ts`: o objeto `ptBR` com as chaves (`as const`), de onde sai o tipo `Key`. Uma mensagem é um texto ou `{ one, other }`. `one` vale só para exatamente 1, que mantém o "0 itens" que o app sempre mostrou; uma língua com mais formas de plural amplia esse tipo. Listas (meses, dias) são textos separados por `|`.
- `src/i18n/i18n.ts`: `LOCALES`, `setLocale`, `currentLocale`, `t`, `list`, `formatCount`, `numberFormat`, `collator`, `dateText` e `placeholders` (usado no teste). A chave é tipada, então uma chave errada não compila.
- `MONTHS` e `WEEKDAYS` (timeline) e `SECTIONS` (barra lateral) são lidos do idioma quando o app abre; trocar de idioma em tempo real fica para quando existir um segundo idioma.
- `main.tsx` põe `lang` no `<html>`.

## Segunda parte (I18N-06)

- O catálogo é plano, com chaves por tela (`timeline.*`, `viewer.*`, `folders.*`, `settings.*`…), e reaproveita as da primeira parte (`section.*`, `count.*`, `calendar.*`).
- `maybe(chave)` para chaves montadas em tempo de execução, como os códigos de falha do Rust (`onboarding.failures.<código>`).
- `rich(texto, { path: <code>…</code> })` em `src/i18n/rich.tsx`.
- O `t` formata todo parâmetro numérico, então anos vão como texto (`String(year)`), para não virar "2.022".
- Os particípios de Pastas e Lixeira ("movido", "restaurado") viram verbos (`"moved" | "trashed" | "restored"`) com plural no catálogo.
