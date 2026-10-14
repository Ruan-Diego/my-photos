# F12 — Thumbnail Service Design

Migração **v2** (`db/migrations/0002_thumbnails.sql`): `thumbnails (media_id, size, width, height, bytes, source_fingerprint, created_at, last_accessed_at)`, PK `(media_id, size)`, índice `idx_thumbnails_lru`; `thumbnail_failures (media_id PK, source_fingerprint, reason, created_at)`. Ambas com `ON DELETE CASCADE`. O caminho não é gravado: sai de `thumb_path(root, id, size)`. Abrir o banco do `E:` faz a cópia `library.db.bak-v1` antes (F04).

Módulo `src-tauri/src/thumbs/` (dependência nova: `image` 0.25.10 só com jpeg, png, gif, webp, tiff, bmp — Rust puro):

- `render.rs` — `render(original, size, orientation) -> Result<Rendered, ThumbError>`: formato pelo conteúdo (`with_guessed_format`), `thumbnail()` quando passa do maior lado, `apply_orientation`, achata alfa em branco, `JpegEncoder` q82.
- `queue.rs` — `ThumbQueue`: `BinaryHeap` + mapa `(id, tamanho) → prioridade`. Subir a prioridade empilha de novo; a cópia antiga é pulada no `pop`. `drop_background()`.
- `service.rs` — `ThumbService::start(conn, root, workers, limit)`. Workers tiram da fila, chamam `render` e `write_thumb` (arquivo `.jpg.partial` + rename). Uma thread de escrita é dona da `Connection`: recebe resultados e pedidos por canal, grava em transação de até 64, despeja (`evict`) e, se o preenchimento encheu o cache, marca `cache_full` e limpa a fila de fundo. `request`, `enqueue`, `wait_idle(progress)`, `summary`, `shutdown() -> (Connection, ThumbSummary)`.
- `mod.rs` — `pending(conn, root, size)` (fotos da biblioteca cujo `root_path` é o disco atual, `taken_at DESC`, menos as em dia/falhadas), `job_for`, `thumbnail_for`, `record_done`, `record_failure`, `evict`, `cache_limit`, `cache_bytes`, `generate_library(conn, root, size, workers, progress)` e `generate_default_library(size, progress)` (F11 acha o disco).

Validação local: `src/bin/f12-thumbs.rs [small|medium|large]` imprime o `ThumbSummary`.
