# F13 — Video Thumbnail Design

Sem migração: usa `thumbnails`/`thumbnail_failures` da v2 e `media_items.duration_ms` da v1.

`src-tauri/src/thumbs/video.rs`:

- `VideoTool` (trait, `Send + Sync`): `probe(path) -> VideoProbe { duration_ms, width, height }` e `frame(path, at_ms, max_edge) -> JPEG`. Erro `VideoError::{Media(ThumbError), Tool}`.
- `Ffmpeg { ffmpeg, ffprobe }` implementa com `std::process::Command`: `ffprobe -show_entries stream=width,height:stream_tags=rotate:stream_side_data=rotation:format=duration -of json` → `parse_probe`; `ffmpeg -ss T -i f -frames:v 1 -vf scale='min(E,iw)':'min(E,ih)':force_original_aspect_ratio=decrease -f image2pipe -c:v mjpeg -`. Stdout lido numa thread, espera com limite de 60 s, `CREATE_NO_WINDOW` no Windows.
- `locate_ffmpeg()` / `locate_in(pastas)`, `cover_time`.

Integração na F12:

- `Job.video`; `pending(.., videos)` e `job_for(.., videos)` só incluem vídeos quando há ferramenta.
- `ThumbService::start` acha o ffmpeg; `start_with(.., Option<Arc<dyn VideoTool>>)` para testes. O worker chama `make_video` (probe → frame → `imagesize` → `write_thumb`); `Outcome::Done { video: Some(probe) }` faz o writer chamar `record_video` (COALESCE: valor que o ffprobe não leu mantém o antigo). `Outcome::ToolFailed` não grava nada.
- `generate_library` acha o ffmpeg e preenche `ffmpegDir`; `generate_with` conta `videosSkipped` quando não há.
- `ThumbSummary` ganha `videos`, `toolFailed`, `videosSkipped`, `ffmpegDir`.

F10 (`index::write_media`): `width`/`height` de vídeo ficam como estão na re-resolução.
