# F23 — People View Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`People`** (claro e escuro) (AD-014, `.specs/DESIGN.md`).

## Problem Statement

O índice já guarda quem o Google Fotos marcou em cada foto (JSONs do Takeout, F06/F09), mas o item "Pessoas" da lateral abria uma página vazia. O usuário quer ver as pessoas e chegar às fotos de cada uma (RF-032, RF-033, RF-038, US-06).

## Goals

- [x] `#/pessoas`: título "Pessoas", "28 pessoas marcadas no Google Fotos, lidas dos JSONs do Takeout" e o campo "Filtrar por nome"
- [x] Grade de círculos de 132 px com nome e "812 itens", quem tem mais itens primeiro
- [x] O filtro acha o nome sem acento, maiúscula ou ordem das palavras
- [x] Clicar numa pessoa abre a busca (F22) já filtrada por ela, como o link do canvas (`Search`)

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Recorte do rosto | O Takeout não traz onde o rosto está; o círculo mostra uma foto inteira da pessoa |
| Renomear, juntar ou esconder pessoas | Não está no canvas |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Foto do círculo (marcador de cor no canvas) | A foto mais recente com a pessoa; vídeo só se não houver foto; sem miniatura, as iniciais | Não há rosto no Takeout | n |
| Contagem | Itens visíveis (sem ocultos e sem arquivos que sumiram), como a timeline | Mesmo número da busca | n |
| Ordem | Mais itens primeiro; empate pelo nome | Canvas ordena por quantidade | n |
| Destino do clique | `#/buscar?pessoa=7`: a busca com o chip da pessoa | O canvas liga a pessoa à `Search` | n |
| Sem ninguém marcado | "Ninguém marcado no Google Fotos" | Fora do canvas | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| PPL-01 | `people_list` devolve pessoas com itens visíveis, contagem e capa |
| PPLUI-01 | O filtro acha nomes sem acento, maiúscula ou ordem |
| PPLUI-02 | Textos do canvas; a pessoa abre a busca filtrada |
