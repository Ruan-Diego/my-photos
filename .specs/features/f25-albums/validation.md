# F25 — Validation

| Requisito | Teste |
| --------- | ----- |
| ALB-01 | `albums::tests::alb01_*` |
| ALB-02 | `albums::tests::alb02_*` (título do metadata.json, partes do export, oculto ligado mas fora da contagem, id mantido, álbum vazio fora) |
| SRCH-12 | `search::tests::srch12_*`; `commands::tests::acl01_*` |
| ALBUI-01..02 | `albums.test.ts`; `query.test.ts` (chip de álbum) |
| Tela | Capturas `albums-*` e `albums-search-*` (claro e escuro) em `/mnt/project-files/f25-albums/` |

Pendente: reindexar o `E:` e conferir os álbuns reais.
