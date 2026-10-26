# F21 — Details Panel Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Viewer`**, painel à direita (escuro nos dois temas, regra 2) (AD-014, `.specs/DESIGN.md`).

## Problem Statement

O visualizador (F19) e o player (F20) mostram a mídia, mas não dizem de onde veio a data, onde foi, quem aparece nem onde o arquivo está. O usuário precisa conferir isso sem sair da foto (RF-026, RF-027, RF-028, US-03).

## Goals

- [x] O botão Detalhes (ⓘ) abre e fecha o painel à direita, na foto e no vídeo; a escolha fica lembrada
- [x] Data e hora com o selo da fonte (JSON do Takeout, EXIF ou data do arquivo)
- [x] Local: o ponto no mapa e as coordenadas
- [x] Pessoas marcadas no Google Fotos (do JSON)
- [x] Arquivo: nome, pasta no disco, dimensões, tamanho (e duração no vídeo)
- [x] "Técnico" recolhível: JSON pareado, pareamento, câmera, SHA-256

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Nome do lugar ("Lisboa, Portugal") e mapa de verdade | F24; o índice só tem as coordenadas e o app é offline |
| Pessoas e pasta como links | F23 (Pessoas) e F48 (Pastas); por enquanto texto |
| Álbum | F25; o índice ainda não guarda álbuns do Takeout |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Estado inicial | Fechado; depois lembra (`myphotos.viewer.details`) | A foto ocupa a tela até o usuário pedir | n |
| Mapa | A grade do canvas com o pino no centro, sem posição real | Sem mapa offline até a F24 | n |
| Data do arquivo | Selo em tom de alerta | É a fonte menos confiável (ex.: os 406 vídeos de 23/set) | n |
| Carregar | Comando `media_panel_details` só com o painel aberto | Não pesa ao passar fotos com o painel fechado | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| DET-01 | Data, fonte da data e coordenadas como no canvas |
| DET-02 | Linhas de Arquivo e Técnico com o que se sabe; vídeo mostra a duração |
| DET-03 | `media_panel_details` lê fonte da data, GPS, pessoas, pasta, JSON, câmera (EXIF) e SHA-256 |
| DET-04 | O painel abre e fecha na foto e no vídeo; a tela bate com `Viewer` em 1440×900 |
