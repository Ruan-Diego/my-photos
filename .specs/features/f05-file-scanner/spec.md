# F05 — File Scanner Specification

## Problem Statement

A F04 entregou o banco `E:\.myphotos\library.db` com o esquema v1 vazio. O F02 sabe casar mídia e JSON, mas só em memória. Falta o passo que olha o disco: percorrer `E:\`, reconhecer as fotos e vídeos, gravar uma linha por pasta e por mídia, calcular o SHA-256 de cada arquivo e, numa segunda varredura, perceber o que é novo, o que mudou e o que sumiu, sem refazer o trabalho caro (hash no HDD frio é o gargalo medido na F03).

## Goals

- [ ] Uma varredura de `E:\` grava em `folders` todas as pastas como estão no disco e em `media_items` todas as fotos e vídeos, com tamanho, datas do sistema de arquivos e SHA-256
- [ ] Uma segunda varredura sem mudanças no disco não lê o conteúdo de nenhum arquivo
- [ ] Mídia nova, modificada, removida e reaparecida é detectada; o `id` de uma mídia nunca muda por causa da varredura
- [ ] A varredura nunca escreve, move, renomeia nem apaga nada na biblioteca; só escreve no banco
- [ ] Nenhum teste lê ou escreve em `E:\`; a raiz e o banco são parâmetros

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Ler JSON do Takeout, casar mídia↔JSON, preencher `taken_at`, local, pessoas, álbuns | F06/F08/F09, orquestrados pela F10. O F05 só grava o que vem do sistema de arquivos |
| EXIF (dimensões, orientação, data) | F07 |
| Progresso, cancelamento, ETA, execução em segundo plano, comando Tauri | F10/F17. O F05 é uma função síncrona; a F10 a envolve |
| Reconhecer um arquivo movido fora do app (mesmo hash em outro caminho) e reaproveitar o `id` | F10/F48. Aqui ele vira mídia nova + mídia `missing_file` |
| Apagar do banco mídias que sumiram | Nunca automático: favoritos e notas apontam para o `id`. A mídia fica `missing_file` |
| Hash perceptual, duplicatas | F29 |
| Paralelizar o hash / ajuste fino de desempenho | F32. O F05 lê um arquivo por vez, em ordem de caminho |
| Volume ID / letra de unidade trocada | F11 |
| Thumbnails | F12 |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Raiz da biblioteca | `E:\` (constante `LIBRARY_ROOT`); a função recebe a raiz como parâmetro | AD-007, AD-010 | y (AD) |
| Biblioteca no banco | Uma linha em `libraries` por `root_path`; criada na primeira varredura com `name = 'E:'`, `status = 'active'` | O esquema exige `library_id`; uma biblioteca no MVP | n — default |
| O que é mídia | Mesma lista de extensões do F02 (`takeout::classify`), sem diferenciar caixa. `mp4 mov mkv avi 3gp webm m4v` são `video`; o resto da lista é `photo` | Uma só fonte da verdade para "o que é mídia" | n — default |
| Outros arquivos (`.json`, `.zip`, `.html`, …) | Ignorados, sem linha e sem erro | AD-010 | y (AD) |
| Pastas puladas | `.myphotos` na raiz (qualquer caixa), pastas com atributos oculto **e** sistema (`$RECYCLE.BIN`, `System Volume Information`) em qualquer nível, links simbólicos e junções (nem seguidos nem gravados) | AD-010, AD-011; mesmo critério do F02 | y (AD) |
| Árvore | `folders` tem uma linha por pasta física, com o caminho como está no disco; partes `takeout-…-N-NNN` são pastas distintas | AD-010 | y (AD) |
| Caminhos | Relativos à raiz, separador `/`, como no F02 e no F04 | F04 | y |
| Detecção de mudança | Mídia já no banco com o mesmo `size_bytes`, o mesmo `fs_modified_at` e hash presente é "inalterada" e não é lida. Qualquer diferença, ou hash ausente, faz ler e recalcular | Evita reler 4 mil arquivos no HDD a cada abertura | n — default |
| Precisão de data | `fs_modified_at` e `fs_created_at` em ISO-8601 UTC com milissegundos (`2026-10-02T12:00:00.123Z`); `fs_created_at` NULL se o sistema não informar | NTFS guarda 100 ns; segundos inteiros esconderiam uma edição rápida | n — default |
| Mídia que sumiu | `indexing_status = 'missing_file'`, linha e `id` mantidos. Se o arquivo voltar no mesmo caminho, volta a `ok` | Favoritos/notas (F04 DB-05) não podem se perder por um HD mal conectado | n — default |
| Pasta que sumiu | A linha de `folders` é apagada (nada referencia `folders`) | Mostrar a árvore como está no disco (AD-010) | n — default |
| Pasta ilegível | Erro `unreadable_directory` em `indexing_errors`; mídias e pastas já conhecidas dentro dela ficam como estão (não viram `missing_file`) | Falha de leitura não é prova de remoção | n — default |
| Arquivo ilegível | Linha gravada com `indexing_status = 'error'`, `indexing_error = 'unreadable'`, hash NULL; erro em `indexing_errors`; tentado de novo na próxima varredura | RF-014: falha num item não para a varredura | n — default |
| Disco some no meio | A varredura para, a execução fica `failed`, o erro é `root_missing`, e **nenhuma** mídia é marcada `missing_file` nessa execução. O que já foi gravado fica | Disco USB (AD-011) | n — default |
| Gravação | Novas pastas e mídias são inseridas numa transação antes do hash (hash NULL); os hashes são gravados em lotes de até 100 arquivos por transação | Uma queda no meio perde no máximo um lote; a próxima varredura continua de onde parou (hash NULL = ler de novo) | n — default |
| Ordem do hash | Um arquivo por vez, em ordem de caminho | HDD: leitura sequencial é melhor que concorrente; paralelismo fica para a F32 | n — default |
| Execução | Cada varredura cria uma linha em `indexing_runs`: `run_type = 'full'` se a biblioteca nunca teve execução `completed`, senão `'incremental'`; `status` termina `completed` ou `failed` | Esquema F04 | n — default |
| Ferramenta de validação local | Binário `f05-scan` que abre `E:\.myphotos\library.db`, varre `E:\` e imprime o resumo em JSON | O ambiente de desenvolvimento não tem o `E:`; o stakeholder valida no disco real | n — default |

**Open questions:** nenhuma bloqueante. Os defaults acima foram escolhidos sem confirmação ponto a ponto (o stakeholder pediu "go ahead" em 2026-10-05). Qualquer um pode ser revisto antes da F10.

### Dimensões implícitas

| Dimension | Resolução |
| --------- | --------- |
| Input validation & bounds | Raiz relativa → `root_not_absolute`; raiz ausente → `root_missing`; raiz que não é pasta → `root_not_directory`. Nada é gravado no banco nesses casos (SCAN-01) |
| Failure / partial-failure | Arquivo ilegível vira `error` e a varredura segue (SCAN-05). Pasta ilegível vira erro e protege o que está dentro (SCAN-05). Disco que some encerra como `failed` sem marcar remoções (SCAN-06). Falha de escrita no banco → `db_write_failed`, execução `failed` |
| Idempotency / retry | Segunda varredura sem mudança: nenhum arquivo lido, nenhuma linha de mídia alterada além de nada (SCAN-04). Hash ausente é refeito na próxima |
| Auth boundaries & rate limits | N/A: local, um usuário, sem rede |
| Concurrency / ordering | Função síncrona sobre uma `&mut Connection`; o WAL + `busy_timeout` da F04 cobrem um leitor simultâneo. Duas varreduras ao mesmo tempo: N/A (a F10 serializa) |
| Data lifecycle / expiry | Mídia sumida nunca é apagada pelo scanner (fica `missing_file`); pasta sumida é apagada |
| Observability | `indexing_runs` com contadores e `indexing_errors` por item. Resumo devolvido pela função e impresso pelo `f05-scan` |
| External-dependency failure | Disco desconectado antes (SCAN-01) ou durante (SCAN-06) |
| State-transition integrity | `indexing_status`: `ok ↔ missing_file`, `error → ok` quando a leitura volta a funcionar. `indexing_runs.status`: `running → completed | failed` |

---

## User Stories

### P1: Primeira varredura grava pastas e mídias ⭐ MVP

**User Story**: Como usuário, quero que o My Photos encontre todas as fotos e vídeos do `E:\` e guarde uma impressão digital de cada um, para que a biblioteca exista no banco.

**Acceptance Criteria** (SCAN-01, SCAN-02, SCAN-03):

1. WHEN a raiz não é absoluta THEN SHALL retornar `root_not_absolute`; WHEN não existe THEN `root_missing`; WHEN é um arquivo THEN `root_not_directory`; nos três casos nenhuma linha SHALL ser gravada em nenhuma tabela
2. WHEN a primeira varredura termina THEN `libraries` SHALL ter uma linha com `root_path` igual à raiz e `indexing_runs` SHALL ter uma linha `run_type = 'full'`, `status = 'completed'`, `finished_at` preenchido
3. WHEN a árvore tem pastas (inclusive vazias e partes `takeout-…-1-001` e `takeout-…-1-002`) THEN `folders` SHALL ter exatamente uma linha por pasta física, com o caminho relativo `/` como no disco, e nenhuma linha para a raiz
4. WHEN um arquivo tem extensão de mídia do F02 (qualquer caixa) THEN `media_items` SHALL ter uma linha com `relative_path`, `filename`, `extension` em minúsculas, `media_type` (`video` para `mp4 mov mkv avi 3gp webm m4v`, `photo` para o resto), `size_bytes`, `fs_modified_at` ISO-8601 UTC com milissegundos e `Z`, `file_hash_sha256` em hex minúsculo de 64 caracteres igual ao SHA-256 do conteúdo, `indexing_status = 'ok'`
5. WHEN um arquivo não é mídia (`.json`, `.zip`, `.html`, sem extensão) THEN SHALL NOT ter linha em `media_items` nem em `indexing_errors`
6. WHEN existem `.myphotos` na raiz (qualquer caixa), um link simbólico/junção ou uma pasta oculta+sistema THEN nada dentro deles SHALL ter linha em `folders` ou `media_items`
7. WHEN a varredura termina THEN cada arquivo e pasta sob a raiz (fora de `.myphotos`) SHALL ter os mesmos bytes, tamanho e data de modificação de antes, e nenhum arquivo ou pasta SHALL ter sido criado ou apagado fora de `.myphotos`

**Independent Test**: árvore temporária com fotos, vídeos, JSON, `.myphotos`, pasta vazia e duas partes de Takeout; varrer para um banco v1 e conferir as linhas e o hash de um conteúdo conhecido; comparar um retrato da árvore antes e depois.

---

### P1: Varreduras seguintes são incrementais ⭐ MVP

**User Story**: Como usuário, quero reabrir o app sem esperar o HD reler tudo, e ver o que mudou.

**Acceptance Criteria** (SCAN-04):

1. WHEN uma segunda varredura roda sem mudança no disco THEN nenhum arquivo SHALL ser aberto para leitura, nenhuma linha de `media_items` ou `folders` SHALL mudar, e a nova linha de `indexing_runs` SHALL ser `run_type = 'incremental'`
2. WHEN um arquivo de mídia novo aparece THEN SHALL ganhar uma linha nova com hash, e os `id`s existentes SHALL continuar os mesmos
3. WHEN o conteúdo ou o tamanho ou a data de modificação de uma mídia muda THEN o mesmo `id` SHALL ter `size_bytes`, `fs_modified_at`, `file_hash_sha256` e `updated_at` novos, e só esse arquivo SHALL ser lido
4. WHEN uma mídia conhecida não está mais no disco THEN a linha SHALL continuar com o mesmo `id` e `indexing_status = 'missing_file'`; WHEN ela volta no mesmo caminho THEN SHALL voltar a `ok`, mantendo o `id`
5. WHEN uma pasta conhecida não está mais no disco THEN a linha dela em `folders` SHALL ser apagada; WHEN uma pasta nova aparece THEN SHALL ganhar uma linha
6. WHEN uma mídia está no banco com `file_hash_sha256` NULL (varredura anterior interrompida ou arquivo ilegível) THEN SHALL ser lida e ter o hash gravado, mesmo com tamanho e data iguais
7. WHEN a varredura termina THEN `indexing_runs` SHALL ter `files_scanned` = mídias encontradas no disco, `files_indexed` = mídias lidas com sucesso nesta execução e `files_errored` = linhas gravadas em `indexing_errors` nesta execução, e a função SHALL devolver um resumo com as contagens de novas, modificadas, inalteradas, sumidas, reaparecidas e com erro

**Independent Test**: varrer, mudar a árvore (adicionar, editar, apagar, recolocar), varrer de novo com um contador de aberturas de arquivo injetado.

---

### P1: Falhas não param nem corrompem a biblioteca ⭐ MVP

**User Story**: Como usuário com um HD USB antigo, quero que um arquivo ruim ou uma desconexão não estrague o que já foi indexado.

**Acceptance Criteria** (SCAN-05, SCAN-06):

1. WHEN um arquivo de mídia não pode ser lido THEN a varredura SHALL continuar, a linha dele SHALL ter `indexing_status = 'error'`, `indexing_error = 'unreadable'`, hash NULL, e `indexing_errors` SHALL ter uma linha `error_type = 'unreadable'` com o caminho relativo dele
2. WHEN esse arquivo volta a ser legível THEN a próxima varredura SHALL gravar o hash e voltar o status para `ok` com `indexing_error` NULL
3. WHEN uma subpasta não pode ser listada THEN `indexing_errors` SHALL ter `unreadable_directory` com o caminho dela, as mídias e pastas já conhecidas dentro dela SHALL continuar como estavam (status e linhas), e o resto da árvore SHALL ser varrido
4. WHEN um nome de arquivo ou pasta não é Unicode válido THEN SHALL ser pulado com `invalid_name` em `indexing_errors`
5. WHEN a raiz deixa de existir durante a varredura THEN SHALL retornar `root_missing`, a execução SHALL ficar `status = 'failed'` com `finished_at`, nenhuma mídia SHALL ser marcada `missing_file` por causa da desconexão (se a raiz some durante a listagem, nenhuma é marcada nessa execução), e os hashes já gravados SHALL continuar no banco; as mídias que ficaram sem hash SHALL ser lidas na próxima varredura

**Independent Test**: abridor de arquivo injetado que falha para um caminho; listador injetado que falha para uma pasta; abridor que apaga a raiz no meio da execução.

---

## Edge Cases

- WHEN a raiz está vazia THEN a varredura SHALL terminar `completed` com zero pastas e zero mídias
- WHEN uma mídia tem 0 bytes THEN SHALL ter linha com `size_bytes = 0` e o SHA-256 da entrada vazia (`e3b0c442…b855`)
- WHEN `.myphotos` existe numa subpasta (não na raiz) THEN SHALL ser varrida como qualquer pasta
- WHEN o mesmo arquivo é lido e o banco já tem a linha com outro caminho só na caixa (`A.JPG` vs `a.jpg`) THEN a coluna `COLLATE NOCASE` trata como o mesmo caminho e SHALL atualizar a linha existente (NTFS não diferencia caixa)

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| SCAN-01 | P1: Primeira varredura — validação da raiz, biblioteca, execução | Design | Pending |
| SCAN-02 | P1: Primeira varredura — pastas, mídias, hash, o que é pulado | Design | Pending |
| SCAN-03 | P1: Primeira varredura — só leitura na biblioteca | Design | Pending |
| SCAN-04 | P1: Varreduras incrementais | Design | Pending |
| SCAN-05 | P1: Falhas por item | Design | Pending |
| SCAN-06 | P1: Disco some no meio | Design | Pending |
| SCAN-07 | Ferramenta `f05-scan` para validação local no `E:` | Design | Pending |

**Coverage:** 7 total, 0 mapped to tasks, 7 unmapped.

---

## Success Criteria

- [ ] `cargo run --release --bin f05-scan` com o `E:` conectado grava as ~4,2 mil mídias do Takeout + as demais, e uma segunda execução termina sem ler nenhum arquivo (`unchanged` = total)
- [ ] Com o `E:` desconectado, `f05-scan` sai com o código `volume_missing` e nada é criado
- [ ] Todos os critérios acima passam em `cargo test` usando só diretórios temporários
