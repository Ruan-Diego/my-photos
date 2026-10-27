# F24 — Locations View Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Locations`** (claro e escuro) (AD-014, `.specs/DESIGN.md`).

## Problem Statement

O índice guarda latitude e longitude (EXIF e JSON do Takeout, F09), mas sem nome de lugar: o item "Locais" estava vazio, o painel Detalhes só mostrava coordenadas e o filtro Local da busca estava inativo. O usuário quer ver onde as fotos foram tiradas e achar as fotos de um lugar (RF-029, RF-030, RF-031, RF-037, US-06), sem rede (RNF-005).

## Goals

- [x] Nome do lugar offline: a cidade (5 mil habitantes ou mais) mais perto, até 50 km, gravado em `media_locations.place_label` ao indexar e, para o que já estava indexado, ao abrir o app
- [x] `#/locais`: "3.402 itens com coordenadas · 845 sem localização", mapa e "Lugares com mais fotos"
- [x] Mapa offline com contorno dos continentes, alfinetes com a contagem ("1,8 mil"), agrupados quando ficam próximos, arrastar, roda e botões + e −
- [x] Lista com capa, nome, "SP, Brasil · 2015–2024" e contagem
- [x] Clicar num lugar (lista ou alfinete) abre a busca filtrada por ele; o campo Local da busca passa a filtrar
- [x] A busca de texto acha o nome do lugar ("lisbon", "sao paulo")

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Ruas e mapa detalhado (tiles) | Não há tiles offline no app; o contorno 1:110m basta para achar o lugar |
| Nomes de cidade em português ("Lisboa") | O GeoNames usa o nome mais comum ("Lisbon"); as cidades do Brasil já vêm em português |
| Lugar e mapa no painel Detalhes | Fica para depois; o painel continua com as coordenadas |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Base de cidades | GeoNames via `all-the-cities` (CC BY 4.0), cidades de 5 mil+ sem bairros, embutida (1,5 MB) | Sem rede; cobre as cidades do Brasil | n |
| Cidade escolhida | A maior entre as que estão até 4 km além da mais próxima | Times Square é Nova York, não Hoboken; Osasco continua Osasco | n |
| Rótulo | "São Paulo, SP, Brasil"; EUA com o estado; outros países só o país | Igual ao canvas ("SP, Brasil", "Portugal") | n |
| Fotos longe de cidade (mar, trilha) | Contam como "com coordenadas", mas não entram na lista nem no mapa | Sem nome confiável | n |
| Mapa | Natural Earth 1:110m (domínio público), equiretangular; créditos "Natural Earth · GeoNames" no canto | Canvas pede "tiles offline" | n |
| Período | "ago 2022" num mês, "2022" num ano, senão "2015–2024" | Canvas | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| PLC-01 | Coordenadas viram "Cidade, UF, País" offline; longe de cidade, nada |
| PLC-02 | O que já estava indexado ganha lugar ao abrir; a busca acompanha |
| LOC-01 | `locations_overview` conta itens com e sem coordenadas e agrupa por lugar com datas, centro e capa |
| SRCH-11 | Filtro `place`: uma frase na coluna de lugar, sem acento |
| LOCUI-01 | Textos do canvas |
| LOCUI-02 | Alfinetes agrupados conforme o zoom; a primeira vista enquadra todos |
| LOCUI-03 | O lugar abre a busca filtrada |
