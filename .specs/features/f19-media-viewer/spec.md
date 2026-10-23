# F19 — Media Viewer Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Viewer`** (só escuro; o visualizador é escuro nos dois temas, regra 2) (AD-014, `.specs/DESIGN.md`).

## Problem Statement

A linha do tempo (F18) mostra a biblioteca, mas não abre nenhuma foto. O usuário precisa ver a foto grande, no tamanho original, e passar para a anterior ou a próxima sem voltar à grade (RF-021, RF-022, RF-024, US-02, US-03).

## Goals

- [x] Clicar numa miniatura abre o visualizador por cima da grade; voltar (botão ou Esc) devolve a grade onde estava
- [x] Cabeçalho do canvas: voltar, data e hora ("Qua, 10 de agosto de 2022 · 14:37"), "3 de 14" no dia, Ajustar / 100%
- [x] Carregamento progressivo: a miniatura pequena na hora, o original por cima quando chega
- [x] 100%: tamanho original, arrastar para mover
- [x] Anterior / próxima pelas setas da tela e do teclado, na ordem da grade (com o filtro dela)
- [x] Tira de miniaturas vizinhas embaixo, com a atual destacada
- [x] Formatos que o app ainda não desenha (HEIC, RAW) e vídeos ficam na miniatura, com um aviso

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Painel Detalhes (data com a fonte, mapa, pessoas, arquivo, Técnico) | F21; o botão aparece, inativo |
| Favoritar | F26; o coração mostra o estado, inativo |
| Mais ações | Operações de arquivo (F48) |
| Player de vídeo | F20; o vídeo mostra a capa |
| Local no cabeçalho ("Lisboa, Portugal") | O índice não tem nome de lugar; F24 |
| HEIC e RAW em tamanho grande | O F12 não decodifica esses formatos; fica a miniatura quando houver |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Endereço | `#/fotos/<id>`; a grade continua montada por baixo | Voltar não perde a rolagem | n |
| Original | Protocolo `media` lê o arquivo do `E:` (JPEG, PNG, GIF, WebP); a WebView aplica a rotação do EXIF | Sem gerar miniatura grande em disco | n |
| Pré-carregar | Só o original da próxima, 400 ms depois | Um HD USB não aguenta ler vários de 5–20 MB de uma vez | n |
| Botões sem feature | Visíveis e inativos com dica, como Busca e Selecionar na F18 | Layout igual ao canvas | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| VIEW-01 | A fila do visualizador é a ordem da grade, com a posição no dia |
| VIEW-02 | Data e hora no fuso da foto, como no canvas |
| VIEW-03 | A tira mostra até 12 vizinhas, a atual o mais ao centro possível |
| VIEW-04 | `#/fotos/<id>` abre e `#/fotos` fecha; aviso para HEIC/RAW e vídeo |
| VIEW-05 | `media_details` diz se o original pode ser mostrado; o protocolo `media` entrega só fotos que a WebView desenha |
| VIEW-06 | Setas e Esc no teclado; 100% com arrastar; a tela bate com `Viewer` em 1440×900 |
