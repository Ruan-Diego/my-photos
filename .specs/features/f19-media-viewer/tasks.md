# F19 Media Viewer Tasks

**Design**: `.specs/features/f19-media-viewer/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `media_details`, protocolo `media`, permissão | `cargo test --manifest-path src-tauri/Cargo.toml viewer acl01` |
| T2 | Fila, cabeçalho, Ajustar/100%, setas, teclado, tira, avisos | `npm run build`; `node --experimental-strip-types --test src/viewer/navigation.test.ts` |
| T3 | Comparar com `Viewer` | Capturas 1440×900 com o Tauri simulado |
| T4 | Abrir fotos reais do `E:` no Windows (JPEG grande, HEIC, vídeo) | manual |
