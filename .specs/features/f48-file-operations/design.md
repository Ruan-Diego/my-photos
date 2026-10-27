# F48 — Design

## Rust (`src-tauri/src/fileops/`)

- O arquivo muda primeiro e o índice vem logo depois, na mesma chamada. Se o índice falhar, o arquivo volta.
- Lixeira: `trash_entries` (do F04) guarda a origem, o lugar na lixeira e os caminhos do JSON. Cada item fica numa pasta própria `.myphotos/trash/<data>-<n>/`, então nomes iguais não colidem.
- A mídia na lixeira continua com a sua linha, marcada `missing_file`. Todas as telas já escondem esse estado, e a varredura não mexe nele, porque ela pula `.myphotos` e não marca de novo o que já está faltando.
- Pasta na lixeira: as mídias de dentro recebem `updated_at = trashed_at`. É assim que restaurar e apagar de vez sabem quais são dela, sem coluna nova.
- `repath` troca o prefixo de `folders.relative_path`, `media_items.relative_path`, `json_relative_path` e `albums.folder_relative_path` (NOCASE). O gatilho do F14 atualiza a busca.
- `purge_expired` roda no `setup` do app. Apagar a linha leva junto, em cascata, pessoas, local e miniaturas do índice; os arquivos das miniaturas são apagados à parte.
- Comandos em `commands/fileops.rs`: `folder_tree`, `folder_items`, `folder_create`, `folder_rename`, `folder_trash`, `media_move`, `media_trash`, `trash_list`, `trash_restore`, `trash_restore_all`. As escritas recusam enquanto `IndexingState::is_running()`.

## Front

- `src/folders/folders.ts`: rotas (`#/pastas/<caminho codificado>`), caminho na barra, seleção com Shift e textos.
- `FoldersScreen.tsx` e `TrashScreen.tsx`. Depois de cada escrita, `onChanged` relê o resumo do disco, e com ele a linha do tempo.
