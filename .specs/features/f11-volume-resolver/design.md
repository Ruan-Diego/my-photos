# F11 — Volume Resolver Design

Módulo `src-tauri/src/volume/`:

- `locate_library() -> Result<String, LocateError>` / `locate_with(&dyn Volumes)`: regra do VOL-01. `Volumes` = `roots()` (`GetLogicalDrives` → `roots_from_mask`) + `has_library_db(root)`.
- `volume_info(root) -> Option<VolumeInfo { serial, label }>`: `GetVolumeInformationW` na raiz da letra (`bench::volume::volume_root_of`), `format_serial`.
- `relink(conn, root, volume)` e `record_volume(conn, library_id, volume)`: chamados por `scan::start_run` na mesma transação que cria/acha a biblioteca e abre o `indexing_runs`. Valem para F05 e F10.

Entradas de produção:

- `db::open_located() -> (root, Connection)`: `locate_library` + `root\.myphotos\library.db`. `open_library()` (startup do app) usa isso. `LocateError` vira `DbError::VolumeMissing` ("disco da biblioteca não encontrado").
- `scan_default_library` e `index_default_library` usam o `root` devolvido em vez de `E:\` fixo. `LIBRARY_DB_PATH` saiu; `LIBRARY_ROOT` fica só como documentação do AD-007.
