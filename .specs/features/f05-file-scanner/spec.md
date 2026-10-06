# F05 — File Scanner Specification

**Status**: Confirmada pelo stakeholder em 2026-10-06 ("pode prosseguir"), com os defaults abaixo. Escrita numa sessão na nuvem, sem o PRD e sem o `E:`

## Problem Statement

A F02 encontra e casa as mídias do Takeout só em memória, e a F04 entrega um banco vazio em `E:\.myphotos\library.db`. Nada ainda grava o que existe no disco. O pipeline (F10), a timeline (F18) e as operações de arquivo (F48) precisam de uma tabela `media_items` que reflita o disco: cada mídia com caminho, tipo, tamanho, datas do sistema de arquivos e SHA-256, e cada pasta em `folders`. Uma segunda varredura precisa saber o que é novo, o que mudou e o que sumiu, sem recalcular o hash de tudo e sem perder a identidade (`id`) de quem continua lá.

## Goals

- [ ] Uma varredura de `E:\` grava em `folders` e `media_items` todas as pastas e mídias do disco, ignorando `.myphotos`, pastas de sistema e arquivos não compatíveis (AD-010, AD-011)
- [ ] Cada mídia tem SHA-256, tamanho e datas do sistema de arquivos; o casamento com o JSON do Takeout (F02) fica gravado em `json_relative_path` e `json_match_confidence`
- [ ] Uma nova varredura detecta mídias novas, modificadas e removidas, e só recalcula o hash do que é novo ou mudou
- [ ] Uma mídia que continua no disco nunca troca de `id`; uma mídia removida não apaga favoritos nem notas
- [ ] Cada varredura deixa um registro em `indexing_runs` e os problemas por arquivo em `indexing_errors`
- [ ] Nenhum teste lê ou escreve em `E:\`; a raiz e o banco são parâmetros

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Ler o conteúdo do JSON do Takeout para preencher data, local, pessoas, título | F06/F09/F10. A F05 grava só o casamento (qual JSON e com que confiança), não os campos |
| EXIF, dimensões, orientação, duração de vídeo | F07 e F13 |
| `taken_at` e `taken_at_source` | F09 (resolver). Ficam `NULL` depois da F05 |
| Álbuns do Takeout (`metadata.json`) | F25. A F02 já os encontra; a F05 não grava `albums` |
| Progresso na UI, cancelamento pela UI, tela de onboarding | F16/F17. A F05 expõe contagens no resultado, sem tela |
| Varredura automática ao abrir o app | F10 orquestra quando varrer. Ver premissas |
| Hash perceptual e agrupamento de duplicatas | F29. Duas cópias iguais em pastas diferentes são duas linhas |
| Revinculação por volume ID / troca de letra | F11 |
| Vigiar o disco em tempo real (watcher) | Fora do MVP. A detecção é por varredura |
| Apagar linhas de mídia removida | F28/F48 decidem. A F05 só marca `missing_file` |
| Escrever qualquer coisa fora de `E:\.myphotos` | AD-008 vale só para ação explícita do usuário; a varredura é só leitura |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Raiz varrida | `E:\` inteiro (AD-007, AD-010), árvore como está no disco. A função recebe a raiz e o banco como parâmetros | AD-010 | y |
| O que é ignorado | `.myphotos` na raiz (caixa ignorada), pastas oculto+sistema (`$RECYCLE.BIN`, `System Volume Information`), symlinks e junções (não seguidos), arquivos que não são mídia (`.zip`, `.html`, `.txt`, JSON). Ignorado não vira erro | AD-010, AD-011; F02 já não segue symlink | y |
| O que é mídia | As 18 extensões da F02 (`classify`). `media_type` = `video` para `mp4 mov mkv avi 3gp webm m4v`, `photo` para o resto | Mesma lista da F02; o esquema só aceita `photo`/`video` | y |
| Linha da biblioteca | A primeira varredura cria `libraries` com `name = 'E:'`, `root_path = 'E:\'`, `status = 'active'`; as seguintes reusam a mesma linha (por `root_path`) | Esquema da F04 exige `library_id`; só existe uma biblioteca no MVP | y |
| Pastas | Toda pasta visitada (exceto a raiz e as ignoradas) vira uma linha em `folders`, inclusive vazia ou sem mídia. Pasta que sumiu do disco é apagada de `folders` | F04: "a raiz não tem linha"; `folders` não tem status nem filhos com FK | y |
| Identidade de uma mídia | Mesmo `relative_path` (comparação sem caixa, como o NTFS e o `COLLATE NOCASE`) = mesma linha, mesmo `id` | F04: `id` nunca reutilizado; caminho mutável | y |
| Mídia nova | Caminho que não existe no banco → `INSERT` com hash, tamanho, datas, casamento F02, `indexing_status = 'ok'` | — | y |
| Mídia modificada | Mesmo caminho, mas `size_bytes` ou `fs_modified_at` diferente do banco → recalcula o hash e atualiza a linha (mesmo `id`) | Tamanho + mtime é o critério barato padrão; reler 13 GB a cada varredura não escala | y |
| Mídia inalterada | Mesmo caminho, mesmo tamanho e mesmo mtime → hash **não** é recalculado; só o casamento F02 é atualizado se mudou | Varredura incremental (US-07) | y |
| Mídia removida | Caminho no banco que não apareceu numa varredura **completa** → `indexing_status = 'missing_file'`, linha mantida (favorito, notas e lixeira continuam ligados). Se o arquivo voltar no mesmo caminho → volta a `ok` com o mesmo `id` | Apagar a linha cascatearia favoritos e notas (F04 DB-05) | y |
| Mídia movida ou renomeada fora do app | Mídia nova cujo SHA-256 e tamanho batem com **exatamente uma** linha `missing_file` → essa linha recebe o novo caminho (mesmo `id`). Zero ou mais de uma candidata → linha nova | Mantém favoritos quando o usuário reorganiza pelo Explorer. Empate não adivinha | y |
| Varredura incompleta | Se a raiz some no meio (disco desconectado) ou uma pasta não pode ser lida, nenhuma mídia daquela parte vira `missing_file`. A run termina `failed` (raiz sumiu) ou `completed` com erro `unreadable_directory` (pasta) | Disco USB que cai não pode "apagar" a biblioteca | y |
| Hash | SHA-256 do arquivo inteiro, hex minúsculo de 64 caracteres em `file_hash_sha256`. Leitura sequencial, uma mídia por vez, em ordem de caminho | `E:` é USB e respondeu `unknown` no F03; leitura paralela em HDD disputa a cabeça. F32 ajusta | y |
| Datas do sistema de arquivos | `fs_modified_at` e `fs_created_at` em ISO-8601 UTC com `Z`, precisão de segundo (fração truncada). Data indisponível → `NULL` | Formato das datas da F04 | y |
| Casamento com JSON | Usa a descoberta da F02 sobre a mesma árvore: `has_takeout_json`, `json_relative_path`, `json_match_confidence` (`exact`/`heuristic`/`none`). JSON inválido ou ilegível → `none` + erro `invalid_json`/`unreadable` | STATE: "usar o matcher da F02" | y |
| Erros por arquivo | Arquivo que não abre no hash → linha com `indexing_status = 'error'`, `indexing_error` com o motivo, e uma linha em `indexing_errors` (`unreadable`). A varredura continua | US-08, RF-063 | y |
| Registro da varredura | Uma linha em `indexing_runs` por chamada: `run_type = 'full'` na primeira da biblioteca, `'incremental'` nas seguintes; `files_scanned` = mídias vistas, `files_indexed` = inseridas ou atualizadas, `files_errored` = com erro; `status` final `completed` ou `failed`. Uma run que ficou `running` (app fechado no meio) vira `failed` na próxima varredura | Esquema da F04 | y |
| Gravação | Em lotes de até 500 mídias por transação, para uma queda do disco perder no máximo um lote e não travar o banco por minutos. A marcação de `missing_file` acontece só no fim, numa transação | WAL + `synchronous=FULL` (F04) | y |
| Como a varredura é disparada | Função Rust `scan(connection, root)` mais um teste `#[ignore]` que varre o `E:` real, como o `real_takeout_survey` da F02. **Sem** comando Tauri e sem varrer ao abrir o app | Phase 1 é "sem UI além de logging"; F10/F16/F17 decidem o gatilho | y |
| `last_indexed_at` | `libraries.last_indexed_at` = fim da última run `completed` | Esquema da F04 | y |

