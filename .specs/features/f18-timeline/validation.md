# F18 — Validation

| Requisito | Teste |
| --------- | ----- |
| TL-01 | `timeline.test.ts` "TL-01 …"; `commands::timeline::tests::tl01_*` |
| TL-02 | `timeline.test.ts` "TL-02 …"; captura `timeline-videos-light.png`, `timeline-empty-light.png` |
| TL-03 | `timeline.test.ts` "TL-03 …" |
| TL-04 | `timeline.test.ts` "TL-04 …"; captura `timeline-small-light.png` |
| TL-05 | `timeline.test.ts` "TL-05 …"; captura `timeline-2021-light.png` (salto pelo trilho; 26–45 miniaturas no DOM com 2.400 mídias) |
| TL-06 | `commands::timeline::tests::tl01_*`, `tl02_*`, `tl03_*` |
| TL-07 | `commands::timeline::tests::tl04_serves_a_current_small_thumbnail_and_nothing_else`; `commands::tests::acl01_*` |
| TL-08 | Capturas 1440×900 em `/mnt/project-files/f18-timeline/` comparadas com `Timeline`/`TimelineEscuro` |

Local (Linux, 2026-10-20):
- Rust: 248 testes passaram (4 novos da F18); as 42 falhas são as mesmas de antes e só passam no Windows.
- Front: 24/24 testes em `node --test`; `npm run build` ok.

Pendente: rolar a biblioteca real do `E:` no Windows (6.130 mídias).
