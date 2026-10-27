# F25 — Albums Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Albums`** (claro e escuro) (AD-014, `.specs/DESIGN.md`).

## Problem Statement

O Takeout guarda cada álbum do Google Fotos como uma pasta em `Google Fotos/` (às vezes repetida em várias partes do export) com um `metadata.json` que traz o título. A tabela `albums` existia desde a migração 1, mas ninguém a preenchia: o item "Álbuns" estava vazio e o painel Detalhes não tinha álbum (RF-043, RF-044, RF-045).

## Goals

- [x] Reconstruir os álbuns a cada indexação: uma pasta logo abaixo de "Google Fotos"/"Google Photos" é um álbum; as partes do export (`takeout-…-1-001`, `-2-001`) se juntam
- [x] Nome do `metadata.json` (title); sem ele, o nome da pasta; a descrição também é guardada
- [x] Álbum que sumiu do disco sai; o mesmo álbum mantém o id entre indexações
- [x] `#/albuns`: grade com capa empilhada (3 fotos), nome e "124 itens · ago 2022"; ordenar por recentes, nome ou tamanho (lembrado)
- [x] Clicar num álbum abre a busca filtrada por ele, com o chip do álbum

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Criar, renomear ou editar álbuns no app | RF-045 pede só leitura do Takeout; edição fica para depois da F48 |
| Página própria do álbum (timeline do álbum) | A busca filtrada já mostra as fotos em grade com o viewer; divergência registrada |
| Álbum no painel Detalhes | Fica para depois, junto com os links de pessoas e pasta |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| O que é álbum | Pasta direta em `Google Fotos/`, inclusive as "Photos from 2021" do próprio Takeout | É assim que o Takeout exporta; esconder os anuais seria adivinhar pelo nome | n |
| Discos já indexados | Os álbuns aparecem depois de "Procurar arquivos novos" | A reconstrução usa o relatório da descoberta, que só existe na indexação | n |
| Álbum sem mídia indexada | Não aparece na grade | Nada para mostrar | n |
| Capa | As 3 fotos mais novas do álbum (vídeo só sem foto); ocultos não contam | Canvas mostra capa empilhada | n |
| Clique | Abre `#/buscar?album=ID` | Divergência: o canvas não desenha a página do álbum | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| ALB-01 | Só pastas diretamente em "Google Fotos"/"Google Photos" são álbuns, e as partes do export se juntam |
| ALB-02 | A reconstrução nomeia, liga as mídias, mantém o id, troca o nome quando o metadata.json some e apaga o que sumiu |
| SRCH-12 | Filtro `album` na busca e lista de álbuns nas opções |
| ALBUI-01 | Ordenações e textos do canvas |
| ALBUI-02 | O álbum abre a busca filtrada |
