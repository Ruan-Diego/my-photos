# F16 — Validation

| Requisito | Teste |
| --------- | ----- |
| ONB-01 | `commands::onboarding::tests::onb01_parts_of_one_export_count_once`; `flow.test.ts` "ONB-01 …" |
| ONB-02 | `flow.test.ts` "ONB-02 …" |
| ONB-03 | Desde a F17: `progress.test.ts` "PROG-06 …"; clique + evento de progresso simulados no Chromium |
| ONB-04 | `flow.test.ts` "ONB-04 …" |
| ONB-05 | Capturas 1440×900 claro/escuro comparadas com `Main`/`MainEscuro` |

Local (Linux, 2026-10-15): `onb01` 1/1; `flow.test.ts` + `shell.test.ts` 11/11; `npm run build` ok. Pendente: primeiro uso real no Windows com um banco novo.

Primeiro uso real no Windows (2026-10-15, `E:`): passou depois do PR #14 (permissões dos comandos); 6.130 mídias, 5 JSONs sem foto. Achados levados para a F17: hash sem contagem na lateral, `finished_at` igual ao início, exports em `E:\Ruan\` não contados.
