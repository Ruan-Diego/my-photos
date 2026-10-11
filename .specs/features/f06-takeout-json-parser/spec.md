# F06 — Takeout JSON Parser Specification

## Problem Statement

O F02 já lê o sidecar do Takeout e o `metadata.json` de álbum, mas só o bastante para provar o matcher em memória. Para a F10 gravar o resultado no esquema da F04 faltam peças que o F02 deixou de fora ou adiou: saber **de qual campo** veio o local (`media_locations.source` exige `takeout_geoData` ou `takeout_geoDataExif`), um nome de pessoa normalizado para a chave `UNIQUE (library_id, normalized_name)` de `people`, a data no formato ISO-8601 das colunas, o texto bruto do JSON para `media_metadata_raw.takeout_json`, e as tolerâncias que o F02 adiou para cá (teto de tamanho — design F02, linha "Ler o JSON inteiro sem limite") ou não cobriu (BOM UTF-8, coordenadas fora da faixa, raiz que não é objeto).

## Goals

- [ ] Um sidecar de mídia vira um valor pronto para as colunas da F04: `taken_at` ISO, local com fonte, pessoas com nome normalizado, título/descrição/url limpos e o JSON bruto
- [ ] O `metadata.json` de álbum recebe a mesma normalização de data, local e texto
- [ ] Sidecar enorme, com BOM, com coordenada impossível ou com raiz que não é objeto nunca derruba a pasta nem grava dado inválido
- [ ] Tudo que o F02 já garantia continua garantido (os testes TD do F02 seguem passando, ajustados só onde este spec muda a regra)

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Gravar no banco (`media_items`, `media_locations`, `people`, `media_metadata_raw`) | F10 orquestra e persiste. O F06 devolve valores prontos para as colunas |
| Casar mídia↔JSON | F02 (feito) / F08 |
| Escolher entre JSON, EXIF e sistema de arquivos para `taken_at` | F09 |
| Fuso horário de `taken_at` | O sidecar só traz o instante UTC; `taken_at_timezone` fica NULL até a F07/F09 |
| Tirar acentos do nome normalizado (`José` = `Jose`) | Exigiria tabela Unicode nova; a busca sem acento é da F14. Aqui só caixa e espaços |
| `creationTime` como data de captura | Continua extraído como está (é a data de upload); quem decide usar é a F09 |
| Motion Photos, favoritos do Takeout, `googlePhotosOrigin` | Fora do produto (F02) |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Onde fica | Mesmo módulo `src-tauri/src/takeout/` do F02, ampliando `MediaMetadata` e `AlbumMetadata` com campos novos | F02 design: "F06 e F08 importam daqui em vez de abrir outro crate" | y (F02) |
| Teto de tamanho | 1 MiB (1 048 576 bytes). Acima disso o sidecar é `invalid_json`; a leitura nunca passa de 1 MiB + 1 byte | Sidecars reais têm 1–20 KB; 1 MiB é 50× a margem e limita a memória. `indexing_errors` (F04) não tem um tipo "grande demais" e mudar o `CHECK` exigiria migração | y |
| BOM UTF-8 | Ignorado no início; o JSON bruto guardado não tem BOM | Arquivos regravados por editores do Windows ganham BOM; o `serde_json` recusaria | y |
| Raiz que não é objeto (`[]`, `"x"`, `3`) | `invalid_json` | Não é um sidecar; aceitar viraria um par com "nenhum metadado" que esconde o problema | y |
| Formato de `taken_at` | ISO-8601 UTC com milissegundos e `Z` (`2022-08-10T14:37:00.000Z`), o mesmo de `fs_modified_at` na F05; instantes antes de 1970 suportados | Um formato só no banco ordena certo como texto | y |
| Timestamp `0` | Ausente | O Takeout escreve `0` quando não sabe; 1970-01-01T00:00:00Z nunca é a data real de uma foto. Negativos (antes de 1970, foto escaneada) valem | y |
| Faixa do instante | Ano entre 1800 e 9999; fora disso, ausente | Evita data absurda que quebraria a ordenação ISO (ano com 5 dígitos ou negativo) | y |
| Coordenada válida | Latitude em [-90, 90], longitude em [-180, 180], ambas números finitos e não as duas `0.0`. Inválida conta como ausente e a fonte seguinte (`geoDataExif`) é tentada | `0.0/0.0` já era ausência no F02; fora da faixa é dado corrompido | y |
| Altitude | Só com ponto válido; número finito ou ausente | F02 já descartava altitude junto do ponto | y (F02) |
| Fonte do local | `takeout_geoData` se `geoData` válido, senão `takeout_geoDataExif` se `geoDataExif` válido, senão nenhum local | Valores do `CHECK` de `media_locations.source` (F04) | y (F04) |
| Nome de pessoa | Exibição: espaços das pontas removidos e espaços internos (qualquer espaço Unicode) colapsados em um. Normalizado: exibição em minúsculas Unicode. Nome vazio depois disso é descartado. Repetido pelo normalizado no mesmo JSON: fica o primeiro | `Ana  Maria`, `ana maria` e ` Ana Maria ` são a mesma pessoa para `people` | y |
| Texto (título, descrição, url) | Pontas sem espaço; vazio ou só espaço → ausente. Quebras de linha internas da descrição mantidas | F02 já tratava vazio como ausente; espaço sobrando é ruído do Takeout | y |
| JSON bruto | O texto UTF-8 do arquivo, sem BOM, exatamente como lido (não re-serializado) | `media_metadata_raw.takeout_json` deve permitir reprocessar sem reler o disco | y |

