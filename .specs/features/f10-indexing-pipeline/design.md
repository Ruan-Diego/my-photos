# F10 — Indexing Pipeline Design

Módulo `src-tauri/src/index/`; binário `src/bin/f10-index.rs` (mesma forma do `f05-scan`).

```rust
pub fn index_library(conn, root, progress: &mut dyn FnMut(Progress)) -> Result<IndexSummary, ScanError>
pub fn index_default_library(progress) -> Result<IndexSummary, &'static str>   // E:\ (AD-007/010/011)
pub(crate) fn index_with(conn, root, &Sources { fs, discover, read_exif }, now, progress)
```

Fluxo (uma linha em `indexing_runs`):

1. `scan::start_run` → `scan::scan_run` (F05, agora devolve os ids lidos) .
2. `takeout::discover(root)` (F02/F06/F08). Problemas de sidecar → `indexing_errors` (fora de `.myphotos`).
3. Pendentes = `media_items` não `missing_file` que: foram lidos no passo 1, OU têm `taken_at_source` vazio, OU têm `json_relative_path`/`json_match_confidence` diferente da descoberta.
4. Em lotes de 100 por transação: EXIF (só `photo`) → `resolve::resolve` (F09) → `UPDATE media_items`, `media_locations` (apaga e insere), `media_metadata_raw` (apaga e insere), `media_people` (apaga e insere; `people` com `INSERT OR IGNORE`).
5. `scan::finish_run` + `files_indexed = resolvidas`.

Disco que some no meio (leitura falha e a raiz não é mais pasta) → `RootMissing`, como no F05; o lote em andamento é descartado e a execução fica `failed`.

Mudança no F05: `check_root`, `start_run`, `finish_run`, `insert_error`, `scan_run` viraram `pub(crate)`; `scan_run` devolve os ids lidos. Comportamento do `scan_with` igual.
