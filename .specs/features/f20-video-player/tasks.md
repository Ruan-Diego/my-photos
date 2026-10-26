# F20 Video Player Tasks

**Design**: `.specs/features/f20-video-player/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | Protocolo `media` com Range para vídeo; `playable` e `sizeBytes` | `cargo test --manifest-path src-tauri/Cargo.toml viewer` |
| T2 | Player: cabeçalho, quadro, controles, teclado, avisos | `npm run build`; `node --experimental-strip-types --test src/viewer/navigation.test.ts` |
| T3 | Comparar com `Video` | Capturas 1440×900 com o Tauri simulado e um WebM de teste |
| T4 | Tocar vídeos reais do `E:` no Windows (MP4 de celular, MOV, um longo) | manual |
