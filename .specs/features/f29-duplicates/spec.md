# F29 — Duplicate Detection Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Duplicates`** (claro e escuro) (AD-014, `.specs/DESIGN.md`).

## Problem Statement

O Takeout repete a mesma foto na pasta do ano e na de cada álbum, e o acervo tem cópias e versões redimensionadas. O índice já tinha o SHA-256 (F05) e a coluna `perceptual_hash`, mas nada agrupava as cópias. O usuário quer ver onde estão, sem nada ser apagado (RF-050, RF-051, RF-052).

## Goals

- [x] Idênticas: mesmo SHA-256, os grupos com mais espaço repetido primeiro, "3,1 MB repetidos"
- [x] Parecidas: dHash de 64 bits da miniatura pequena, até 6 bits de diferença, agrupadas; o mesmo arquivo em pastas diferentes fica só em Idênticas
- [x] `#/duplicatas`: "Todas · 214 / Idênticas · 187 / Parecidas · 27", nota sobre o Takeout, grupos com selo, título, cópias com miniatura, nome, pasta e tamanho/data
- [x] A miniatura abre o visualizador sobre as cópias
- [x] Nada é excluído; o hash perceptual some quando o conteúdo do arquivo muda

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| "Mostrar na pasta" | A tela Pastas é a F48; o link fica desativado |
| Excluir ou mover cópias | Só informativo (RF-052); operações de arquivo na F48 |
| Vídeos parecidos | O hash vem da capa; só fotos por enquanto |
| Grupos `same_json` e a tabela `duplicate_groups` | Os grupos saem na hora da consulta; a tabela fica para quando houver ação sobre eles |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Quando calcular o hash perceptual | Ao fim das miniaturas de cada indexação, lendo a miniatura pequena | Já está no disco; sem decodificar o original de novo | n |
| Discos já indexados | Parecidas aparecem depois de "Procurar arquivos novos" (a tela diz quantas fotos já foram comparadas) | Idênticas funcionam já, com o SHA-256 | n |
| Limiar | 6 de 64 bits | Pega redimensionada e recomprimida; evita fotos diferentes da mesma cena | n |
| Teto | 200 grupos na tela | Contagem nos filtros é de todos | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| DUP-01 | dHash estável ao redimensionar e diferente para outra foto |
| DUP-02 | Grupos idênticos por SHA-256 e parecidos por dHash, ocultos e sumidos fora |
| DUP-03 | Cópias idênticas sozinhas não viram grupo "parecidas" |
| DUP-04 | Hash calculado das miniaturas pequenas de fotos, uma vez |
| DUPUI-01 | Filtros, títulos e textos do canvas |
| DUPUI-02 | O visualizador anda pelas cópias; aviso quando faltam comparações |
