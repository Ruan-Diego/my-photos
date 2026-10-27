# F23 — People View Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`People`**.

## Rust

- `commands/people.rs`: `people_cards(connection)` → `PersonCard {id, name, count, cover}`; a capa é a foto visível mais recente (vídeo só sem foto). Comando `people_list()` com a permissão `allow-people-list` (teste `acl01`).

## React

- `src/people/people.ts`: filtro por nome, textos ("28 pessoas…", "812 itens"), `personHash(id)` = `#/buscar?pessoa=7`, iniciais. Sem React.
- `src/people/PeopleScreen.tsx`: cabeçalho, campo (Esc limpa), grade `auto-fill minmax(150px, 1fr)` com círculo de 132 px. A capa vem do protocolo `thumb` (F18); sem capa, ou se ela falhar, as iniciais.
- `SearchScreen` abre com a pessoa marcada quando o hash é `#/buscar?pessoa=7` (também se ele mudar com a busca aberta).

## Fora do canvas

- Estados vazio ("Ninguém marcado no Google Fotos"), sem resultado no filtro ("Ninguém com esse nome") e falha, com `page-empty`.
- Hover: o círculo cresce 3% (sem movimento com `prefers-reduced-motion`).
