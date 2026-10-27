# F31 — Design

## Rust

- `settings/mod.rs`: `flag`/`set_flag` (só `video_covers` e `scan_on_connect`), `set_cache_limit` (só `CACHE_LIMITS`), `settings` (visão da tela), `clear_thumbnails`.
- `thumbs::video_tool(connection)`: o ffmpeg só quando `video_covers` não está desligada. É usado na geração da indexação, no `ThumbService::start` e no `library_summary` (para o "faltam N miniaturas"). A tela Problemas continua mostrando se o ffmpeg existe.
- Comandos em `commands/settings.rs`: `settings_overview`, `settings_set_flag`, `settings_set_cache_limit` e `thumbnails_clear`, que recusa enquanto `IndexingState::is_running()`. Cada um tem permissão em `permissions/library.toml` e na capability (teste `acl01`).

## Front

- `src/settings/settings.ts`: textos (`volumeLine`, `databaseLine`, `cacheUsage`, `cacheOptions`, `videoCoversNote`) e as animações (`MotionChoice`, `reducedMotion`, `loadMotion`, `saveMotion`).
- `SettingsScreen.tsx` substitui o `Appearance` provisório da F15.
- `App`: `useMotion` grava `data-motion` no `<html>` e alimenta o `MotionConfig`. Um efeito busca as configurações quando o disco conecta e, com `scanOnConnect`, chama `startIndexing(true)` (silencioso).
