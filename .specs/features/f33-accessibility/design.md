# F33 — Design

- `src/a11y/focus.ts`: o seletor `FOCUSABLE` e `wrapTarget`, que diz para onde o Tab vai na borda de um modal.
- `src/a11y/useDialogFocus.ts`: lembra o elemento focado na primeira renderização, antes do `autoFocus` de um campo mover o foco. Ao abrir, foca o `[autofocus]`, o primeiro focável ou o próprio diálogo; ao fechar, devolve o foco. `trap` prende o Tab e `onEscape` fecha.
- `src/a11y/Dialog.tsx`: o diálogo não modal de Pastas, como `div` ou `form`.
- `src/a11y/contrast.ts`: luminância e contraste do WCAG, leitura dos blocos claro e escuro do `tokens.css`, e `PAIRS` com os pares que as telas usam.
- `tileLabel` em `timeline.ts`: o rótulo das miniaturas usado na linha do tempo, na busca e em Favoritos.
- `.skip-link` fica fora da tela até receber o foco do teclado. Ele é um botão, não um link `#`, porque o roteamento usa o hash.
- Token `--on-accent` para texto sobre `--accent-fill`.