**Open questions:** nenhuma. O PRD não está no repositório; RF-001, RF-002 e RF-010 a RF-013 foram inferidos do roadmap.

### Dimensões implícitas

| Dimension | Resolução |
| --------- | --------- |
| Input validation & bounds | Raiz que não existe ou não é pasta → erro `root_missing`, nada gravado. Banco é uma `Connection` já aberta pela F04 |
| Failure / partial-failure | Arquivo ilegível → erro por arquivo, a varredura continua (SCAN-05). Raiz some no meio → run `failed`, lotes já gravados ficam, nada vira `missing_file` (SCAN-04) |
| Idempotency / retry | Duas varreduras seguidas sem mudança no disco → mesmas linhas, mesmos `id`s, nenhum hash recalculado na segunda (SCAN-03) |
| Auth boundaries & rate limits | N/A porque é local, um usuário, sem rede |
| Concurrency / ordering | Uma varredura por vez (a função recebe `&mut Connection`). Ordem de visita e de gravação = ordem lexicográfica do caminho relativo. Leituras de outra conexão continuam (WAL) |
| Data lifecycle / expiry | Linha `missing_file` fica até a F28/F48 decidir. Pasta sumida sai de `folders` |
| Observability | `indexing_runs` + `indexing_errors`; a função devolve um resumo com contagens. Sem log de conteúdo, sem telemetria |
| External-dependency failure | Disco `E:` desconectado antes → `root_missing`; durante → run `failed` (SCAN-04) |
| State-transition integrity | `indexing_status`: `ok` ↔ `missing_file`, `ok` ↔ `error`; `indexing_runs.status`: `running` → `completed`/`failed` |

