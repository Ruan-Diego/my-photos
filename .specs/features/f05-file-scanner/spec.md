# F05 — File Scanner Specification

## Problem Statement

A F04 entregou o banco vazio em `E:\.myphotos\library.db` e a F02 já casa mídia e JSON em memória. Falta quem leve o disco para o banco: varrer `E:\`, registrar pastas e mídias com tamanho, datas do sistema de arquivos e SHA-256, e, numa segunda varredura, perceber o que é novo, o que mudou, o que foi movido e o que sumiu — sem recalcular hash do que não mudou (US-07).

## Goals

- [x] Uma função `scanner::scan(conexão, raiz)` grava `folders`, `media_items`, `indexing_runs` e `indexing_errors` a partir do disco
- [x] Re-varrer uma biblioteca sem mudanças não lê o conteúdo de nenhum arquivo
- [x] Uma foto movida ou renomeada por fora do app mantém o `id` (favoritos, notas e álbuns continuam ligados)
- [x] Nenhum teste lê ou escreve em `E:\`

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Data, local, pessoas, título vindos do JSON ou EXIF (`taken_at`, `media_locations`, `media_people`) | F06/F07/F09. A F05 grava só o vínculo com o JSON (`has_takeout_json`, `json_relative_path`, `json_match_confidence`) |
| Orquestração, progresso, cancelamento, comando Tauri e botão na UI | F10 e F17. A F05 é uma função de biblioteca |
| Hash em paralelo | O baseline da F03 mediu HDD USB; leitura sequencial é o padrão seguro. F10 pode paralelizar |
| Álbuns (`metadata.json`) | F25 |
| Volume ID / letra de unidade | F11 |
| Duplicatas | P2. O hash já fica gravado em `idx_media_hash` |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Quem varre | `takeout::discover` (F02), estendido para ignorar `.myphotos` na raiz e listar todas as pastas | Reaproveita o casamento entre partes do Takeout (AD-010) | y |
| Pasta do app | `<raiz>/.myphotos` é pulada só na raiz, sem diferenciar caixa. Uma `.myphotos` mais funda é pasta comum | AD-011 | y |
| Biblioteca | Uma linha em `libraries` por `root_path`, criada na primeira varredura | O esquema já prevê várias; hoje só existe `E:\` | y |
| Mudou? | Tamanho **ou** `fs_modified_at` (segundos, UTC) diferentes, ou linha sem hash / com status diferente de `ok` → recalcula o hash | Padrão de indexadores; ler o arquivo inteiro a cada varredura custaria horas no HDD | y |
| Arquivo sumiu | `indexing_status = 'missing_file'`, a linha **não** é apagada | Apagar levaria favoritos, notas e álbuns em cascata (F04). Se o arquivo voltar, a mesma linha volta a `ok` | y |
| Movido por fora do app | Arquivo novo com mesmo tamanho e SHA-256 de uma linha sumida herda o `id` dela | AD-008: identidade estável ao mover | y |
| Cópia | Arquivo novo igual a um que **continua** no disco é mídia nova | Duplicata é P2, não movimento | y |
| Arquivo ilegível | Linha com `indexing_status = 'error'` e a mensagem; se já existia, mantém o hash antigo. Registro `unreadable` em `indexing_errors`. A varredura continua | US-08: falha isolada não para a indexação | y |
| Pasta ilegível | Mídias e pastas abaixo dela **não** viram `missing_file` nem são apagadas | Um erro de leitura no USB não pode parecer exclusão | y |
| Problemas da F02 | JSON inválido, órfão, ilegível, nome inválido e pasta ilegível viram linhas em `indexing_errors` com o tipo da F02 | Os tipos já existem no `CHECK` da F04 | y |
| Pastas | Toda pasta no disco (inclusive vazia) tem linha em `folders`; linha de pasta que sumiu é apagada | Pasta não tem dados do usuário ligados | y |
| Escrita | Hash fora da transação; todas as escritas de uma varredura numa transação só. Falha de banco → run `failed`, nada gravado | Atômico e simples para ~4 mil mídias | y |
| Tipo de mídia | `video` para `mp4 mov mkv avi 3gp webm m4v`, senão `photo` | Mesma lista da F02 | y |

**Open questions:** nenhuma. Premissas confirmadas pelo stakeholder em 2026-10-11.

---

## User Stories

### P1: Primeira varredura ⭐ MVP

**Acceptance Criteria** (SC-01):

1. WHEN a biblioteca é varrida pela primeira vez THEN SHALL criar a linha em `libraries`, uma linha por mídia com `filename`, `extension` (minúscula), `media_type`, `size_bytes`, `file_hash_sha256`, `fs_created_at`/`fs_modified_at` (ISO UTC com `Z`) e o vínculo com o JSON, e um `indexing_runs` `full` `completed` com as contagens
2. WHEN existe pasta vazia THEN SHALL existir linha em `folders` para ela
3. WHEN existe `<raiz>/.myphotos` THEN nada dentro dela SHALL virar mídia ou pasta

### P1: Varredura incremental ⭐ MVP

**Acceptance Criteria** (SC-02..SC-05):

1. WHEN nada mudou THEN nenhum arquivo SHALL ser lido para hash, os `id` SHALL continuar os mesmos e o run SHALL ser `incremental` (SC-02)
2. WHEN tamanho ou data de modificação mudam THEN o hash SHALL ser recalculado na mesma linha (SC-03)
3. WHEN o arquivo some THEN a linha SHALL virar `missing_file` uma vez; WHEN volta THEN SHALL voltar a `ok` com o mesmo `id` e os favoritos (SC-04)
4. WHEN o arquivo é movido ou renomeado (inclusive só a caixa) THEN SHALL manter o `id` com o caminho novo; uma cópia SHALL ser mídia nova (SC-05)

### P1: Falhas isoladas ⭐ MVP

**Acceptance Criteria** (SC-06..SC-08):

1. WHEN um arquivo não pode ser lido THEN SHALL ficar com status `error`, um registro `unreadable` SHALL existir e as demais mídias SHALL ser gravadas; a próxima varredura que o ler SHALL voltar a `ok` (SC-06)
2. WHEN a F02 reporta problemas THEN cada um SHALL virar uma linha de `indexing_errors` do run; WHEN um JSON é adicionado depois THEN a mídia inalterada SHALL ganhar o vínculo sem novo hash (SC-07)
3. WHEN a raiz não existe THEN SHALL retornar `root_unreadable` sem gravar nada; WHEN uma pasta some THEN a linha dela SHALL ser apagada (SC-08)

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| SC-01 | Primeira varredura | T1–T4 | Implemented |
| SC-02 | Incremental sem leitura | T3–T4 | Implemented |
| SC-03 | Arquivo alterado | T3–T4 | Implemented |
| SC-04 | Arquivo sumiu / voltou | T3–T4 | Implemented |
| SC-05 | Movido / renomeado / cópia | T3–T4 | Implemented |
| SC-06 | Arquivo ilegível | T3–T4 | Implemented |
| SC-07 | Problemas da F02 e JSON tardio | T3–T4 | Implemented |
| SC-08 | Raiz ausente, pasta removida | T3–T4 | Implemented |

## Success Criteria

- [ ] `cargo test` verde no Windows (CI)
- [ ] Varredura real de `E:\` pela F10 grava as 4.247 mídias do Google Fotos mais as 1.885 de fora
