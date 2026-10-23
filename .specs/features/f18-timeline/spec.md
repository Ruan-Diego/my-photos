# F18 — Timeline View Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboards **`Timeline`** / **`TimelineEscuro`** (AD-014, `.specs/DESIGN.md`).

## Problem Statement

Depois da indexação a seção Fotos ainda dizia "esta tela está sendo construída". O usuário precisa rolar a biblioteca inteira por data, mais nova primeiro, saltar para um ano e ver as fotos sem esperar (RF-016 a RF-020, RNF-001, US-01, US-02).

## Goals

- [x] Grade por mês ("Agosto de 2022") e dia ("Qua, 10 de ago · 14 itens"), mais nova primeiro
- [x] Linhas justificadas de 176 px que respeitam a proporção da foto, como o `flex-wrap` do canvas
- [x] Só as linhas visíveis vão para o DOM (virtualização própria, sem dependência nova)
- [x] Miniaturas pequenas do F12 servidas por um protocolo `thumb`; placeholder `--sunken` até chegar
- [x] Vídeo com selo de duração; favorito com coração
- [x] Filtros Tudo / Fotos / Vídeos / Favoritos; controle Tamanho (5 alturas, 176 px no meio)
- [x] Trilho de anos à direita: salta para o ano; o ano na tela fica em `--accent-fill`
- [x] Estados vazio (biblioteca sem mídia, filtro sem resultado) e erro

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Abrir a foto ao clicar | F19 (visualizador) |
| Busca (`Ctrl F`) | F22; o campo aparece como no canvas, inativo |
| Selecionar | Operações de arquivo (F48); o botão aparece, inativo |
| Nome do local no dia ("Lisboa, Portugal") | O índice só tem coordenadas (`place_label` vazio); entra com Locais (F24) |
| Gerar miniatura na hora ao rolar | A indexação (F17) já gera as pequenas; a que falta fica no placeholder até a próxima indexação |
| Biblioteca com o disco desconectado | F27 |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Dia de cada foto | No fuso da foto quando o arquivo o informa (`taken_at_timezone`); senão no fuso do PC | O banco guarda UTC | n |
| Sem data | Grupo "Sem data" no fim | Não somem da biblioteca | n |
| Quais mídias | As mesmas que a lateral conta: sem ocultas e sem `missing_file` | Contagem igual nos dois lugares | n |
| Última linha do dia | Não estica; fica na proporção da foto | "Respeitando a proporção" (DESIGN.md); esticar uma foto sozinha cortaria demais | n |
| Proporção | Da miniatura pequena (já girada); senão largura/altura do índice com a orientação 5–8; senão 1; limitada a 0,5–3 | Panorama não ocupa a linha toda | n |
| Tamanho | 96 / 128 / 176 / 232 / 296 px, lembrado no navegador | 176 é o canvas; 296 ≈ a miniatura pequena (300 px) | n |
| Busca e Selecionar inativos | Visíveis com dica "chega com…" | Mesmo critério do "Trocar pasta" na F16 | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| TL-01 | Mídias agrupadas por mês e dia no fuso certo, mais nova primeiro, sem data no fim |
| TL-02 | Filtros Tudo, Fotos, Vídeos, Favoritos, com texto próprio quando vazios |
| TL-03 | Rótulos como no canvas: "Agosto de 2022", "Qua, 10 de ago", "14 itens", duração "2:18" |
| TL-04 | Linhas quebram como o `flex-wrap` do canvas; panorama e foto estreita limitados |
| TL-05 | Só as linhas na tela (mais uma tela de folga) são renderizadas; trilho de anos salta e destaca o ano |
| TL-06 | O comando `timeline` devolve as mídias em vetores (id, hora, fuso, proporção, duração, flags) |
| TL-07 | O protocolo `thumb` entrega a miniatura pequena atual e marca o uso (LRU do F12); 404 quando não há |
| TL-08 | A tela bate com `Timeline` / `TimelineEscuro` em 1440×900 |
