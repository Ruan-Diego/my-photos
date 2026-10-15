# F15 — Validation

| Requisito | Teste |
| --------- | ----- |
| SHELL-01 | `commands::library::tests::shell01_*` (3); `shell.test.ts` "SHELL-01 …" (2) |
| SHELL-02 | `shell.test.ts` "SHELL-02 sidebar order and groups follow the canvas" |
| SHELL-03 | `shell.test.ts` "SHELL-03 …" (2) |
| SHELL-04 | `shell.test.ts` "SHELL-04 …" (2); troca ao vivo e pré-pintura conferidas no Chromium |
| SHELL-05 | Revisão: `grep -n "#[0-9a-f]\{3,6\}" src --include=*.tsx --include=App.css` vazio |

Local (Linux, 2026-10-15): `commands::library` 3/3; `shell.test.ts` 7/7; `npm run build` ok. Capturas 1440×900 de Fotos e Configurações em claro e escuro conferem com `Sidebar`/`Settings` (medidas, cores, ícones, item ativo invertido). Pendente: abrir no Windows com o `E:` para ver o cartão conectado.
