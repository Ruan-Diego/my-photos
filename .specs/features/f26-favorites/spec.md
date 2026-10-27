# F26 — Favorites Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Favorites`** (claro e escuro) e o coração do **`Viewer`** (AD-014, `.specs/DESIGN.md`).

## Problem Statement

O índice tinha `media_items.is_favorite`, a grade já mostrava o coração e o filtro "Favoritos", mas nada marcava um item: o botão do visualizador estava desativado e as favoritas do Google Fotos não vinham do Takeout. O usuário quer marcar o que gosta sem mexer no arquivo original (RF-046, US-10, AD-008).

## Goals

- [x] Coração no visualizador de foto e no player de vídeo marca e desmarca na hora; só o banco do app muda
- [x] As favoritas do Google Fotos (`"favorited": true` no JSON do Takeout) entram uma vez por item: desmarcar no app não volta na próxima indexação
- [x] `#/favoritos`: "96 itens · marcados só no app, o arquivo original não muda", filtro Tudo/Fotos/Vídeos e grade quadrada por ano, com o coração em cada item
- [x] Clicar abre o visualizador sobre os favoritos; desmarcar ali mantém o item na fila até fechar
- [x] Grade, filtro "Favoritos" da linha do tempo e "Só favoritos" da busca acompanham na hora

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Favoritar vários de uma vez | Depende da seleção (F48) |
| Atalho de teclado para favoritar | Fica com os atalhos (F30) |
| Mandar a marcação de volta ao Google Fotos | O app não usa rede (RNF-005) |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Favoritas do Takeout | Importadas uma vez por item (marca `takeout_favorites_imported_upto` em `app_settings`), ao abrir e ao fim de cada indexação | O que veio do Google aparece sem reindexar; desmarcar no app vale | n |
| Disco desconectado | Coração desativado | O banco vive no disco (AD-011) | n |
| Grade | Quadrada, 180 px mínimo, por ano, sem virtualizar | Igual ao canvas; favoritos são poucos | n |
| Falha ao gravar | O coração volta ao estado anterior | Sem mensagem; raro | n |

## Requirements

| ID | Requirement |
| -- | ----------- |
| FAV-01 | `favorite_set` marca e desmarca um item só no banco |
| FAV-02 | Favoritas do Takeout entram uma vez por item |
| FAVUI-01 | Textos do canvas |
| FAVUI-02 | Favoritos por ano, mais novo primeiro, com o filtro de tipo |
| FAVUI-03 | O coração muda um item; o desmarcado fica até o visualizador fechar |
| FAVUI-04 | O visualizador aberto em Favoritos volta para lá |
