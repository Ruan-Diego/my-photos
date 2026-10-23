# F19 — Validation

| Requisito | Teste |
| --------- | ----- |
| VIEW-01 | `viewer.test.ts` "VIEW-01 …" |
| VIEW-02 | `viewer.test.ts` "VIEW-02 …" |
| VIEW-03 | `viewer.test.ts` "VIEW-03 …" |
| VIEW-04 | `viewer.test.ts` "VIEW-04 …"; capturas `viewer-heic.png`, `viewer-video.png` |
| VIEW-05 | `commands::viewer::tests::view01_*`, `view02_*`; `commands::tests::acl01_*` |
| VIEW-06 | Capturas `viewer.png`, `viewer-100.png`, `back-to-grid.png` em `/mnt/project-files/f19-media-viewer/` (seta → `#/fotos/4`, Esc → `#/fotos`) |

Local (Linux, 2026-10-23):
- Rust: 250 testes passaram (2 novos da F19); as 42 falhas são as mesmas de antes e só passam no Windows.
- Front: 28/28 testes em `node --test`; `npm run build` ok.

Pendente: abrir fotos reais do `E:` no Windows.
