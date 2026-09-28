# F02 — Takeout Discovery Specification

## Problem Statement

O Google Takeout não entrega um par mídia↔JSON estável. O mesmo export usa `arquivo.jpg.json`, `arquivo.json`, `arquivo.jpg.supplemental-metadata.json`, nomes truncados, `-edited` sem JSON próprio e `metadata.json` de álbum no meio dos sidecars. Sem um matcher e um parser que acertem essas variações em fixtures, a indexação (F06, F08, F09) vai gravar data e local errados — ou nenhum.

## Goals

- [x] Dada uma árvore de pastas no formato Takeout, o My Photos associa cada mídia ao JSON da mesma pasta, com confiança `exact`, `heuristic` ou `none`
- [x] O JSON associado produz data de captura, local, pessoas, título, descrição e url conforme a seção 9.2 do PRD, com `0.0/0.0` tratado como ausência de local
- [x] `metadata.json` vira metadado de álbum e nunca sidecar de mídia
- [x] JSON inválido ou arquivo ilegível não interrompe o restante da pasta, e nenhum arquivo da árvore é criado, alterado ou apagado

## Out of Scope

Explicitamente fora. Documentado para impedir escopo extra.

| Feature | Reason |
| ------- | ------ |
| Varredura da biblioteca `E:\` (AD-007, F05) | F02 prova o formato em fixtures do repositório. A pasta real do usuário não entra no teste nem no critério de acerto |
| Esquema SQLite, migrações, gravação de `takeout_json` (F04) | O resultado desta feature é em memória, consumido pelos testes |
| EXIF, filesystem timestamp e a hierarquia completa de `taken_at` (F07, F09) | Sem JSON, a descoberta para em `none`. Quem escolhe a próxima fonte é o resolver |
| Pipeline, progresso, indexação incremental, hash (F05, F10, F29) | Duplicata física em pasta do ano e pasta do álbum é esperada e não é erro. Agrupar por hash fica fora |
| Thumbnails, UI, painel de diagnóstico (F12, F15, F28) | Phase 0 não tem tela. O relatório é o valor devolvido pela descoberta |
| Álbuns na UI (F25) | F02 só reconhece e interpreta `metadata.json`. Não cria álbum persistido |
| Motion Photos, favoritos do Takeout, `googlePhotosOrigin` | A seção 9.2 do PRD não usa esses campos no produto. Não entram na extração |
| Medir >98% contra a biblioteca pessoal | O critério de Phase 0 sobre biblioteca real vale para F08/F10, quando houver varredura. Aqui o critério é 100% do conjunto rotulado de fixtures |

---

## Assumptions & Open Questions

Cada ambiguidade está resolvida ou registrada aqui.

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Onde vivem as fixtures | Árvore commitada no repositório, com JSON realista e arquivos de mídia vazios. Nenhum acesso a `E:\` | AD-004 e AD-007. Teste tem de passar sem a biblioteca pessoal e sem gravar foto de usuário no git | y |
| Limite de truncamento do sidecar | 51 bytes UTF-8 no nome do arquivo, preservando o sufixo `.json` | O PRD diz "~47+ caracteres". Medidas públicas de Takeout (gpth, 2024–2026) fixam 51 bytes com `.json` intacto. É o limite que gera `.supplemental-meta.json` e o exemplo truncado | y |
| O que é `exact` | Só os passos 1 e 2 da cascata, sem truncar: `arquivo.ext.json` e `arquivo.json`, comparação sem considerar maiúsculas | O PRD chama esses dois de correspondência exata e sufixo alternativo, antes das heurísticas | y |
| O que é `heuristic` | Supplemental (inteiro ou truncado), nome truncado, sufixo `(N)` trocado de lugar, e `-edited` reusando o JSON do original | São os passos 3 a 5 da seção 9.4, mais a divergência de caixa já absorvida na comparação | y |
| Empate | Se dois candidatos empatam no mesmo passo, ou dois arquivos de mídia (que não sejam par original/`-edited`) disputam o mesmo JSON, os dois ficam `none` com motivo `ambiguous` | Escolher "o maior prefixo" no escuro associa a foto errada. A seção 9.4 não manda adivinhar | y |
| `-edited` compartilha o JSON | `nome-edited.ext` pode apontar para o mesmo JSON já usado por `nome.ext`. A confiança do editado é sempre `heuristic` | O PRD diz que o editado não tem JSON próprio e que o metadado está no original | y |
| Marcador `(N)` | `nome(N).ext` também aceita sidecar `nome.ext(N).json` e `nome.ext.supplemental-metadata(N).json` (o `(N)` entra depois do truncamento e o nome pode passar de 51 bytes) | Padrão observado no Takeout quando o mesmo nome se repete. O PRD avisa que a numeração não coincide | y |
| Timestamp | `photoTakenTime.timestamp` e `creationTime.timestamp` aceitam string decimal ou inteiro JSON. Qualquer outro formato deixa o campo ausente, sem falhar o arquivo | O exemplo do PRD usa string. Inteiro cru aparece em exports antigos e ainda é um instante inequívoco | y |
| Extensões de mídia | jpg, jpeg, png, heic, heif, gif, webp, dng, cr2, nef, arw, mp4, mov, mkv, avi, 3gp, webm, m4v | Lista do MVP na seção 18 do PRD. O matcher precisa saber o que é mídia para não tratar JSON como foto | y |

**Open questions:** nenhuma. Spec confirmada pelo stakeholder em 2026-09-28, com os defaults acima.

### Dimensões implícitas

| Dimension | Resolução |
| --------- | --------- |
| Input validation & bounds | WHEN o nome não bate em nenhum passo da cascata, a extensão não está na lista, ou um campo tem tipo inutilizável THEN a descoberta SHALL seguir as regras normativas abaixo, sem lançar erro da pasta inteira |
| Failure / partial-failure | WHEN um JSON é ilegível ou não é JSON THEN só aquele arquivo entra no relatório de problemas e os demais SHALL ser descobertos |
| Idempotency / retry | WHEN a mesma árvore é descoberta de novo THEN o resultado SHALL ser igual. Nenhum JSON é consumido nem reservado em disco |
| Auth boundaries & rate limits | N/A porque o uso é local, sem conta, sem rede e sem chamada externa |
| Concurrency / ordering | N/A porque uma descoberta lê um snapshot de diretório numa única passagem. Ordem de pessoas no JSON é preservada; ordem entre arquivos no relatório é a ordem lexicográfica do caminho relativo |
| Data lifecycle / expiry | N/A porque nada é persistido. O resultado morre com a chamada |
| Observability | N/A para log e telemetria. O relatório em memória é o único registro. F28 define painel e pacote de diagnóstico. Nomes de pessoas e coordenadas existem no resultado porque são o dado extraído, não um log |
| External-dependency failure | A dependência é o sistema de arquivos local. Falha de leitura de um arquivo é `unreadable` e não aborta a descoberta |
| State-transition integrity | A confiança de cada mídia é exatamente uma entre `exact`, `heuristic` e `none`. Não há estado anterior para transicionar |

---

## Regras normativas

Aplicam-se a uma árvore de diretórios. A associação acontece só entre arquivos da mesma pasta. Comparação de nome de arquivo ignora maiúsculas e minúsculas ASCII.

**Extensão de mídia.** Sem o ponto, em minúsculas, está na lista da tabela de assumptions. Qualquer outro arquivo que não seja JSON nem `archive_browser.html` é ignorado: não é mídia, não é problema.

**Álbum.** Arquivo cujo nome, sem considerar maiúsculas, é `metadata.json` é metadado de álbum. Não é candidato a sidecar.

**Truncamento.** Para um nome completo `S` em bytes UTF-8: se o tamanho é no máximo 51, o sidecar esperado é `S`. Se é maior, o sidecar esperado é os primeiros 46 bytes de `S` seguidos de `.json`. O corte só é válido se cair em fronteira de caractere UTF-8; se não cair, a descoberta não inventa outro corte e aquele candidato não conta.

**Cascata, por arquivo de mídia, parando no primeiro passo com um único candidato.** Cada mídia vê sozinha os `.json` da mesma pasta, exceto `metadata.json`. Uma mídia não tira o JSON das outras durante a cascata.

1. `exact`: existe arquivo cujo nome é `{nome}{ext}.json` ou `{nome}.json`, sem truncar. `{nome}{ext}` é o nome da mídia.
2. `heuristic`: existe arquivo cujo nome é o truncamento de `{nome}{ext}.supplemental-metadata.json` ou de `{nome}.supplemental-metadata.json`.
3. `heuristic`: existe arquivo cujo nome é o truncamento de `{nome}{ext}.json` ou de `{nome}.json`, usado só quando o passo 1 não achou o nome inteiro.
4. `heuristic`, só se o nome da mídia, sem extensão, termina em `(N)` com `N` inteiro positivo sem sinal: o sidecar `base.ext(N).json` ou `base.ext.supplemental-metadata(N).json`, em que `base` é o nome sem o `(N)`. O `(N)` é acrescentado depois do truncamento da parte anterior a ele, e o nome resultante pode ter mais de 51 bytes.
5. `heuristic`, só se o nome sem extensão termina em `-edited`: repetir os passos 1 a 4 procurando o JSON de `{nome-sem-edited}{ext}`. A confiança permanece `heuristic`. Esse passo só vale depois que o arquivo original, se existir na pasta, ficou com esse JSON. Se o original ficou `ambiguous` ou `none`, o editado fica `none`.

Empate em qualquer passo: a mídia fica `none`, motivo `ambiguous`, e nenhum dos JSON empatados é escolhido.

Depois da cascata: se duas mídias que não são o par `{nome}{ext}` e `{nome}-edited{ext}` ficaram com o mesmo JSON, as duas passam a `none` com motivo `ambiguous`. O editado que só compartilhava esse JSON também fica `none`. O JSON, sem mídia associada, entra como `orphan_json`.

**JSON de mídia associado.** Campos ausentes, vazios ou de tipo inutilizável ficam ausentes no resultado. O restante do JSON ainda vale.

- `taken_at_unix` vem só de `photoTakenTime.timestamp`. `creationTime.timestamp` vai para `creation_time_unix` e nunca para `taken_at_unix`.
- Local vem de `geoData`. Se `latitude` e `longitude` existem e são ambas exatamente `0.0`, o local está ausente. Se `geoData` tem local ausente, usar `geoDataExif` com a mesma regra. `altitude` só acompanha um local presente.
- `people` é a lista de `people[].name` não vazios, na ordem do JSON.
- `title`, `description` e `url` só entram se forem texto não vazio.
- Campos desconhecidos são ignorados.

**JSON de álbum.** `title` e `description` com a mesma regra de vazio. `date.timestamp` vira `date_unix` com a mesma regra de timestamp. Local com a mesma regra de `0.0/0.0`.

---

## User Stories

### P1: Associar mídia ao JSON da mesma pasta ⭐ MVP

**User Story**: Como desenvolvedor do My Photos, quero que cada arquivo de mídia encontre o JSON do Takeout na mesma pasta, para as variações da seção 9.4 deixarem de ser surpresa na indexação.

**Why P1**: RF-003. Sem o par certo, data e pessoas vêm do arquivo errado.

**Acceptance Criteria**:

1. WHEN a pasta contém `IMG_20220810_143700.jpg` e `IMG_20220810_143700.jpg.json` THEN a descoberta SHALL associar os dois com confiança `exact` e com o caminho relativo do JSON.
2. WHEN a mídia é `IMG_20220810_143700.JPG` e o JSON é `IMG_20220810_143700.jpg.json` THEN a descoberta SHALL associar os dois com confiança `exact`.
3. WHEN a pasta contém `foto.jpg` e só `foto.json` THEN a descoberta SHALL associar os dois com confiança `exact`.
4. WHEN a pasta contém `foto.jpg` e `foto.jpg.supplemental-metadata.json` THEN a descoberta SHALL associar os dois com confiança `heuristic`.
5. WHEN a mídia é `foto_com_nome_longo_demais_para_o_takeout_001.jpg` (49 bytes) e o único JSON da pasta é `foto_com_nome_longo_demais_para_o_takeout_001..json` (51 bytes, truncamento de `…001.jpg.json`) THEN a descoberta SHALL associar os dois com confiança `heuristic`.
6. WHEN a mídia é `camphoto_1271212614(1).jpg` e o JSON é `camphoto_1271212614.jpg.supplemental-metadata(1).json` THEN a descoberta SHALL associar os dois com confiança `heuristic`.
7. WHEN a pasta contém `foto.jpg`, `foto.jpg.json` e `foto-edited.jpg` THEN `foto.jpg` SHALL ficar `exact` nesse JSON e `foto-edited.jpg` SHALL ficar `heuristic` nesse mesmo JSON.
8. WHEN dois arquivos de mídia que não são par original/`-edited` têm o mesmo único JSON truncado como candidato THEN os dois SHALL ficar `none` com motivo `ambiguous`, e o JSON SHALL NOT ficar associado.
9. WHEN o JSON correspondente está na pasta pai e a pasta da mídia não tem sidecar THEN a mídia SHALL ficar `none`.
10. WHEN a mesma árvore é descoberta duas vezes THEN as associações SHALL ser iguais, na ordem lexicográfica do caminho relativo da mídia.

**Independent Test**: `cargo test` sobre uma fixture com os dez layouts acima, sem UI e sem banco.

---

### P1: Extrair os campos do JSON de mídia ⭐ MVP

**User Story**: Como desenvolvedor do My Photos, quero os campos da seção 9.2 já normalizados, para o resolver e a timeline não reinterpretarem o JSON do Google.

**Why P1**: RF-004. O exemplo do PRD tem data, geo nulo disfarçado de zero, pessoas e url.

**Acceptance Criteria**:

1. WHEN o JSON associado tem `photoTakenTime.timestamp` igual a `"1660142220"` e `creationTime.timestamp` igual a `"1660145820"` THEN `taken_at_unix` SHALL ser `1660142220` e `creation_time_unix` SHALL ser `1660145820`.
2. WHEN `photoTakenTime` está ausente e `creationTime.timestamp` está presente THEN `taken_at_unix` SHALL estar ausente e `creation_time_unix` SHALL receber esse timestamp.
3. WHEN `geoData.latitude` e `geoData.longitude` são ambas `0.0` THEN latitude, longitude e altitude SHALL estar ausentes, mesmo se `altitude` for `0.0`.
4. WHEN `geoData` é `0.0/0.0` e `geoDataExif` tem latitude `-23.5505` e longitude `-46.6333` THEN o resultado SHALL usar essas coordenadas e a altitude de `geoDataExif`.
5. WHEN `geoData` tem latitude `-23.5505` e longitude `-46.6333` e `geoDataExif` tem outras coordenadas THEN o resultado SHALL usar só `geoData`.
6. WHEN `people` é `[{ "name": "Pessoa A" }, { "name": "" }, { "name": "Pessoa B" }]` THEN a lista SHALL ser exatamente `Pessoa A`, depois `Pessoa B`.
7. WHEN `title`, `description` ou `url` é `""` ou está ausente THEN esse campo SHALL estar ausente. WHEN `url` é `"https://photos.google.com/photo/x"` THEN o resultado SHALL conter exatamente esse texto e a descoberta SHALL concluir sem rede.
8. WHEN `photoTakenTime.timestamp` é `"ontem"` THEN `taken_at_unix` SHALL estar ausente e a associação SHALL permanecer.

**Independent Test**: `cargo test` com um JSON mínimo por critério, associado por correspondência exata.

---

### P1: Separar o metadata.json de álbum ⭐ MVP

**User Story**: Como desenvolvedor do My Photos, quero que o `metadata.json` do álbum não seja lido como sidecar de uma foto, para o título do álbum não virar título de mídia.

**Why P1**: A seção 9.1 coloca `metadata.json` na mesma pasta das fotos. A seção 9.3 descreve outro formato.

**Acceptance Criteria**:

1. WHEN a pasta `Viagem Europa 2022` contém `metadata.json`, `foto001.jpg` e `foto001.jpg.json` THEN `foto001.jpg` SHALL associar a `foto001.jpg.json` com confiança `exact`, e `metadata.json` SHALL NOT ser o JSON dessa mídia.
2. WHEN `metadata.json` tem `title` `"Viagem Europa 2022"`, `description` `""`, `date.timestamp` `"1660000000"` e `geoData` `0.0/0.0` THEN o álbum SHALL ter título `Viagem Europa 2022`, descrição ausente, `date_unix` `1660000000` e local ausente.
3. WHEN a pasta `Photos from 2022` não tem `metadata.json` THEN a descoberta SHALL NOT criar registro de álbum para ela.
4. WHEN `metadata.json` não é JSON válido THEN a descoberta SHALL registrar o caminho relativo com motivo `invalid_json` e SHALL continuar a associar as mídias da pasta.

**Independent Test**: `cargo test` com uma pasta de álbum e uma pasta `Photos from 2022` sem `metadata.json`.

---

### P1: Falhar por arquivo, sem tocar nos originais ⭐ MVP

**User Story**: Como desenvolvedor do My Photos, quero que um JSON quebrado ou um arquivo ilegível pare só naquele item, e que a descoberta não escreva na árvore, para a prova do Takeout respeitar a não destrutividade.

**Why P1**: AD-004. O PRD (US-08, RF-063) exige que um JSON corrompido não derrube o lote. O relatório de UI é F28; aqui basta o resultado por arquivo.

**Acceptance Criteria**:

1. WHEN `foto.jpg.json` não é JSON válido e a pasta também tem `outra.jpg` com `outra.jpg.json` válido THEN `foto.jpg` SHALL ficar `none`, o caminho de `foto.jpg.json` SHALL constar com motivo `invalid_json`, e `outra.jpg` SHALL ficar `exact`.
2. WHEN um arquivo `.json` da pasta não abre para leitura THEN a descoberta SHALL registrá-lo com motivo `unreadable` e SHALL devolver as demais mídias da pasta.
3. WHEN a pasta contém `archive_browser.html` THEN esse arquivo SHALL NOT aparecer como mídia, álbum, sidecar nem problema.
4. WHEN existe `notas.txt` na pasta THEN esse arquivo SHALL NOT aparecer no resultado.
5. WHEN a descoberta termina THEN o conjunto de caminhos, tamanhos e conteúdos dos arquivos da árvore SHALL ser idêntico ao de antes da chamada. A descoberta SHALL NOT criar, renomear, apagar nem escrever arquivo.

**Independent Test**: `cargo test` compara um snapshot dos bytes da fixture antes e depois, com um JSON inválido ao lado de um par válido.

---

## Edge Cases

- WHEN a mídia não tem JSON em nenhum passo THEN a confiança SHALL ser `none`, sem motivo `invalid_json`. O item permanece no resultado.
- WHEN um `.json` da pasta não é `metadata.json` e nenhum passo o associa THEN ele SHALL aparecer com motivo `orphan_json`.
- WHEN `people` está ausente THEN a lista de pessoas SHALL ser vazia.
- WHEN o JSON tem um campo extra não descrito no PRD THEN a associação e os campos conhecidos SHALL permanecer, e o campo extra SHALL NOT aparecer no resultado.
- WHEN o nome da mídia termina em `-edited` e o original não tem JSON THEN o editado SHALL ficar `none`.

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| TD-01 | P1: Associar mídia ao JSON | Tasks | Done |
| TD-02 | P1: Associar mídia ao JSON | Tasks | Done |
| TD-03 | P1: Associar mídia ao JSON | Tasks | Done |
| TD-04 | P1: Associar mídia ao JSON | Tasks | Done |
| TD-05 | P1: Associar mídia ao JSON | Tasks | Done |
| TD-06 | P1: Associar mídia ao JSON | Tasks | Done |
| TD-07 | P1: Associar mídia ao JSON | Tasks | Done |
| TD-08 | P1: Associar mídia ao JSON | Tasks | Done |
| TD-09 | P1: Associar mídia ao JSON | Tasks | Done |
| TD-10 | P1: Associar mídia ao JSON | Tasks | Done |
| TD-11 | P1: Extrair campos do JSON | Tasks | Done |
| TD-12 | P1: Extrair campos do JSON | Tasks | Done |
| TD-13 | P1: Extrair campos do JSON | Tasks | Done |
| TD-14 | P1: Extrair campos do JSON | Tasks | Done |
| TD-15 | P1: Extrair campos do JSON | Tasks | Done |
| TD-16 | P1: Extrair campos do JSON | Tasks | Done |
| TD-17 | P1: Extrair campos do JSON | Tasks | Done |
| TD-18 | P1: Extrair campos do JSON | Tasks | Done |
| TD-19 | P1: Separar metadata.json de álbum | Tasks | Done |
| TD-20 | P1: Separar metadata.json de álbum | Tasks | Done |
| TD-21 | P1: Separar metadata.json de álbum | Tasks | Done |
| TD-22 | P1: Separar metadata.json de álbum | Tasks | Done |
| TD-23 | P1: Falhar por arquivo | Tasks | Done |
| TD-24 | P1: Falhar por arquivo | Tasks | Done |
| TD-25 | P1: Falhar por arquivo | Tasks | Done |
| TD-26 | P1: Falhar por arquivo | Tasks | Done |
| TD-27 | P1: Falhar por arquivo | Tasks | Done |

Os critérios de borda (`none` sem erro, `orphan_json`, `people` vazio, campo extra, `-edited` sem original) são consequência das regras normativas e dos critérios TD-01 a TD-27. Não são requisito novo.

**Coverage:** 27 requisitos, mapeados em `.specs/features/f02-takeout-discovery/tasks.md`.

---

## Success Criteria

- [x] Os 27 critérios passam em teste automatizado sobre fixtures no repositório
- [x] Cada variação nomeada na seção 9.1–9.4 do PRD tem um critério correspondente (TD-01 a TD-09, TD-19 a TD-22, TD-23)
- [x] A fixture não é modificada e `E:\` não é lida
- [x] Um JSON inválido não remove do resultado a mídia vizinha que tem JSON válido