---

## User Stories

### P1: Primeira varredura grava o disco no banco ⭐ MVP

**User Story**: Como usuário, quero que o My Photos encontre todas as fotos e vídeos do `E:\` e guarde o que achou, para a biblioteca existir sem eu organizar nada.

**Acceptance Criteria** (SCAN-01, SCAN-02):

1. WHEN uma raiz com `a/foto.jpg`, `a/b/clip.mp4`, `vazia/` e `notas.txt` é varrida num banco novo THEN `media_items` SHALL ter exatamente `a/foto.jpg` (`photo`) e `a/b/clip.mp4` (`video`), e `folders` SHALL ter exatamente `a`, `a/b` e `vazia`
2. WHEN a raiz tem `.myphotos/trash/x.jpg` (ou `.MyPhotos/…`) THEN nada de dentro dela SHALL estar em `media_items` nem em `folders`
3. WHEN a raiz tem um symlink ou junção de pasta apontando para fora dela THEN nada do alvo SHALL ser gravado
4. WHEN uma mídia é gravada THEN `filename`, `extension` (minúscula), `size_bytes`, `fs_modified_at` (ISO UTC com `Z`) e `file_hash_sha256` (64 hex minúsculos, igual ao SHA-256 dos bytes do arquivo) SHALL estar preenchidos e `indexing_status` SHALL ser `ok`
5. WHEN a pasta tem `foto.jpg` e `foto.jpg.json` THEN a linha de `foto.jpg` SHALL ter `has_takeout_json = 1`, `json_relative_path` = caminho do JSON, `json_match_confidence = 'exact'`; WHEN a mídia não tem JSON THEN `has_takeout_json = 0` e `json_match_confidence = 'none'`
6. WHEN a varredura termina THEN SHALL existir uma linha em `libraries` para a raiz, uma em `indexing_runs` com `run_type = 'full'`, `status = 'completed'`, `files_scanned` = número de mídias, e `libraries.last_indexed_at` = `finished_at`
7. WHEN a varredura termina THEN os bytes de todos os arquivos da raiz SHALL continuar idênticos e nenhum arquivo SHALL ser criado fora do banco

**Independent Test**: árvore temporária criada pelo teste + banco temporário aberto com `db::open`; ler as tabelas.

---

### P1: Varredura incremental ⭐ MVP

**User Story**: Como usuário, quero que uma nova varredura só processe o que mudou, para não esperar o hash de 13 GB de novo (US-07).

**Acceptance Criteria** (SCAN-03):

1. WHEN o mesmo disco é varrido duas vezes sem mudança THEN as linhas de `media_items` SHALL ser idênticas (mesmos `id`s, hashes e datas), nenhum hash SHALL ser recalculado na segunda, e a segunda run SHALL ser `incremental`
2. WHEN um arquivo novo aparece entre as varreduras THEN ele SHALL ser inserido e os `id`s dos demais SHALL continuar os mesmos
3. WHEN o conteúdo de um arquivo muda (tamanho ou mtime diferente) THEN o hash SHALL ser recalculado e a linha SHALL manter o `id`
4. WHEN só o JSON de uma mídia aparece ou some entre as varreduras THEN o casamento da linha SHALL ser atualizado sem recalcular o hash

**Independent Test**: varrer, mudar a árvore, varrer de novo; um contador de hashes injetado prova o que foi relido.

---

### P1: Mídia removida e mídia movida ⭐ MVP

**User Story**: Como usuário, quero que apagar ou mover uma foto pelo Explorer não apague o favorito dela por engano.

**Acceptance Criteria** (SCAN-04):

1. WHEN uma mídia some do disco e a varredura é completa THEN a linha SHALL ficar com `indexing_status = 'missing_file'`, com o mesmo `id`, e o favorito e a nota dela SHALL continuar
2. WHEN a mídia `missing_file` volta ao mesmo caminho THEN a linha SHALL voltar a `ok` com o mesmo `id`
3. WHEN `a/foto.jpg` é movida para `b/foto2.jpg` com o mesmo conteúdo THEN a linha SHALL passar a ter `relative_path = 'b/foto2.jpg'` com o mesmo `id`, e não SHALL existir linha nova
4. WHEN duas linhas `missing_file` têm o mesmo hash do arquivo novo THEN o arquivo novo SHALL virar uma linha nova e as duas SHALL continuar `missing_file`
5. WHEN uma pasta não pode ser lida THEN as mídias conhecidas dela SHALL continuar com o status que tinham, e SHALL existir um `indexing_errors` `unreadable_directory` com o caminho da pasta
6. WHEN a raiz deixa de existir durante a varredura THEN a run SHALL terminar `failed`, nenhuma linha SHALL virar `missing_file`, e as mídias já gravadas em lotes anteriores SHALL continuar no banco
7. WHEN uma pasta some do disco THEN a linha dela SHALL sair de `folders`

**Independent Test**: varreduras sobre árvores temporárias; a queda do disco é simulada por um leitor injetado que falha a partir do N-ésimo arquivo.

---

### P1: Erros por arquivo e registro da varredura ⭐ MVP

**User Story**: Como usuário, quero que um arquivo corrompido ou bloqueado não pare a varredura, e saber depois o que falhou (US-08).

**Acceptance Criteria** (SCAN-05, SCAN-06):

1. WHEN um arquivo de mídia não pode ser lido para o hash THEN a linha SHALL existir com `indexing_status = 'error'` e `file_hash_sha256` `NULL`, SHALL existir um `indexing_errors` `unreadable` com o caminho, e as demais mídias SHALL ser gravadas normalmente
2. WHEN o JSON casado é inválido THEN a mídia SHALL ficar `json_match_confidence = 'none'` e SHALL existir um `indexing_errors` `invalid_json` com o caminho do JSON
3. WHEN um nome de arquivo não é Unicode válido THEN SHALL existir um `indexing_errors` `invalid_name` e o arquivo SHALL ficar fora de `media_items`
4. WHEN a varredura termina com erros por arquivo THEN a run SHALL ser `completed` com `files_errored` igual ao número de mídias com erro
5. WHEN existe uma run `running` de uma execução anterior THEN a nova varredura SHALL marcá-la `failed` antes de começar
6. WHEN a raiz não existe THEN a função SHALL retornar `root_missing` e nenhuma linha SHALL ser gravada

**Independent Test**: árvore temporária com um arquivo bloqueado (leitor injetado), um JSON inválido e uma run `running` pré-gravada.

---

### P2: Varredura do `E:` real (manual)

**User Story**: Como stakeholder, quero rodar a varredura uma vez no `E:` e ver as contagens, para confirmar que as 4.247 mídias do Google Fotos e as ~1.885 de fora entram no banco.

**Acceptance Criteria** (SCAN-07):

1. WHEN o teste `#[ignore]` `real_library_scan` roda com o `E:` conectado THEN SHALL imprimir mídias por tipo, por confiança de JSON, erros por tipo e tempo total, e `E:\.myphotos\library.db` SHALL ter as linhas
2. WHEN roda de novo logo em seguida THEN SHALL imprimir zero hashes recalculados

