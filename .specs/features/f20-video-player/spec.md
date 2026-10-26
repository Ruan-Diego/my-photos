# F20 — Video Player Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Video`** (sempre escuro, como o `Viewer`) (AD-014, `.specs/DESIGN.md`).

## Problem Statement

A F19 abre fotos, mas um vídeo mostra só a capa. O usuário precisa assistir aos vídeos da biblioteca dentro do app, sem copiar nada para o PC (RF-023, RF-025).

## Goals

- [x] Clicar num vídeo da grade abre o player (`#/fotos/<id>`), por cima da grade; voltar ou Esc devolve a grade
- [x] Cabeçalho do canvas: voltar, data e hora, "VID_….mp4 · 1920 × 1080 · 84 MB"
- [x] Quadro na proporção do vídeo, capa como pôster, botão grande de reproduzir
- [x] Controles: barra com o trecho carregado e o tocado, play/pause, "0:42 / 2:18", volume, tela cheia
- [x] Teclado: Espaço, ← → (5 s), F, Esc; rodapé com as teclas e "Tocando o original direto do disco"
- [x] O original toca direto do `E:`, em trechos (HTTP Range), sem copiar o arquivo
- [x] Vídeo que a WebView não toca (MKV, AVI, 3GP, codec ausente) mostra a capa com um aviso

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Painel Detalhes | F21; o botão aparece, inativo |
| Favoritar | F26; o coração mostra o estado, inativo |
| Anterior / próxima no player | O artboard `Video` não tem setas nem tira; ← → avançam no vídeo |
| Tocar MKV/AVI/3GP ou HEVC sem a extensão do Windows | Exigiria transcodificar (ffmpeg, decisão pendente) |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Entrega | Protocolo `media` responde `206` com no máximo 4 MiB por pedido | A WebView pede trechos; um HD USB não precisa ler o vídeo inteiro | n |
| Formatos | MP4, M4V, MOV e WebM vão para o `<video>`; o resto fica na capa | É o que a WebView2 toca sem extensão | n |
| Volume | Guardado no `localStorage` (70 por padrão) | Preferência do aparelho, não da biblioteca | n |
| Proporção | A da capa (já girada); sem ela, 16:9 | Vídeos de celular em pé saem em pé | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| VIDEO-01 | Cabeçalho e relógio como no canvas ("84 MB", "0:42 / 2:18") |
| VIDEO-02 | ← → andam 5 s dentro do vídeo, sem passar do início nem do fim |
| VIDEO-03 | MKV e afins dizem que ainda não tocam; erro de reprodução fala do codec |
| VIDEO-04 | `media_details` diz se o vídeo toca e o tamanho; o protocolo `media` responde trechos (`206`, `416`) |
| VIDEO-05 | A tela bate com `Video` em 1440×900; tocar, avançar e Esc funcionam |
