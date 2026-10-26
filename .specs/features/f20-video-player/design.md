# F20 — Video Player Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Video`**.

## Rust

- `commands/viewer.rs`: o protocolo `media` passa a servir vídeos (MP4/M4V → `video/mp4`, MOV → `video/quicktime`, WebM → `video/webm`). Lê o cabeçalho `Range` (`bytes=a-b`, `a-`, `-n`) e responde `206` com `Content-Range`, no máximo 4 MiB por resposta; sem `Range`, começa do byte 0; fora do arquivo, `416`. Fotos continuam `200` com o arquivo inteiro.
- `media_details` ganha `playable` e `sizeBytes`. Sem comando novo, sem permissão nova.

## React

- `src/viewer/media.ts`: o que foto e vídeo dividem (`useMediaInfo`, URLs dos protocolos, `fit`).
- `src/viewer/navigation.ts`: `fileSize`, `videoLine`, `clock`, `seek`, `videoProblem`.
- `src/viewer/Viewer.tsx`: vídeo vai para o `VideoPlayer`; foto continua no visualizador da F19.
- `src/viewer/VideoPlayer.tsx`: `<video preload="metadata">` com a capa de pôster; o quadro é medido no palco (até 1180 px de largura) na proporção da capa; controles próprios (a barra é um `input range` invisível por cima do desenho do canvas); tela cheia no quadro.

## Fora do canvas

- Aviso no topo do quadro quando o vídeo não toca, no estilo da etiqueta da F19.
- Favoritar e detalhes inativos com dica, como na F19.