**Open questions:** nenhuma. Defaults confirmados pelo stakeholder em 2026-10-06 ("continue with defaults").

### Dimensões implícitas

| Dimension | Resolução |
| --------- | --------- |
| Input validation & bounds | Tamanho (JSON-05), raiz, BOM, faixa de coordenadas e de instante (JSON-01, JSON-02, JSON-05) |
| Failure / partial-failure | Sidecar inválido vira `invalid_json` só dele; a mídia fica sem par e a pasta segue (regra do F02, mantida) |
| Idempotency / retry | Função pura sobre bytes: mesmos bytes, mesmo resultado |
| Auth boundaries & rate limits | N/A: local, sem rede |
| Concurrency / ordering | N/A: função pura. Ordem das pessoas = ordem do JSON |
| Data lifecycle / expiry | N/A: nada é persistido aqui |
| Observability | Problemas continuam no relatório do F02 (`invalid_json`, `unreadable`) |
| External-dependency failure | Leitura limitada do arquivo; erro de leitura continua `unreadable` |

---

## User Stories

### P1: Sidecar de mídia pronto para o banco ⭐ MVP

**User Story**: Como o pipeline (F10), quero receber de cada sidecar valores já no formato das colunas, para gravar sem regra extra.

**Acceptance Criteria** (JSON-01, JSON-02, JSON-03, JSON-04):

1. WHEN `photoTakenTime.timestamp` é `"1660142220"` ou `1660142220` THEN `taken_at_unix` SHALL ser `1660142220` e `taken_at` SHALL ser `"2022-08-10T14:37:00.000Z"`; o mesmo vale para `creationTime` → `creation_time_unix`/`creation_time`
2. WHEN o timestamp é `0`, não numérico, ou fora dos anos 1800–9999 THEN o par (`*_unix`, ISO) SHALL ser ausente e o resto do sidecar SHALL ser lido; WHEN é negativo dentro da faixa (ex.: `-86400`) THEN SHALL valer (`"1969-12-31T00:00:00.000Z"`)
3. WHEN `geoData` é válido THEN o local SHALL vir dele com fonte `takeout_geoData`; WHEN `geoData` falta ou é inválido (`0/0`, latitude fora de [-90, 90], longitude fora de [-180, 180], não numérico) e `geoDataExif` é válido THEN SHALL vir de `geoDataExif` com fonte `takeout_geoDataExif`; WHEN nenhum é válido THEN latitude, longitude, altitude e fonte SHALL ser ausentes
4. WHEN o ponto é válido e `altitude` é número THEN SHALL ser copiada; senão ausente
5. WHEN `people` tem `" Ana  Maria "`, `"ana maria"`, `""`, `"   "` e `"Bruno"` THEN `people` SHALL ser `["Ana Maria", "Bruno"]` e o normalizado de cada um SHALL ser `"ana maria"` e `"bruno"`
6. WHEN `title`, `description` ou `url` tem espaços nas pontas THEN SHALL vir sem eles; WHEN fica vazio THEN ausente; quebras de linha internas da descrição SHALL ser mantidas
7. WHEN o sidecar é lido THEN `raw_json` SHALL ser o texto do arquivo, byte a byte, sem o BOM inicial se houver

