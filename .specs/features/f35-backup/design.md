# F35 — Design

- `src-tauri/src/backup/`:
  - `folder` aponta para `.myphotos/backups`, ao lado do banco.
  - `list` lê os arquivos do mais novo para o mais antigo; a hora está no nome.
  - `backup_now` faz o `VACUUM INTO` e apaga as cópias além de `KEEP`; `auto_backup` decide pelo `mtime` da cópia mais nova.
  - `export`, `write_export`, `read_export` e `import` tratam os favoritos e as notas.
  - `Finder` procura a mídia pelo caminho (sem diferenciar maiúsculas) e, se não achar, por todas as cópias com o mesmo SHA-256.
  - O `ON CONFLICT` nas notas dispara o gatilho da busca, então a nota importada já aparece na busca.
- Comandos em `commands/backup.rs`: `backup_overview`, `backup_now`, `appdata_export` e `appdata_import`, com permissões em `library.toml` e na capability (teste `acl01`).
- Em `lib.rs`, o `setup` chama `auto_backup` numa thread com conexão própria.
- O cartão Backup fica em `SettingsScreen.tsx`; o texto sai de `settings/backup.ts` (`lastBackupLine`, `exportOptions` e `importText`), e `trashLine` está em `settings.ts`.
