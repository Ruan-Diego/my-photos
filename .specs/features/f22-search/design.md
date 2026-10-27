# F22 — Search & Filters Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Search`**.

## Rust

- `search/mod.rs` (F14): `SearchQuery` ganha `people` (todas) e `folder` (prefixo do caminho, sem `LIKE` por causa de `_`); o filtro virou `where_clause(query, with_type)`. Novos `search_ids` (todos os ids na ordem da timeline + contagem de fotos e vídeos sem o filtro de tipo) e `search_options` (pessoas por quantidade, pastas com mídia).
- `commands/search.rs`: `search_media(query)` e `search_filters()`, com permissões `allow-search-media` e `allow-search-filters` (teste `acl01`).

## React

- `src/search/query.ts`: estado dos filtros → `SearchQuery`, datas, chips, título. Sem React.
- `src/search/SearchScreen.tsx`: campo (Esc limpa; vazio, volta para Fotos), chips, coluna de filtros, grade com 240 tiles por vez (mais ao rolar). Os itens vêm da linha do tempo já carregada (`timeline`), então o comando só manda ids.
- Seção `search` (`#/buscar`, grupo `hidden`: fora da lateral). Ctrl F em qualquer tela abre a busca.
- Visualizador e player recebem `base` (`fotos` ou `buscar`): setas, tira e voltar ficam dentro dos resultados.

## Fora do canvas

- Convite quando não há texto nem filtro; "Nenhum resultado"; "A busca falhou".
- Campo de data inválida marcado em tom de alerta.
- Local inativo com dica (F24).
