# F04 — Database Schema Specification

## Problem Statement

O matcher (F02) já encontra as 4.247 mídias do Google Fotos em `E:\`, mas o resultado só existe em memória. O scanner (F05), o pipeline (F10) e as operações de arquivo (F48) precisam de um banco SQLite com um esquema estável e versionado, onde uma foto mantém a mesma identidade quando é movida ou renomeada (AD-008) e onde a lixeira do app guarda o que foi excluído por 30 dias (AD-009). Hoje o app só abre um banco em memória para responder `SELECT 1`.

## Goals

- [ ] O app abre (ou cria) o banco em `E:\.myphotos\library.db` ao iniciar, aplica as migrações pendentes e informa a versão do esquema na tela de saúde
- [x] O esquema v1 cobre todas as entidades do PRD 8.3 mais as decisões AD-008 a AD-011 (pastas, lixeira, identidade estável ao mover)
- [x] As migrações são versionadas, atômicas e nunca rodam num banco mais novo que o app
- [x] Nenhum teste lê ou escreve em `E:\`; o caminho do banco é um parâmetro

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Popular o banco (varrer, casar, gravar mídias) | F05/F10. Esta feature entrega o esquema vazio e a infraestrutura de abrir e migrar |
| Camada de repositório (insert/update/query tipados) | Nasce junto com quem a usa (F10, F48). Os testes daqui exercitam o esquema com SQL direto |
| FTS5 e índices de busca textual | F14 adiciona por migração própria |
| Operações de arquivo e esvaziamento da lixeira | F48. Aqui só existe a tabela e o prazo `purge_after` |
| `PRAGMA integrity_check`, backup automático, exportação de dados (RF-066) | F36 e F35 |
| Modo offline com o `E:` desconectado | Removido pelo AD-011 |
| Rollback de migração (downgrade) | Ver premissas: migrações são só para frente, com cópia de segurança antes |
| Volume ID / revinculação de letra de unidade | F11. A coluna existe; ninguém a preenche ainda |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Local do banco | `E:\.myphotos\library.db`. A pasta `.myphotos` é criada com atributo oculto se não existir; nada além dela é criado em `E:\` | AD-011 | y |
| `E:` ausente | O app não cria nada, não tenta outro disco, e a tela de saúde mostra `Falha: disco E: não encontrado` | AD-011 abre mão do offline; criar o banco no `C:` esconderia o problema | y |
| Identidade da mídia | `media_items.id INTEGER PRIMARY KEY AUTOINCREMENT`: nunca reutilizado. `relative_path` é mutável e único por biblioteca | Mover/renomear (AD-008) só troca o caminho; favoritos, notas e lixeira continuam apontando para o mesmo id | y |
| Caminhos | Relativos à raiz da biblioteca, separador `/`, como o F02 já reporta (inclui o nome da parte `takeout-…-1-002`) | PRD 8.3 (nota de design) e AD-010 | y |
| Pastas | Tabela `folders` com uma linha por pasta conhecida, inclusive vazia | Criar pasta vazia e renomear pasta (AD-008) precisam existir no banco sem mídia dentro | y |
| Lixeira | Tabela `trash_entries` com caminho original, caminho dentro de `.myphotos/trash`, `trashed_at` e `purge_after = trashed_at + 30 dias`, calculado pelo banco | AD-009 | y |
| Mecanismo de migração | `PRAGMA user_version` + lista ordenada de migrações SQL embutidas no binário. Cada migração roda numa transação; `user_version` só muda se ela inteira passar | Sem dependência nova; o rusqlite já está no projeto | y |
| Direção das migrações | Só para frente. Antes de migrar um banco existente (`user_version` ≥ 1), o arquivo é copiado para `library.db.bak-v{N}` ao lado | O PRD pede rollback "quando aplicável"; uma cópia é rollback suficiente para um app de um usuário | y |
| Banco mais novo que o app | Erro `db_newer_than_app`, arquivo intocado | Um app antigo não pode adivinhar o esquema novo | y |
| Arquivo que não é SQLite | Erro `db_not_sqlite`, arquivo intocado, nunca sobrescrito | Pode ser um arquivo do usuário com o mesmo nome | y |
| PRAGMAs por conexão | `foreign_keys = ON`, `journal_mode = WAL`, `synchronous = FULL`, `busy_timeout = 5000` | WAL para leitura concorrente com a indexação; FULL porque o disco é USB e pode ser desconectado | y |
| Datas | `TEXT` ISO-8601 UTC com sufixo `Z`, como no PRD 8.3 | Ordenável como texto e legível | y |
| Tabelas do PRD que só serão usadas em P2 (duplicatas, notas) | Entram na v1 | O roadmap pede o "esquema completo"; tabela vazia não custa nada e evita migração cedo | y |

**Open questions:** nenhuma. Spec confirmada pelo stakeholder em 2026-10-02, com os defaults acima.

### Dimensões implícitas

| Dimension | Resolução |
| --------- | --------- |
| Input validation & bounds | O esquema impõe os domínios com `CHECK` (tipos de mídia, confiança, status, fontes) e `NOT NULL`/`UNIQUE` (DB-04, DB-05). Caminho do banco inválido → erro sem criar nada (DB-01) |
| Failure / partial-failure | Migração que falha no meio é revertida inteira e `user_version` não muda (DB-03). Arquivo não-SQLite ou mais novo não é tocado (DB-02) |
| Idempotency / retry | Abrir o mesmo banco N vezes aplica as migrações só na primeira; as seguintes não mudam nada, nem criam cópia de segurança (DB-03) |
| Auth boundaries & rate limits | N/A porque o banco é local, de um usuário, sem rede |
| Concurrency / ordering | WAL + `busy_timeout`. Migrações rodam em ordem crescente de versão, uma transação cada. Duas instâncias do app ao mesmo tempo: N/A nesta feature (o app é instância única) |
| Data lifecycle / expiry | `trash_entries.purge_after` = 30 dias após `trashed_at` (DB-06). Excluir uma mídia do banco remove em cascata local, pessoas-da-mídia, álbuns-da-mídia, JSON bruto, notas e membros de duplicata; entradas da lixeira mantêm o caminho e perdem só o vínculo (DB-05) |
| Observability | A tela de saúde mostra a versão do esquema. Sem log de conteúdo, sem telemetria |
| External-dependency failure | Disco `E:` ausente ou que some durante a abertura → erro com código, nada criado em outro lugar (DB-01) |
| State-transition integrity | `user_version` só avança de N para N+1 dentro da transação da migração N+1 (DB-03) |

---

## User Stories

### P1: Banco aberto e migrado ao iniciar ⭐ MVP

**User Story**: Como usuário, quero que o My Photos crie e mantenha o banco do acervo dentro do próprio `E:\`, para que a organização viaje junto com o HD.

**Acceptance Criteria** (DB-01, DB-02):

1. WHEN o app abre o banco num caminho cuja pasta pai existe e o arquivo não existe THEN SHALL criar o arquivo, aplicar todas as migrações e `PRAGMA user_version` SHALL ser igual à última versão (1 nesta feature)
2. WHEN a pasta `.myphotos` não existe e a raiz do volume existe THEN SHALL criar `.myphotos` com o atributo oculto e então o banco dentro dela
3. WHEN a raiz do volume do caminho não existe (disco desconectado) THEN SHALL retornar o erro `volume_missing` e SHALL NOT criar pasta nem arquivo em lugar nenhum
4. WHEN o arquivo existe e não é um banco SQLite THEN SHALL retornar `db_not_sqlite` e os bytes do arquivo SHALL continuar idênticos
5. WHEN `user_version` do arquivo é maior que a última versão conhecida THEN SHALL retornar `db_newer_than_app` e os bytes do arquivo SHALL continuar idênticos
6. WHEN a conexão é aberta THEN `PRAGMA foreign_keys` SHALL ser 1, `journal_mode` SHALL ser `wal` e `synchronous` SHALL ser 2 (FULL)
7. WHEN o app inicia THEN a tela de saúde SHALL mostrar `Banco: esquema v1`; WHEN a abertura falha THEN SHALL mostrar `Falha: ` seguido da mensagem do código (ex.: `disco E: não encontrado`)

**Independent Test**: abrir um caminho num diretório temporário, ler `user_version` e os PRAGMAs; repetir com arquivo lixo e com `user_version` 99.

---

### P1: Migrações seguras e idempotentes ⭐ MVP

**User Story**: Como desenvolvedor das próximas features, quero adicionar migrações sem risco de deixar o banco do usuário pela metade.

**Acceptance Criteria** (DB-03):

1. WHEN um banco já migrado é aberto de novo THEN nenhuma migração SHALL rodar, nenhuma cópia `.bak` SHALL ser criada e o esquema (`sqlite_schema`) SHALL ser idêntico ao anterior
2. WHEN uma migração falha no meio THEN as mudanças dela SHALL ser revertidas, `user_version` SHALL continuar na versão anterior e a abertura SHALL retornar `migration_failed`
3. WHEN um banco existente com `user_version` N ≥ 1 vai ser migrado para N+1 THEN uma cópia byte a byte SHALL existir em `library.db.bak-vN` antes da primeira alteração
4. WHEN há migrações N+1..M pendentes THEN SHALL aplicá-las em ordem crescente, cada uma em sua transação

**Independent Test**: lista de migrações de teste com uma migração quebrada; conferir versão, esquema e arquivo `.bak`.

---

### P1: Esquema v1 completo ⭐ MVP

**User Story**: Como as features F05–F48, quero tabelas com restrições que impeçam dado inválido e preservem a identidade das fotos.

**Acceptance Criteria** (DB-04, DB-05, DB-06):

1. WHEN o esquema v1 é aplicado THEN SHALL existir exatamente as tabelas `libraries`, `folders`, `media_items`, `media_locations`, `people`, `media_people`, `albums`, `media_albums`, `media_metadata_raw`, `duplicate_groups`, `duplicate_group_members`, `user_notes`, `trash_entries`, `indexing_runs`, `indexing_errors`, `app_settings`, com as colunas do PRD 8.3 mais as listadas no design
2. WHEN se insere uma segunda mídia com o mesmo `(library_id, relative_path)` THEN SHALL falhar com violação de `UNIQUE`; o mesmo vale para `folders`
3. WHEN se insere `media_type` fora de `photo|video`, `json_match_confidence` fora de `exact|heuristic|none`, `taken_at_source` fora de `takeout_json|exif|filesystem` ou `indexing_status` fora de `ok|error|orphan_json|missing_file` THEN SHALL falhar com violação de `CHECK`
4. WHEN se atualiza o `relative_path` de uma mídia THEN o `id` SHALL continuar o mesmo e favoritos, notas, pessoas e álbuns ligados a ela SHALL continuar ligados
5. WHEN a mídia de maior `id` é apagada e outra é inserida THEN o novo `id` SHALL ser maior que o apagado (nunca reutilizado)
6. WHEN uma mídia é apagada THEN as linhas dela em `media_locations`, `media_people`, `media_albums`, `media_metadata_raw`, `user_notes` e `duplicate_group_members` SHALL ser apagadas, e `trash_entries.media_id` que apontava para ela SHALL virar NULL mantendo os caminhos
7. WHEN se insere mídia com `library_id` inexistente THEN SHALL falhar com violação de chave estrangeira
8. WHEN se insere em `trash_entries` com `trashed_at = '2026-10-02T12:00:00Z'` THEN `purge_after` SHALL ser `'2026-11-01T12:00:00Z'`
9. WHEN o esquema v1 é aplicado THEN SHALL existir os índices `idx_media_taken_at`, `idx_media_hash`, `idx_media_type` do PRD 8.3 e um índice de `trash_entries(purge_after)`

**Independent Test**: aplicar v1 num banco temporário e executar os INSERT/UPDATE/DELETE de cada critério.

---

## Edge Cases

- WHEN o caminho do banco é relativo THEN SHALL retornar `path_not_absolute` sem criar nada
- WHEN `.myphotos` já existe sem o atributo oculto THEN SHALL usá-la como está, sem mudar atributos
- WHEN existe `library.db-wal` de uma sessão interrompida THEN o SQLite SHALL recuperá-lo na abertura (comportamento padrão do WAL; nenhum código extra)
- WHEN a cópia `.bak-vN` já existe THEN SHALL ser sobrescrita pela cópia atual antes da migração

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| DB-01 | P1: Banco aberto e migrado — local, criação, disco ausente, PRAGMAs | T1–T8 | Verified |
| DB-02 | P1: Banco aberto e migrado — arquivo inválido ou mais novo intocado | T1–T8 | Verified |
| DB-03 | P1: Migrações seguras e idempotentes | T1–T8 | Verified |
| DB-04 | P1: Esquema v1 — tabelas, colunas, índices | T1–T8 | Verified |
| DB-05 | P1: Esquema v1 — restrições, identidade estável, cascatas | T1–T8 | Verified |
| DB-06 | P1: Esquema v1 — lixeira de 30 dias | T1–T8 | Verified |
| DB-07 | P1: Tela de saúde mostra a versão do esquema | T1–T8 | Verified |

**Coverage:** 7 total, 7 mapped to tasks, 0 unmapped. Verified in `validation.md` (PASS, iteration 2).

---

## Success Criteria

- [ ] `npm run tauri dev` com o `E:` conectado cria `E:\.myphotos\library.db` e mostra `Banco: esquema v1`
- [ ] Com o `E:` desconectado, mostra `Falha: disco E: não encontrado` e nada é criado em nenhum disco
- [x] Todos os critérios acima passam em `cargo test` usando só diretórios temporários
