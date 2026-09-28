# F03 — Performance Baseline Specification

## Problem Statement

O scanner (F05) e o pipeline de indexação (F10) vão percorrer centenas de milhares de arquivos e calcular SHA-256. O PRD (RNF-003 e a seção 17) pede que as metas quantitativas só sejam fechadas depois de medir bibliotecas sintéticas. Sem esse número — em HDD e em SSD, em Windows — as features seguintes otimizam no escuro.

## Goals

- [x] Uma chamada local cria uma biblioteca sintética, mede a varredura e o SHA-256, classifica o volume e devolve um relatório com contagens e tempos inteiros
- [x] O SHA-256 de cada arquivo é determinístico e verificável por um vetor conhecido, em qualquer máquina
- [x] A chamada só cria e apaga arquivos dentro do diretório informado, e recusa apagar uma pasta que já tenha conteúdo
- [x] Os tamanhos oficiais 50.000 e 150.000 rodam no volume que o operador escolher (HDD ou SSD). O teste automático prova o mesmo contrato com 64 arquivos, sem meta de milissegundos

## Out of Scope

Explicitamente fora. Documentado para impedir escopo extra.

| Feature | Reason |
| ------- | ------ |
| Biblioteca real em `E:\` (AD-007) | Esta feature nunca aponta o padrão para `E:\` nem lê fotos do usuário. Um subdiretório vazio em qualquer volume só entra se o operador o passar |
| Indexação, SQLite, EXIF, JSON do Takeout, matcher, thumbnails (F04–F13) | O número medido aqui é só varredura de diretório mais SHA-256. Quem persiste e quem resolve metadado vem depois |
| Hash perceptual (F29) | RNF desta feature é o SHA-256 de identidade, não quase-duplicata |
| Metas de UI: abertura em 2s, ~60 fps, busca abaixo de 300ms (seção 17 do PRD, F18, F32) | Não há interface nem banco indexado. Esses tempos não são deste baseline |
| Corpus de 500.000 itens (seção 17 do PRD) | O roadmap da F03 para em 50.000 e 150.000. 500.000 é carga da F32 |
| Indexação incremental, pausa, disco desconectado (RNF-015, F10, F11) | Uma execução lê um diretório que ela mesma acabou de criar |
| macOS e Linux | AD-003. Fora do Windows a classe do volume é `unknown`; o restante do contrato continua o mesmo |
| Telemetria, rede, log do conteúdo dos arquivos | RNF-007. O relatório fica em memória e, na corrida oficial, num arquivo que o operador grava no repositório da feature |

---

## Assumptions & Open Questions

Cada ambiguidade está resolvida ou registrada aqui.

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Tamanho de cada arquivo | Exatos 4.096 bytes, conteúdo `(índice + deslocamento) mod 251` | 150.000 fotos de vários MB exigiriam centenas de GB e mediriam o disco sequencial, não o custo de muitos arquivos. 4 KiB × 150.000 = 600 MiB, dá para gerar e apagar, e o hash lê cada byte | y |
| Layout | `f03/{ano}/{mês}/item-{índice de 7 dígitos}.bin`, 100 arquivos por pasta de mês, ano = 2000 + (índice / 1200), mês = ((índice / 100) mod 12) + 1 | Densidade parecida com pastas de mês/álbum do Takeout, sem copiar nomes reais de usuário | y |
| Tamanhos oficiais | 50.000 e 150.000. O teste automático usa 64. Qualquer inteiro de 1 a 150.000 inclusive é aceito pela função; 0 ou acima de 150.000 é erro | O roadmap cita 50k e 150k. 64 cabe no CI. O teto impede um 500k acidental | y |
| Onde o corpus vive | Diretório absoluto com letra de unidade, pai existente, e vazio se já existir. Raiz de volume (`C:\`) é recusada. Pasta não vazia é recusada sem apagar nada | Permite um diretório vazio no HDD, inclusive um subdiretório criado à parte em `E:\`, e impede varrer ou limpar o acervo | y |
| Limpeza | No fim da execução — sucesso ou falha de arquivo individual — o diretório raiz informado deixa de existir. Falha de validação, antes de criar arquivo, não altera o disco | Não deixar 150.000 arquivos para trás. Não destruir uma pasta rejeitada | y |
| Paralelismo do hash | Número de workers = paralelismo disponível do processo, mínimo 1, gravado no relatório. A varredura é uma única passagem. O resumo dos hashes é na ordem do índice, não na ordem em que os workers terminam | O scanner futuro será paralelo. Um único protocolo, com o número de workers visível, evita dois baselines divergentes | y |
| Tempos | Inteiros em milissegundos, fração truncada em direção a zero. Nenhum teste falha por ultrapassar um limite de tempo | A seção 17 do PRD diz para fechar meta depois da medição. Um gate de ms no CI quebra com runner e disco diferentes | y |
| HDD e SSD | Cada execução classifica o volume do diretório. O gate automático não exige os dois meios. A evidência da feature são duas corridas oficiais por meio presente na máquina; meio ausente fica registrado como não medido | Esta máquina pode não ter os dois discos ao mesmo tempo. Inventar um HDD em CI não mede nada | y |
| Classe do volume | No Windows, `IncursSeekPenalty == true` → `hdd`, `false` → `ssd`. Falha da consulta, ou sistema que não é Windows → `unknown` | É o sinal que o Windows já usa para distinguir disco com penalidade de seek (HDD) de disco sem penalidade (SSD) | y |
| Relatório oficial | A função devolve a estrutura. Na corrida de 50.000 e de 150.000 o operador grava o JSON em `.specs/features/f03-performance-baseline/baselines/`. O CI não gera esses arquivos | O artefato da Phase 0 é o número medido, separado do teste de correção | y |

**Open questions:** nenhuma. Spec confirmada pelo stakeholder em 2026-09-28, com os defaults acima.

### Dimensões implícitas

| Dimension | Resolução |
| --------- | --------- |
| Input validation & bounds | WHEN a contagem está fora de 1..=150.000, o caminho não é absoluto com letra de unidade, o pai não existe, o caminho é raiz de volume, ou o diretório já tem qualquer entrada THEN a função SHALL retornar o erro correspondente e SHALL NOT criar, alterar ou apagar arquivo |
| Failure / partial-failure | WHEN a criação do corpus falha no meio THEN o que foi criado sob o diretório SHALL ser removido e a função SHALL retornar erro, sem relatório `ok`. WHEN um arquivo não pode ser lido no hash THEN os demais SHALL ser processados, `ok` SHALL ser false, e a limpeza SHALL ocorrer mesmo assim. WHEN a limpeza falha THEN a função SHALL retornar erro |
| Idempotency / retry | WHEN a mesma contagem roda de novo num diretório vazio THEN os bytes e o `contentDigest` SHALL ser iguais. Os tempos podem mudar. Uma corrida não deixa lixo que impeça a seguinte, salvo crash do processo |
| Auth boundaries & rate limits | N/A porque o uso é local, sem conta, sem rede e sem chamada externa |
| Concurrency / ordering | O hash usa workers em paralelo. O `contentDigest` SHALL concatenar os digests de 32 bytes na ordem crescente do índice. A varredura não atravessa ponto de reanálise (junção ou symlink de diretório) |
| Data lifecycle / expiry | O corpus é temporário. Ao terminar, o diretório raiz informado não existe. Não há TTL nem arquivo de biblioteca |
| Observability | O relatório desta spec é o registro. Sem log de conteúdo, sem telemetria, sem rede |
| External-dependency failure | A dependência é o sistema de arquivos e a consulta de volume no Windows. Falha de criação aborta e limpa. Falha da consulta de penalidade de seek vira `unknown`, sem falhar a medição |
| State-transition integrity | N/A porque não há estado persistido. Cada chamada começa com um diretório vazio (ou inexistente com pai existente) e termina sem ele |

---

## Regras normativas

Aplicam-se à função de baseline. Índices vão de 0 até `itemCount - 1`.

**Bytes do arquivo de índice `i`.** Exatos 4.096 bytes. O byte no deslocamento `j` (0-based) é `(i + j) mod 251`, em inteiro não negativo, gravado como um byte.

**Caminho.** `{raiz}/f03/{ano}/{mês com dois dígitos}/item-{índice com sete dígitos}.bin`, com `ano = 2000 + (i / 1200)` e `mês = ((i / 100) mod 12) + 1`. Divisão inteira, truncada em direção a zero.

**SHA-256.** Digest FIPS 180-4 dos 4.096 bytes, 32 bytes. No relatório, hex minúsculo de 64 caracteres só no agregado.

**`contentDigest`.** SHA-256 da concatenação dos digests de 32 bytes dos arquivos lidos com sucesso, em ordem crescente de índice. Índices que falharam o read ficam de fora da concatenação.

**Vetores conhecidos** (arquivo completo, hex do digest):

| Índice | SHA-256 |
| ------ | ------- |
| 0 | `d67c656e01756650d77717b0839985a056ec28ffe174601d690fc407a2ceffca` |
| 1 | `29fa02d3381a0b093ff2d6d783c1c78176fd1aafce643c8d205631dc85df5e95` |
| 63 | `b21fe85ea1fba0db5690a657db55547f0cf8cb4a9ea81ac332e67de8de84be06` |

`contentDigest` quando os 64 arquivos (índices 0 a 63) são lidos com sucesso: `c067a4e6cde3d7306e1fef52a92ebee10c1e562368ad06488bb1398610067415`.

**Varredura.** Uma passagem a partir da raiz. Conta arquivo regular. Não segue diretório que seja ponto de reanálise. Não sai da raiz. O relógio da varredura não inclui a criação do corpus nem o hash.

**Hash.** Cada arquivo regular encontrado é lido em streaming, com buffer de no máximo 65.536 bytes por worker. O processo não carrega o corpus inteiro em memória. O relógio do hash não inclui a criação nem a varredura. `workerCount` é o paralelismo disponível do processo, no mínimo 1.

**Relógio.** Milissegundos inteiros, fração truncada em direção a zero. Zero é permitido.

**Classe do volume.** `classify(seek_penalty)`: ausência de resposta → `unknown`; `true` → `hdd`; `false` → `ssd`. No Windows a resposta é `IncursSeekPenalty` do volume que contém a raiz. Fora do Windows a resposta está ausente.

**Erros, antes de criar arquivo.** Código estável, sem efeito no disco:

| Código | Condição |
| ------ | -------- |
| `count_out_of_range` | Contagem menor que 1 ou maior que 150.000 |
| `root_not_absolute` | Sem letra de unidade, caminho relativo, ou UNC |
| `root_is_volume` | O caminho normalizado é só a raiz do volume (`C:\`) |
| `parent_missing` | O pai do diretório não existe |
| `root_not_empty` | O diretório existe e tem qualquer entrada |

**Erro durante a criação.** Código `setup_failed`. Tudo o que a chamada criou sob a raiz é removido antes do retorno. Não há relatório.

**Relatório.** Campos exatamente estes:

| Campo | Tipo | Valor |
| ----- | ---- | ----- |
| `itemCount` | inteiro | A contagem pedida |
| `bytesPerItem` | inteiro | 4096 |
| `volumeClass` | string | `hdd`, `ssd` ou `unknown` |
| `volumeRoot` | string | Raiz do volume com barra invertida final, letra maiúscula, por exemplo `C:\`. Fora do Windows, string vazia |
| `workerCount` | inteiro | ≥ 1 |
| `filesHashed` | inteiro | Arquivos cujo SHA-256 foi calculado |
| `totalBytes` | inteiro | `filesHashed * 4096` |
| `failureCount` | inteiro | Tamanho de `failures` |
| `failures` | lista | Cada item: `relativePath` (separador `/`, relativo à raiz) e `reason` igual a `unreadable` |
| `contentDigest` | string | 64 hex minúsculos, ou string vazia se nenhum arquivo foi hasheado |
| `walkElapsedMs` | inteiro | ≥ 0 |
| `hashElapsedMs` | inteiro | ≥ 0 |
| `ok` | booleano | true somente quando `failureCount` é 0 e `filesHashed` é igual a `itemCount` |

**Limpeza depois do relatório.** Se a criação terminou, a função remove a raiz e todo o conteúdo, inclusive quando `ok` é false. Se essa remoção falha, o retorno é erro `cleanup_failed` e o relatório não conta como evidência.

---

## User Stories

### P1: Corpus que não toca o acervo ⭐ MVP

**User Story**: Como desenvolvedor do My Photos, quero gerar e apagar uma biblioteca sintética só dentro de uma pasta vazia que eu indique, para medir I/O sem alterar a biblioteca em `E:\`.

**Why P1**: AD-004 e AD-007. Um benchmark que escreve no acervo quebra o princípio do produto.

**Acceptance Criteria**:

1. WHEN a contagem é 0 ou maior que 150.000 THEN a função SHALL retornar `count_out_of_range` e SHALL NOT criar nem apagar arquivo.
2. WHEN a raiz não é um caminho absoluto com letra de unidade (relativo ou UNC) THEN a função SHALL retornar `root_not_absolute` e SHALL NOT criar nem apagar arquivo.
3. WHEN a raiz normalizada é somente a raiz de um volume THEN a função SHALL retornar `root_is_volume` e SHALL NOT criar nem apagar arquivo.
4. WHEN o pai da raiz não existe THEN a função SHALL retornar `parent_missing` e SHALL NOT criar nem apagar arquivo.
5. WHEN a raiz existe e contém qualquer arquivo ou subdiretório THEN a função SHALL retornar `root_not_empty` e SHALL NOT criar, alterar ou apagar arquivo dentro ou fora dela.
6. WHEN a raiz não existe, o pai existe, a contagem é válida e a execução termina (com `ok` true ou false) THEN a função SHALL ter criado os arquivos apenas sob essa raiz e, antes de retornar com sucesso de limpeza, SHALL fazer a raiz deixar de existir.
7. WHEN a criação de um arquivo do corpus falha THEN a função SHALL retornar `setup_failed`, SHALL remover o que criou sob a raiz, e SHALL NOT devolver relatório com `ok` true.

**Independent Test**: Casos de erro contra uma pasta vazia e uma pasta com um arquivo marcador, conferindo que o marcador permanece. Uma corrida de 64 arquivos confere que a raiz sumiu e que nada fora dela foi criado.

---

### P1: Varredura e SHA-256 determinísticos ⭐ MVP

**User Story**: Como desenvolvedor, quero o tempo da varredura e o tempo do hash, com um digest que só depende do índice, para repetir a medição e saber que leu os bytes certos.

**Why P1**: É o número que a F05 e a F10 vão usar como referência.

**Acceptance Criteria**:

1. WHEN a função termina com os 64 arquivos lidos THEN `contentDigest` SHALL ser `c067a4e6cde3d7306e1fef52a92ebee10c1e562368ad06488bb1398610067415`, `filesHashed` SHALL ser 64, `totalBytes` SHALL ser 262144, `bytesPerItem` SHALL ser 4096 e `ok` SHALL ser true.
2. WHEN o arquivo de índice 0 é hasheado THEN o digest SHALL ser `d67c656e01756650d77717b0839985a056ec28ffe174601d690fc407a2ceffca`. WHEN o índice 1 THEN SHALL ser `29fa02d3381a0b093ff2d6d783c1c78176fd1aafce643c8d205631dc85df5e95`. WHEN o índice 63 THEN SHALL ser `b21fe85ea1fba0db5690a657db55547f0cf8cb4a9ea81ac332e67de8de84be06`.
3. WHEN a varredura e o hash terminam THEN `walkElapsedMs` e `hashElapsedMs` SHALL ser inteiros ≥ 0, medidos em separado da criação do corpus e um do outro.
4. WHEN um arquivo regular do corpus não pode ser lido THEN a função SHALL seguir com os outros, SHALL incluir esse caminho relativo com `reason` igual a `unreadable`, SHALL fazer `ok` false, e SHALL manter `filesHashed` igual à contagem de leituras bem-sucedidas.
5. WHEN o mesmo índice é hasheado duas vezes em processos diferentes THEN o digest daquele arquivo SHALL ser igual. Os tempos MAY diferir.
6. WHEN existe um diretório ponto de reanálise dentro da árvore, apontando para fora da raiz THEN a varredura SHALL NOT contar os arquivos do lado de fora.

**Independent Test**: Corrida de 64 arquivos contra os quatro hex da regra normativa. Um arquivo tornado ilegível (ou uma leitura injetada como falha) deixa `ok` false e um item em `failures`. Duas chamadas do cálculo do arquivo 0 batem o mesmo hex.

---

### P1: Classificar HDD, SSD ou desconhecido ⭐ MVP

**User Story**: Como desenvolvedor, quero saber se a pasta medida está num HDD ou num SSD, para não misturar os dois baselines.

**Why P1**: O roadmap exige a medição nos dois meios. Sem a classe no relatório, o número não diz de qual disco veio.

**Acceptance Criteria**:

1. WHEN `classify` recebe penalidade de seek `true` THEN SHALL retornar `hdd`. WHEN recebe `false` THEN SHALL retornar `ssd`. WHEN a resposta está ausente THEN SHALL retornar `unknown`.
2. WHEN a função roda no Windows e a consulta do volume responde THEN `volumeClass` SHALL ser o resultado de `classify` sobre `IncursSeekPenalty`, e `volumeRoot` SHALL ser a raiz daquele volume com letra maiúscula e barra invertida final.
3. WHEN a consulta do volume falha, ou o sistema não é Windows THEN `volumeClass` SHALL ser `unknown` e a medição SHALL continuar. Fora do Windows `volumeRoot` SHALL ser string vazia.
4. WHEN `workerCount` é lido no relatório THEN SHALL ser inteiro ≥ 1 e SHALL ser o paralelismo disponível do processo no momento da chamada, ou 1 se esse número não puder ser obtido.

**Independent Test**: Três chamadas puras de `classify(true)`, `classify(false)` e `classify` sem valor. Uma corrida de 64 arquivos no Windows confere que `volumeClass` é uma das três strings e que `volumeRoot` termina com `:\`.

---

### P1: Evidência em 50.000 e 150.000 ⭐ MVP

**User Story**: Como desenvolvedor, quero rodar os dois tamanhos oficiais no volume que eu escolher e guardar o JSON, para a Phase 0 ter um baseline real de I/O.

**Why P1**: O critério de saída da Phase 0 pede o benchmark de I/O validado. O teste de 64 arquivos não substitui essa corrida.

**Acceptance Criteria**:

1. WHEN o operador executa a função com `itemCount` 50.000 e, em outra chamada, 150.000, cada uma sobre um diretório vazio num volume cujo `volumeClass` é `hdd` ou `ssd` THEN cada relatório SHALL ter `ok` true, `filesHashed` igual à contagem pedida, `bytesPerItem` 4096 e `failureCount` 0, e a raiz SHALL deixar de existir.
2. WHEN essas duas corridas terminam THEN o operador SHALL gravar os dois JSON em `.specs/features/f03-performance-baseline/baselines/`, no nome `{volumeClass}-{itemCount}.json`.
3. WHEN um dos dois meios não existe nesta máquina THEN a evidência SHALL registrar esse meio como não medido, e as corridas do meio presente SHALL permanecer válidas. O teste automático SHALL NOT falhar por causa do meio ausente.
4. WHEN o CI executa a suíte THEN SHALL rodar o contrato com 64 arquivos e SHALL NOT criar o corpus de 50.000 nem o de 150.000.

**Independent Test**: Na máquina de desenvolvimento, duas corridas por meio presente, JSON conferido contra os campos obrigatórios. No CI, só a suíte de 64 arquivos.

---

## Edge Cases

- WHEN a contagem é exatamente 1 ou exatamente 150.000 THEN a função SHALL aceitar. WHEN é 150.001 THEN SHALL retornar `count_out_of_range`.
- WHEN o diretório raiz está vazio e existe THEN a função SHALL tratar como utilizável e, ao terminar a limpeza, SHALL removê-lo.
- WHEN nenhum arquivo é hasheado porque todos falharam a leitura THEN `contentDigest` SHALL ser string vazia, `filesHashed` SHALL ser 0, `ok` SHALL ser false.
- WHEN `walkElapsedMs` ou `hashElapsedMs` é 0 THEN o relatório SHALL ainda ser válido. Não há divisão exposta no contrato.
- WHEN a limpeza não consegue apagar a raiz THEN a função SHALL retornar `cleanup_failed`.
- WHEN a execução do processo é morta no meio THEN arquivos podem restar. A chamada seguinte SHALL ver a raiz não vazia e retornar `root_not_empty`, sem apagar esse resto sozinha.

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| PERF-01 | P1: Corpus que não toca o acervo | Execute | Done |
| PERF-02 | P1: Varredura e SHA-256 determinísticos | Execute | Done |
| PERF-03 | P1: Classificar HDD, SSD ou desconhecido | Execute | Done |
| PERF-04 | P1: Evidência em 50.000 e 150.000 | Execute | Done |

**ID format:** `PERF-NN`

**Coverage:** 4 total, 4 mapped to tasks, 0 unmapped.

---

## Success Criteria

- [x] Os vetores de SHA-256 dos índices 0, 1 e 63 e o `contentDigest` de 64 arquivos passam em teste automático
- [x] Rejeitar raiz de volume e pasta não vazia não altera nenhum arquivo existente
- [x] Uma corrida de 64 arquivos remove a raiz ao terminar
- [x] `classify` devolve `hdd`, `ssd` ou `unknown` conforme a penalidade de seek
- [x] Existem JSON de 50.000 e de 150.000 para cada meio presente na máquina de desenvolvimento, e o meio ausente está declarado como não medido
