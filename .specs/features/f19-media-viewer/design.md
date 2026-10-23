# F19 — Media Viewer Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Viewer`**.

## Rust

- `commands/viewer.rs`: comando `media_details(id)` (nome, caminho, extensão, vídeo, `viewable`, largura, altura) e protocolo `media` (`http://media.localhost/<id>`), assíncrono, que lê o original fora da trava do banco. Só JPEG, PNG, GIF e WebP; o resto é 404. Permissão `allow-media-details` (teste `acl01`).

## React

- `src/viewer/viewer.ts`: `order` (a grade em fila, com "N de M" no dia), `dateLine`, `stripWindow`, `openedId`/`viewerHash`, `fallbackNote`.
- `src/viewer/Viewer.tsx`: sobreposição fixa sobre a tela; cabeçalho, palco, setas e tira. No modo Ajustar, a foto ocupa o maior retângulo na proporção da miniatura (já girada), com a miniatura por baixo e o original por cima (opacidade 180 ms; sem animação com reduzir movimento). No modo 100% o palco rola e se arrasta com o ponteiro.
- `TimelineScreen.tsx`: as miniaturas viram links `#/fotos/<id>`; com o endereço de uma mídia, o `Viewer` abre por cima.
- Tokens próprios (`--viewer-*` em `tokens.css`), iguais nos dois temas: o canvas só tem o visualizador escuro.

## Fora do canvas

- Aviso na foto para HEIC/RAW e vídeo, no estilo da etiqueta do canvas.
- Botões de favoritar, detalhes e mais ações inativos (opacidade 0,45, dica no `title`).
- 100% inativo quando o original não pode ser mostrado.
