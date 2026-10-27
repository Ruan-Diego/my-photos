# F26 — Favorites Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboards **`Favorites`** e **`Viewer`**.

## Rust

- `favorites/mod.rs`: `set_favorite(conn, id, favorite, now)` (`is_favorite`, `updated_at`); `import_takeout_favorites(conn)` marca os itens com id acima de `app_settings.takeout_favorites_imported_upto` cujo `media_metadata_raw.takeout_json` tem `favorited` verdadeiro, e guarda o maior id.
- `lib.rs` importa ao abrir; `index/mod.rs` importa ao fim de cada indexação.
- `commands/favorites.rs`: `favorite_set(id, favorite)`; permissão `allow-favorite-set`.

## React

- `App.tsx`: `useTimeline` também carrega em Favoritos e devolve `toggleFavorite`, que muda o bit `FAVORITE` em `TimelineData` (`withFavorite`), chama `favorite_set` e desfaz se falhar. `FavoriteContext` entrega a função ao visualizador; `null` com o disco desconectado.
- `Viewer`/`VideoPlayer`: o coração usa o contexto ("Favoritar" / "Remover dos favoritos").
- `src/favorites/favorites.ts`: subtítulo, `favoritesByYear` (com `keep`), `withFavorite`, textos vazios.
- `src/favorites/FavoritesScreen.tsx`: cabeçalho com o grupo Tudo/Fotos/Vídeos, anos e grade; `#/favoritos/<id>` abre o visualizador com `base="favoritos"`.

## Fora do canvas

- Estado vazio ("Nenhum favorito ainda") e falha com `page-empty`; selo de duração nos vídeos, como na grade.
