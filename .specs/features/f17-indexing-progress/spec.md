# F17 — Indexing Progress UI Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboards **`Indexing`** / **`IndexingEscuro`** (AD-014, `.specs/DESIGN.md`).

## Problem Statement

A primeira indexação real do `E:` (2026-10-15) levou ~25 min, 20 deles lendo 109 GB para o hash, e a lateral ficou parada em "procurando arquivos". O usuário precisa ver o que está acontecendo, quanto falta, o que já foi encontrado e poder pausar ou sair navegando (RF-008, RF-009, RF-014).

## Goals

- [x] Tela de progresso do canvas: título com contagem, percentual, tempo restante, barra e as seis etapas
- [x] Contadores: fotos, vídeos, data pelo JSON, data pelo EXIF, JSON em outra parte do export, pessoas
- [x] Problemas até agora, com link para Problemas
- [x] "Chegando agora": miniaturas mais novas e o console com as últimas mídias pareadas
- [x] Etapa Miniaturas na indexação (decisão do Ruan em 2026-10-15: "Incluir")
- [x] Pausar e continuar; "Começar a navegar" com o andamento no cartão da lateral
- [x] Achados do primeiro uso real: progresso do hash, horário de término certo, exports um nível abaixo da raiz

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Miniaturas enquanto os metadados rodam | O F12 tem seu próprio escritor no banco; em paralelo com o lote do F10 disputariam o WAL. As miniaturas vêm depois, então "Chegando agora" se enche na última etapa |
| Tela de Problemas | F28; o link leva à seção, ainda vazia |
| Retomar o andamento depois de fechar o app | O banco guarda o que foi feito; o cartão "Indexação incompleta" oferece continuar |
| Indexação agendada / automática | F32 |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Etapas | Varredura = varrer + hash; "Leitura dos JSONs" e "Pareamento" andam juntas (uma passada do F08); "Metadados" e "Gravação no índice" também (o mesmo lote do F10) | É como o pipeline roda; o canvas mostra seis | Ruan viu no cartão de decisão |
| Percentual | Pesos por etapa: varredura 2%, hash 60% (por bytes), JSON 3%, metadados 15%, miniaturas 20% | Medidos a olho na primeira indexação real; por isso "cerca de" | n |
| Tempo restante | Ritmo desde o início da execução; só depois de 10 s e 2% | Antes disso o número pula demais | n |
| Pausar | Para depois do arquivo ou lote em andamento; a execução fica `cancelled`; "Continuar" roda de novo e só lê o que falta | O F05 já não relê o que tem hash; o F10 só resolve o pendente | n |
| Pausa vista no canvas | Mesmo layout; rótulo "Pausado · E:\", botão "Continuar", sem tempo restante | O canvas não tem esse estado | Ruan viu no cartão de decisão |
| Miniaturas na tela | `recent_thumbnails` devolve as 18 pequenas mais novas como data URL, consultado a cada 1,5 s na etapa Miniaturas | Sem protocolo de arquivos; 18 × ~15 KB | n |
| Contadores | Lidos do banco a cada lote (biblioteca inteira); "JSON em outra parte" = mídia e JSON em partes diferentes do export | O que o canvas mostra | n |

## Requirements

### PROG-01 — Título e percentual
1. O título conta a etapa atual: "N de M arquivos lidos" (hash), "N de M mídias" (metadados), "N de M miniaturas".
2. Ao lado: "62% · cerca de 4 min restantes".

### PROG-02 — Etapas
1. As seis etapas do canvas, marcadas feitas, atual ou por fazer.

### PROG-03 — Barra
1. Anda pela execução inteira; no hash, pelos bytes lidos.

### PROG-04 — Tempo restante
1. Pelo ritmo desde o início; "menos de 1 min", "cerca de N min", "cerca de N h M min".

### PROG-05 — Contadores, problemas e console
1. Seis contadores do canvas, problemas até agora com link para Problemas, as quatro últimas linhas do console.

### PROG-06 — Lateral
1. "Começar a navegar" abre o app; o cartão mostra "Indexando E:\ · 62% · …" e "Ver progresso".
2. Pausado: "Indexação pausada · N% feito · Continuar". Primeira indexação interrompida: "Indexação incompleta · Continuar".

### PROG-07 — Pausar
1. "Pausar" para depois do arquivo ou lote em andamento; o que foi gravado fica; "Continuar" só lê o que falta.

### PROG-08 — Fim da execução
1. `finished_at` e `last_indexed_at` são a hora em que a execução termina.

### PROG-09 — Design
1. Medidas, textos e cores das artboards `Indexing` / `IndexingEscuro`; só tokens.
