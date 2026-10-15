# F14 — Search Engine Design

Migração **v3** (`db/migrations/0003_search.sql`): tabela virtual `media_search` (FTS5, colunas `path, title, description, people, place, note`, `rowid = media_items.id`), preenchida a partir das tabelas atuais e mantida por 13 gatilhos; índice `idx_media_people_person` para o filtro por pessoa. O banco do `E:` faz a cópia `library.db.bak-v2` antes (F04).

Módulo `src-tauri/src/search/`:

- `parse_text(text) -> ParsedText { fts, year, month, media_type }`.
- `search(conn, &SearchQuery) -> SearchPage { total, hits: [SearchHit { id, relative_path, media_type, taken_at }] }`: monta um `WHERE` com parâmetros numerados (`m.id IN (SELECT rowid FROM media_search WHERE media_search MATCH ?)`, intervalos de `taken_at`, `EXISTS` para local), conta e pagina. `SearchQuery` é `Deserialize` em camelCase para o comando Tauri da F22.
- `rebuild(conn)`: apaga e refaz o índice e roda `optimize`.

F10 não muda: os gatilhos cuidam do que ela grava.
