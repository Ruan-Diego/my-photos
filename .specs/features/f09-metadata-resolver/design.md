# F09 — Metadata Resolver Design

Módulo `src-tauri/src/resolve/` (sem I/O):

```rust
pub fn resolve(
    json: Option<&MediaMetadata>,      // F06, já casado pela F08
    exif: Option<&EmbeddedMetadata>,   // F07; None para vídeo ou ilegível
    fs: FsTimes<'_>,                   // fs_created_at / fs_modified_at do F05
    fallback_offset_minutes: i32,      // AD-012, padrão -180
) -> ResolvedMetadata
```

`ResolvedMetadata`: `taken_at` (UTC ISO, mesmo formato de F05/F06/F07, ordena como tempo), `taken_at_source` (`DateSource::as_column`), `taken_at_timezone`, `taken_at_offset_assumed`, `location: Option<Location { latitude, longitude, altitude, source }>`, `width`, `height`, `orientation`, `title`, `description`.

Hora local → UTC: `local_to_utc` lê os 23 caracteres do F07, valida mês/dia/hora, usa `time::days_from_civil` e `time::iso_from_unix_millis` (faixa 1800–9999).

Sistema de arquivos: os textos ISO do F05 têm o mesmo formato, então o menor em ordem de texto é o mais antigo.

A F10 chama `resolve` por mídia e grava o resultado; `taken_at_offset_assumed` não tem coluna — fica para relatório/diagnóstico da F10.
