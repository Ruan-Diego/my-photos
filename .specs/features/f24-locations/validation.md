# F24 — Validation

| Requisito | Teste |
| --------- | ----- |
| PLC-01 | `places::tests::plc01_*` (Paulista → São Paulo, Times Square → New York City, centro de Osasco → Osasco, Atlântico → nada) |
| PLC-02 | `places::tests::plc02_*` |
| LOC-01 | `commands::locations::tests::loc01_*`; `commands::tests::acl01_*` |
| SRCH-11 | `search::tests::srch11_*` ("Porto, Portugal" não pega "Porto Alegre") |
| LOCUI-01..03 | `locations.test.ts`; `query.test.ts` (chip e filtro de lugar) |
| Tela | Capturas `locations-*`, `locations-zoom-*`, `locations-search-*` (claro e escuro) em `/mnt/project-files/f24-locations/`: alfinetes "2,5 mil", "229", "18"; aproximar separa Lisboa+Sintra de Porto; o lugar abre `#/buscar?local=…` com o chip "São Paulo" |

Pendente: conferir os nomes com as fotos reais do `E:`.
