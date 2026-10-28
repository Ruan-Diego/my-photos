# F36 — Design

- `src-tauri/src/isolate.rs`: `contained(f) -> Option<T>` usa `catch_unwind`. Quem chama:
  - `thumbs::render`: um pânico vira `ThumbError::Corrupt`;
  - `photo_meta::read_embedded`: um pânico vira metadados vazios com `exif_corrupt`;
  - `duplicates::fill_perceptual_hashes`: um pânico deixa o item sem hash.
- `backup::backup_now` chama `problems::integrity` antes do `VACUUM INTO`. A cópia automática passa por ela.
- `capabilities/default.json`: os `allow-*` do app e `core:event:default`.
- `tauri.conf.json`, CSP:
  ```
  default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline';
  img-src 'self' data: thumb: http://thumb.localhost media: http://media.localhost;
  media-src 'self' media: http://media.localhost; font-src 'self' data:;
  connect-src ipc: http://ipc.localhost; object-src 'none'; base-uri 'none';
  form-action 'none'; frame-ancestors 'none'
  ```
  - No build, o Tauri acrescenta ao `script-src` o hash do script inline do tema (`index.html`).
  - O `style-src` fica fora dessa modificação (`dangerousDisableAssetCspModification: ["style-src"]`). Com um hash, o `'unsafe-inline'` seria ignorado, e o React usa `style=` para a posição das linhas e dos mosaicos.
  - `data:` em fontes e imagens cobre as fontes woff embutidas no CSS e as miniaturas de "Chegando agora".
  - No `tauri dev` a página vem do Vite e o Tauri não aplica a CSP.

## Auditoria das entradas (SEC-05)

| Comando | Entrada | Validação |
| ------- | ------- | --------- |
| `folder_create`, `folder_rename` | nome | `fileops::valid_name`: vazio, mais de 120 caracteres, `\ / : * ? " < > \|`, caracteres de controle, ponto no fim, `..`, CON/PRN/AUX/NUL/COMn/LPTn, `.myphotos` |
| `folder_*`, `media_move` | pasta | precisa existir na tabela `folders`; o caminho em disco é montado parte por parte a partir da raiz |
| `trash_restore` | id | `slot_of` só aceita `.myphotos/trash/<slot>`, sem `.` nem `..` |
| `appdata_import` | nome | só um nome `dados-*.json` (sem `/`, `\` ou `..`) dentro da pasta de backups, até 64 MB, com `app` e `version` |
| `settings_set_flag`, `settings_set_cache_limit` | chave, bytes | listas fechadas |
| `recent_thumbnails` | limite | no máximo 48 |
| `search_media` | texto, filtros | parâmetros SQL; o texto vira uma expressão FTS com aspas |
| protocolos `thumb`, `media` | caminho | só id numérico; `Range` validado e limitado (`parse_range`, `MAX_RANGE`) |
| ids (`favorite_set`, `media_*`, detalhes) | `i64` | parâmetros SQL; id inexistente devolve `None` ou é pulado |
| comandos que mexem em arquivos | — | recusam enquanto a indexação roda |
