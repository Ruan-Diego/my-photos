# F13 — Validation

| Requisito | Teste |
| --------- | ----- |
| VID-01 | `vid01_probe_reads_size_duration_and_rotation`, `vid01_cover_is_one_second_in_or_mid_clip` |
| VID-02 | `vid02_ffmpeg_needs_both_binaries_in_the_same_folder` |
| VID-03 | `vid03_videos_get_a_cover_and_their_duration_and_size`, `vid03_a_missing_ffmpeg_is_retried_not_recorded` |
| VID-04 | `vid04_without_ffmpeg_videos_are_counted_and_photos_still_done`, `vid04_a_video_request_jumps_the_queue_like_a_photo`; `thumb05` conta o vídeo em `videosSkipped` |
| VID-05 | `vid05_real_ffmpeg_probes_and_takes_a_frame` (roda só onde há ffmpeg no `PATH`) |
| F10 | `index::tests::idx07_re_resolving_a_video_keeps_the_size_f13_read` |

Local (Linux, ffmpeg 6.1.1, 2026-10-15): `thumbs::` 24/24, `vid05` rodou com o ffmpeg real; suíte 223 passam, as mesmas 42 falhas só-Windows de antes. No CI do Windows o `vid05` provavelmente sai cedo (sem ffmpeg no runner); o resto não depende dele.

Manual pendente: instalar ffmpeg (ou copiar `ffmpeg.exe`/`ffprobe.exe` para a pasta do app) e rodar `f12-thumbs` no `E:`.
