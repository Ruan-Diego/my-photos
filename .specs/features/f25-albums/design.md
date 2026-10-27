# F25 — Albums Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Albums`**.

## Rust

- `albums/mod.rs`: `album_folder(path)` devolve a pasta lógica (export sem o sufixo da parte, via `takeout::logical_dir`) logo abaixo de "Google Fotos"/"Google Photos"; `rebuild_takeout_albums(conn, library_id, &DiscoveryReport, now)` faz upsert em `albums` (`source = 'takeout_folder'`, `folder_relative_path` = pasta lógica), apaga os que sumiram e refaz `media_albums` casando `relative_path` sem diferenciar maiúsculas; `album_cards` devolve `{id, name, count, first, last, covers}`, mais novos primeiro.
- `index/mod.rs` chama a reconstrução depois de gravar os problemas, na mesma indexação.
- `commands/albums.rs`: `albums_list()`; permissão `allow-albums-list`.
- `search/mod.rs`: `SearchQuery.album` filtra por `media_albums`; `SearchOptions.albums` lista `{id, name}` para o chip.

## React

- `src/albums/albums.ts`: ordenações (recentes, nome, tamanho), `albumMeta`, `#/buscar?album=`, ordem guardada em `localStorage`.
- `src/albums/AlbumsScreen.tsx`: seletor "Ordenar", grade de álbuns com a pilha de 3 capas; vazio explica "Procurar arquivos novos".
- `SearchScreen`: `?album=` abre com o álbum; o chip mostra o nome e remove o filtro.

## Fora do canvas

- Estados vazio e de falha com `page-empty`.
