# F13 — Video Thumbnail Specification

## Problem Statement

A F12 só faz miniatura de foto; os vídeos ficariam como quadrados vazios na linha do tempo (F18) e sem pôster no player (F20). O banco também não sabe a duração nem o tamanho dos vídeos, que a UI mostra (RF-025: ícone + duração). O Rust puro não decodifica vídeo; o ROADMAP pede ffmpeg.

## Goals

- [x] Capa de cada vídeo nos mesmos três tamanhos da F12, pela mesma fila e cache
- [x] Duração, largura e altura (já em pé) do vídeo gravadas em `media_items`
- [x] Sem ffmpeg o app continua funcionando: fotos normais, vídeos contados como pulados

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Empacotar o ffmpeg no instalador | Vai com a feature de distribuição; binário de ~100 MB não entra no repositório (ver Open Questions) |
| Prévia animada / clipe curto | A "prévia de baixa resolução" é a capa `small`; prévia animada só se a F18/F20 pedir |
| Player | F20 |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Onde achar o ffmpeg | `ffmpeg(.exe)` e `ffprobe(.exe)` na pasta do executável do app (onde o Tauri põe um sidecar) e, se não, no `PATH`; os dois na mesma pasta | Funciona hoje para quem tem ffmpeg instalado e já serve o empacotamento | n |
| Momento da capa | 1 s depois do início, ou o meio de um clipe mais curto; se não vier quadro (duração errada), o primeiro quadro | Evita o quadro preto inicial | n |
| Tamanho | Mesmo maior lado da F12 (300/1280/2560), sem ampliar; JPEG qualidade ~3 do mjpeg | A grade mistura fotos e vídeos | n |
| Rotação | O ffmpeg já gira o quadro; largura/altura do `ffprobe` são trocadas quando há 90°/270° (matriz de exibição ou tag `rotate`) | Vídeo de celular em pé | n |
| Falhas | `ffprobe` sem stream de vídeo → `unsupported_format`; ffmpeg sai com erro ou passa de 60 s → `corrupt_media`; arquivo não existe → `unreadable`. ffmpeg que não executa **não** é gravado (`toolFailed`) e é tentado de novo | Instalar o ffmpeg depois tem que resolver sozinho | n |
| Re-indexação (F10) | Para vídeo, a F10 não apaga mais `width`/`height` | Quem lê o tamanho do vídeo é a F13 | n |

**Open question**: empacotar o ffmpeg (sidecar do Tauri, build LGPL "essentials") no instalador ou pedir para o usuário instalar? Hoje o app usa o que achar.

---

## Requirements

### VID-01 — Ler o vídeo
1. Duração em ms, largura e altura do primeiro stream de vídeo; 90°/270° trocam largura e altura; valor ausente ou zero fica vazio; sem stream de vídeo → nada.
2. Capa em 1 s ou no meio de clipe menor que 2 s.

### VID-02 — Achar o ffmpeg
1. Primeira pasta que tem `ffmpeg` **e** `ffprobe`.

### VID-03 — Gerar
1. Vídeo vira miniatura como foto (linha em `thumbnails`, arquivo em `.myphotos\thumbs\`) e grava `duration_ms`, `width`, `height`.
2. Falhas como na tabela; ffmpeg que não roda não é gravado e volta na próxima execução.

### VID-04 — Sem ffmpeg
1. Vídeos ficam fora da fila e contam em `videosSkipped`; fotos seguem normais.
2. Pedido da UI para vídeo passa na frente como o de foto.

### VID-05 — ffmpeg real
1. Com ffmpeg no `PATH`: `ffprobe` de um clipe 640×360 de 2 s; quadro de 300 px vira 300×169; 2560 não amplia; seek além do fim cai no primeiro quadro; arquivo que não é vídeo falha como mídia.

## Success Criteria

- [x] `thumbs::video_tests` 8/8 e `index::tests::idx07` no Linux (com ffmpeg 6.1 real)
- [ ] CI `windows-latest` verde
- [ ] `f12-thumbs` no `E:` com ffmpeg instalado: `videos`, `videosSkipped`, `ffmpegDir` (manual)