**Independent Test**: chamar `parse_media_json` com bytes literais para cada critério.

---

### P1: Álbum normalizado ⭐ MVP

**User Story**: Como a F25, quero o `metadata.json` de álbum com as mesmas regras de data, local e texto.

**Acceptance Criteria** (JSON-04):

1. WHEN `date.timestamp` é válido THEN `date_unix` e `date` (ISO) SHALL ser preenchidos com as regras de JSON-01
2. WHEN `geoData` do álbum é válido THEN latitude/longitude/altitude SHALL ser copiadas; inválido → ausentes (álbum não tem `geoDataExif`)
3. WHEN `title`/`description` têm espaços nas pontas ou são vazios THEN SHALL seguir a regra de texto de JSON-03

**Independent Test**: `parse_album_json` com bytes literais.

---

### P1: Sidecar hostil não derruba nada ⭐ MVP

**User Story**: Como usuário com um export antigo e arquivos mexidos, quero que um JSON estranho só fique de fora, sem travar a indexação.

**Acceptance Criteria** (JSON-05, JSON-06):

1. WHEN o arquivo tem mais de 1 048 576 bytes THEN `parse_*` SHALL falhar (o descobridor registra `invalid_json`) e o leitor SHALL NOT ler mais que 1 048 577 bytes
2. WHEN o arquivo tem exatamente 1 048 576 bytes de JSON válido THEN SHALL ser aceito
3. WHEN o arquivo começa com o BOM UTF-8 (`EF BB BF`) THEN SHALL ser lido como se não tivesse
4. WHEN a raiz do JSON não é objeto THEN SHALL falhar como JSON inválido
5. WHEN `discover` encontra um sidecar maior que o teto ou com raiz não-objeto THEN o relatório SHALL ter `invalid_json` para ele, a mídia SHALL ficar sem par, e os vizinhos SHALL ser casados normalmente (JSON-06)

**Independent Test**: bytes gerados em memória; uma árvore temporária com um sidecar de 1 MiB + 1 e um leitor que conta bytes.

---

## Edge Cases

- WHEN o mesmo nome aparece com caixa diferente (`"BRUNO"`, `"Bruno"`) THEN fica o primeiro (`"BRUNO"`) com normalizado `"bruno"`
- WHEN `people` não é lista ou um item não tem `name` texto THEN o item SHALL ser ignorado (regra do F02, mantida)
- WHEN o timestamp é string com espaços ou `+` (`" 1660142220"`, `"+1"`) THEN SHALL ser ausente (regra estrita do F02, mantida)
- WHEN a latitude é exatamente 90 ou -90, ou a longitude 180 ou -180 THEN SHALL ser válida

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| JSON-01 | Datas ISO, `0` e faixa | T1–T3 | Verified |
| JSON-02 | Local com fonte e validação | T1–T3 | Verified |
| JSON-03 | Pessoas e texto normalizados | T1–T3 | Verified |
| JSON-04 | JSON bruto e álbum | T1–T3 | Verified |
| JSON-05 | Teto, BOM, raiz | T1–T3 | Verified |
| JSON-06 | `discover` usa o leitor limitado e o parser novo | T1–T3 | Verified |

**Coverage:** 6 total, 6 mapped to tasks, 0 unmapped. Verified in `validation.md` (PASS, iteration 3).

---

## Success Criteria

- [x] Todos os critérios passam em `cargo test` só com bytes em memória e diretórios temporários
- [x] Os testes TD do F02 continuam passando (ajustados só para os campos novos)
