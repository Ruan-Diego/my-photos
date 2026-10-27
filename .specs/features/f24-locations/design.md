# F24 — Locations View Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Locations`**.

## Dados

- `scripts/places-data.mjs` gera `src-tauri/data/places.tsv` (46,5 mil cidades: nome, país, estado, lat, lon, da maior para a menor), `src-tauri/data/countries.tsv` (nome do país em português, `Intl.DisplayNames`) e `src/locations/world.ts` (contorno dos continentes como um path SVG 360 × 180). Os pacotes só são instalados para gerar (`npm i --no-save`); o app não depende deles.

## Rust

- `places/mod.rs`: `include_str!` dos TSV, índice em células de 1°, `nearest` (até 50 km; a maior cidade até 4 km além da mais próxima), `label` ("São Paulo, SP, Brasil"), `fill_place_labels` (preenche `place_label` nulo).
- F10 grava `place_label` ao inserir a localização; `lib.rs` chama `fill_place_labels` ao abrir. Os gatilhos da migração 3 levam o lugar para o índice de busca.
- `commands/locations.rs`: `locations_overview()` → `{located, unlocated, places: [{label, count, latitude, longitude (média), first, last, cover}]}`; permissão `allow-locations-overview`.
- `search/mod.rs`: `SearchQuery.place` vira `place : "<texto>"` no FTS5.

## React

- `src/locations/locations.ts`: textos, período, rótulo do alfinete, tamanho (36–64 px), projeção, agrupamento (`pins`: o maior fica e engole quem estiver a 44 px), `fitView`, `#/buscar?local=`.
- `src/locations/LocationsScreen.tsx`: mapa (SVG do contorno + botões dos alfinetes; arrastar com ponteiro, roda centrada no ponteiro, + e −; alfinete de vários lugares aproxima, de um lugar abre a busca) e lista dos 100 lugares com mais itens.
- `SearchScreen`: `?local=` abre com o lugar; o campo Local edita `Filters.place`; o chip mostra só a cidade.

## Fora do canvas

- Estado sem lugar ("Nenhuma foto com lugar") e falha, com `page-empty`.
- Créditos no canto do mapa; a grade do fundo não acompanha o arrasto.
- Abaixo de 960 px a lista vai para baixo do mapa.
