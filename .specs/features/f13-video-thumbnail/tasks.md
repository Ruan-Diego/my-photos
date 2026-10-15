# F13 Video Thumbnail Tasks

**Design**: `.specs/features/f13-video-thumbnail/design.md`
**Status**: T1–T3 feitos; aguarda CI

| Task | O quê | Gate |
| ---- | ----- | ---- |
| T1 | `thumbs/video.rs` + VID-01/02/05 | `cargo test --manifest-path src-tauri/Cargo.toml video_tests` |
| T2 | Fila/serviço da F12 com vídeos + VID-03/04 | `cargo test --manifest-path src-tauri/Cargo.toml thumbs::` |
| T3 | F10 mantém o tamanho do vídeo (`idx07`) | `cargo test --manifest-path src-tauri/Cargo.toml index::` |
| T4 | Rodar `f12-thumbs` no `E:` com ffmpeg instalado | manual |