**Independent Test**: manual, na máquina do stakeholder. Não roda no CI.

---

## Edge Cases

- WHEN dois arquivos diferem só na caixa do nome na mesma pasta (possível fora do NTFS) THEN só o primeiro em ordem SHALL ser gravado e o segundo SHALL gerar `indexing_errors` `invalid_name`
- WHEN uma mídia tem 0 bytes THEN SHALL ser gravada com o SHA-256 da entrada vazia (`e3b0c442…b855`)
- WHEN a mídia é renomeada só na caixa (`foto.jpg` → `Foto.JPG`) THEN a linha SHALL manter o `id` e passar a ter o novo `relative_path`, `filename` e `extension`
- WHEN uma pasta de sistema (oculta+sistema) aparece fora da raiz THEN SHALL ser pulada igual à da raiz
- WHEN o arquivo muda entre o `stat` e o fim do hash THEN o hash gravado SHALL ser o dos bytes lidos e o `fs_modified_at` o lido antes; a próxima varredura detecta a diferença

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| SCAN-01 | P1: Primeira varredura — o que entra e o que é ignorado | T1–T2 | Implemented (CI pendente) |
| SCAN-02 | P1: Primeira varredura — campos, hash, casamento, registro | T1–T2 | Implemented (CI pendente) |
| SCAN-03 | P1: Varredura incremental | T1–T2 | Implemented (CI pendente) |
| SCAN-04 | P1: Mídia removida e movida, varredura incompleta | T1–T2 | Implemented (CI pendente) |
| SCAN-05 | P1: Erros por arquivo | T1–T2 | Implemented (CI pendente) |
| SCAN-06 | P1: Registro da varredura (`indexing_runs`) | T1–T2 | Implemented (CI pendente) |
| SCAN-07 | P2: Varredura do `E:` real (manual) | T2 | Pending (manual) |

**Coverage:** 7 total, 7 mapped to tasks. Evidência em `validation.md`.

---

## Success Criteria

- [ ] Com o `E:` conectado, `real_library_scan` grava as mídias do `E:` e uma segunda execução não recalcula hash nenhum
- [ ] Mover uma foto favoritada pelo Explorer e varrer de novo mantém o favorito
- [ ] Todos os critérios P1 passam em `cargo test` usando só diretórios temporários, no CI `windows-latest`
