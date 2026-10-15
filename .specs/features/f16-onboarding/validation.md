# F16 — Validation

| Requisito | Teste |
| --------- | ----- |
| ONB-01 | `commands::onboarding::tests::onb01_parts_of_one_export_count_once`; `flow.test.ts` "ONB-01 …" |
| ONB-02 | `flow.test.ts` "ONB-02 …" |
| ONB-03 | `flow.test.ts` "ONB-03 …"; clique + evento de progresso simulados no Chromium |
| ONB-04 | `flow.test.ts` "ONB-04 …" |
| ONB-05 | Capturas 1440×900 claro/escuro comparadas com `Main`/`MainEscuro` |

Local (Linux, 2026-10-15): `onb01` 1/1; `flow.test.ts` + `shell.test.ts` 11/11; `npm run build` ok. Pendente: primeiro uso real no Windows com um banco novo.
