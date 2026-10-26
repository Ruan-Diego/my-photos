# F22 — Search & Filters Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Search`** (claro e escuro) (AD-014, `.specs/DESIGN.md`).

## Problem Statement

O índice já busca (F14), mas não há tela: a caixa "Buscar" da timeline estava inativa. O usuário precisa achar fotos por nome, pasta, pessoa, data e tipo, combinando filtros, e abrir o resultado (RF-034–RF-042, US-06).

## Goals

- [x] A caixa da timeline e Ctrl F abrem a busca (`#/buscar`), com o cursor no campo
- [x] Texto livre do F14: nome de arquivo, pasta, título, descrição, pessoas; ano, mês e "foto"/"vídeo" viram filtro
- [x] Filtros combináveis: data De/Até, Fotos/Vídeos (com contagem), Só favoritos, Pessoas, Pasta
- [x] Chips dos filtros ativos com X e "Limpar filtros"
- [x] "48 resultados", grade quadrada, mais recentes primeiro; vídeo com duração
- [x] O resultado abre o visualizador/player, que anda só pelos resultados ("1 de 9"); Esc volta à busca como estava

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Filtro por nome de lugar ("Lisboa, Portugal") | F24; o campo Local aparece inativo |
| Item "Buscar" na lateral | O canvas não tem; a busca abre pela timeline |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Sem texto e sem filtro | Convite a buscar, não a biblioteca inteira | A timeline já mostra tudo | n |
| Várias pessoas | Todas na mesma foto (E) | É o que "pessoa A com pessoa B" quer dizer | n |
| Datas | dd/mm/aaaa digitado; "Até" inclui o dia; inválida fica marcada e não filtra | Igual ao canvas (campo de texto) | n |
| Dia da data | Comparado em UTC (`taken_at`) | Diferença só perto da meia-noite | n |
| Pastas | As que têm mídia, sem o prefixo `Takeout\` no nome | Como no canvas ("Google Fotos\Viagem Europa 2022") | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| SRCHUI-01 | Filtros viram a consulta do F14 (datas, tipo, pessoas, pasta, favoritos) |
| SRCHUI-02 | Chips e título como no canvas; o X tira só aquele filtro |
| SRCHUI-03 | `search_media` devolve os ids e as contagens por tipo; `search_filters` as pessoas e pastas |
| SRCHUI-04 | Ctrl F, resultado → visualizador → Esc de volta; a tela bate com `Search` em 1440×900, claro e escuro |
