# F17 — Design

## Rust

- `index::Progress` ganhou `bytesDone`, `bytesTotal`, `counts` e `recent`, e o `Stage` ganhou `hashing` e `thumbnails`.
  - `hashing` é enviado depois de cada arquivo lido (`scan::HashProgress`).
  - `resolving` é enviado depois de cada lote de 100, com os contadores lidos do banco (`library_counts`) e as três últimas mídias do lote.
- `index_and_thumbnail_default_library(progress, stop)` é a execução do app: primeiro o F10, depois `thumbs::generate_library` (pequenas) com os últimos contadores. O `f10-index` continua só indexando.
- Pausa:
  - `Sources.stop` é checado entre arquivos (hash) e entre lotes (metadados); a parada vira `ScanError::Cancelled` e a execução fica com `status = 'cancelled'`.
  - Nas miniaturas, `ThumbService::wait_idle_until` volta quando `stop()`.
- `Sources.clock` dá a hora do fim: `finished_at` e `last_indexed_at` deixam de ser a hora do início.
- Comandos novos, com permissão em `permissions/library.toml`:
  - `pause_indexing`;
  - `recent_thumbnails(limit)`, que devolve as pequenas mais novas como `data:image/jpeg;base64`.
- `library_folder` também conta exports um nível abaixo da raiz.

## Front

- `src/indexing/progress.ts`: título, etapas, percentual com pesos, tempo restante, contadores, linhas do console e cartões da lateral. Funções puras, testadas em `progress.test.ts`.
- `src/indexing/IndexingScreen.tsx`: a artboard `Indexing`. A barra anima a largura e as miniaturas entram com Motion; com reduzir movimento as duas ficam secas.
- `App.tsx`:
  - Depois de "Indexar", mostra a tela de progresso.
  - "Começar a navegar" ou o link de problemas vão para o app.
  - "Ver progresso" volta para a tela.
  - Um `finished` com `cancelled` deixa a tela pausada.
- Tokens novos: `--console-ink` e `--console-warn` (texto do console, iguais nos dois temas).
