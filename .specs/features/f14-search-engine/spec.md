# F14 — Search Engine Specification

## Problem Statement

Com milhares de fotos, rolar a linha do tempo não basta: o usuário quer digitar "praia 2022", "ana" ou "agosto" e ver o resultado na hora (RF-034, RNF-001). O banco tem nomes, pastas, títulos, descrições, pessoas, local e notas, mas nada indexado para texto.

## Goals

- [x] Índice FTS5 com caminho (pasta + nome do arquivo), título, descrição, pessoas, local e nota
- [x] Índice sempre em dia, qualquer que seja quem escreve no banco
- [x] Ano, mês e tipo digitados na caixa viram filtros; filtros estruturados (data, tipo, pessoa, com local, favoritos) combinam com o texto
- [x] Resultado em ordem de linha do tempo, paginado, com total

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Tela de busca, chips de filtro | F22 |
| Nome de lugar a partir de GPS (geocodificação) | `place_label` ainda vazio; o índice já o inclui quando existir |
| Busca por conteúdo da imagem | Fora do MVP |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Manter o índice | Gatilhos SQL (migração v3) em `media_items`, `media_people`, `people`, `media_locations`, `user_notes` | Varredura, indexação e futuras edições na UI atualizam sem lembrar de chamar nada | n |
| Tokenização | `unicode61 remove_diacritics 2`, prefixos de 2 e 3 letras | "sao" acha "São"; `_`, `/`, `-` separam palavras, então "1234" acha `IMG_1234.jpg` e o nome da pasta acha o que está dentro | n |
| Palavras | Cada palavra é um prefixo entre aspas, todas obrigatórias (E) | "pra" acha "praia"; sintaxe FTS5 digitada vira texto comum, nunca erro | n |
| Ano / mês / tipo na caixa | 4 dígitos entre 1800 e 2199 → ano; nome completo do mês (pt ou en, com ou sem acento) → mês; foto(s)/imagem(ns)/vídeo(s) → tipo. Só o primeiro de cada | Abreviações como "mar" e "set" são palavras comuns | n |
| Palavras vazias | de, da, do, em, no, na, com, para, por… ignoradas | "praia em 2022" | n |
| Datas | Comparadas em UTC (`taken_at`) | Uma foto perto da meia-noite pode cair no mês vizinho; aceitável até a UI ter fuso | n |
| Ordem | `taken_at` mais recente primeiro, sem data por último | É o que a grade mostra | n |
| Fora do resultado | `is_hidden = 1` e `missing_file` | Não mostrar o que o usuário escondeu ou o que sumiu do disco | n |
| Página | 200 por padrão, até 2000 | Grade virtualizada pede em blocos | n |

---

## Requirements

### SRCH-01 — Caixa de busca
1. Palavras viram termos de prefixo; ano, mês e tipo viram filtros; palavras vazias somem; aspas e operadores digitados não quebram a busca.

### SRCH-02 — Texto
1. Acha por número do arquivo, nome da pasta, título e descrição, sem acento e por prefixo; várias palavras = todas.

### SRCH-03 / 04 — Sincronia
1. Pessoa ligada, desligada ou renomeada; local e nota inseridos, trocados e apagados refletem na busca.

### SRCH-05 — Filtros
1. Ano, mês+ano (dezembro vira o ano), só mês, intervalo `from`/`until`, tipo, pessoa, com/sem local, favoritos; escondidos e ausentes nunca voltam.

### SRCH-06 — Página
1. Mais recente primeiro, sem data no fim, `total` de todas as páginas, `limit` limitado a 1..2000.

### SRCH-07 — Integridade
1. Mover (novo caminho) e apagar atualizam o índice; outras colunas não mexem nele; `rebuild` refaz tudo.

### SRCH-08 — Atualização
1. Banco já indexado (v2) ganha o índice preenchido na migração v3.

### SRCH-09 — Desempenho
1. 20 mil mídias: busca com texto + ano + tipo em menos de 2 s em debug (medido: ~25 ms).

## Success Criteria

- [x] `search::tests` 9/9 no Linux
- [ ] CI `windows-latest` verde (inclui os testes de schema v3)
- [ ] No `E:` (6 132 mídias): busca por nome de pasta e pessoa (manual, quando houver a tela ou um binário)
