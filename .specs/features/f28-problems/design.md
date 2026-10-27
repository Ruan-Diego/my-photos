# F28 — Error Report & Diagnostics Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Problems`**.

## Rust

- `problems/mod.rs`: `problems` junta `indexing_errors` da última execução terminada (sem `corrupt_media` e `unreadable`), mídias com `indexing_status = 'error'`, EXIF corrompido (qualquer execução), `thumbnail_failures`, `taken_at_source = 'filesystem'` e `json_match_confidence = 'heuristic'`; uma linha por (caminho, motivo), pendentes primeiro. `report` conta as fontes de data, compara a parte do export da mídia e do JSON (`index::export_part`) e mede a última execução `completed`. `diagnostics`: caminho do banco, raiz, `thumbs::cache_bytes` e `cache_limit`, ffmpeg. `integrity` roda `PRAGMA integrity_check`. `report_csv`.
- `commands/problems.rs`: `problems_overview`, `database_check`, `problems_export` (CSV em `<pasta do banco>\relatorios\problemas-<data>.csv`); permissões `allow-problems-overview`, `allow-database-check`, `allow-problems-export`.

## React

- `src/problems/problems.ts`: abas, filtros com contagem, `shortPath`/`fullPath`, `durationText`, cartões do relatório, linhas do diagnóstico.
- `src/problems/ProblemsScreen.tsx`: cabeçalho com "Exportar relatório" e a linha "Relatório salvo em …", abas, tabela (até 500 linhas, caminho completo na dica), cartões, diagnóstico com o botão de integridade.

## Fora do canvas

- Estados vazio e de falha com `page-empty`; aviso quando a tabela corta em 500.
