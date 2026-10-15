# F16 — Onboarding Flow Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboards **`Main`** / **`MainEscuro`** (AD-014, `.specs/DESIGN.md`).

## Problem Statement

No primeiro uso o app abre direto na moldura vazia. O usuário precisa ver qual pasta será lida, entender que nada muda no disco sem ele pedir (AD-008, AD-009, AD-011) e começar a primeira indexação com um clique (RF-001, US-01).

## Goals

- [x] Tela de boas-vindas do canvas quando o disco está conectado e nunca foi indexado
- [x] Pasta da biblioteca com o nome do volume e quantos exports do Takeout há na raiz
- [x] "Indexar E:\" inicia a F10 em segundo plano e leva para a moldura; a lateral mostra o andamento
- [x] Falha da indexação aparece com o motivo e pode ser tentada de novo

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Trocar a pasta da biblioteca | Decisão do Ruan em 2026-10-15 ("Depois"): muda a regra do F11 (AD-007); vira feature própria junto com Configurações. O link aparece desativado |
| Tela de progresso (contagem por tipo, percentual, ETA) | F17 |
| Disco desconectado | F11 (UI) |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Quando mostrar | `connected` e `lastIndexedAt` nulo e 0 mídias | Uma indexação que falhou no meio volta para as boas-vindas só se nada entrou no banco | n |
| Exports do Takeout | Pastas na raiz: `takeout-<stamp>-N-NNN` contam uma vez por `<stamp>`; uma pasta `Takeout` conta uma | Mesma regra de partes do F02 | n |
| Rodar a indexação | Thread no Rust (`index_default_library`), conexão própria; eventos `indexing://progress` e `indexing://finished` | O banco é WAL; a UI não trava; a F17 reaproveita os eventos | n |
| Duas indexações | Recusada enquanto uma roda | Uma só escreve no banco | n |
| Fotos do painel escuro | Quatro retângulos de cor (`--photo-1..4`) entrando com Motion | Ainda não há miniaturas no primeiro uso | n |

## Requirements

### ONB-01 — Pasta
1. Mostra a raiz encontrada pelo F11 (`E:\`), o nome do volume e "N exports do Takeout encontrados" (ou "nenhum export do Takeout na raiz").

### ONB-02 — Quando aparece
1. Só com disco conectado, nunca indexado e sem mídias. Depois da primeira indexação, nunca mais.

### ONB-03 — Indexar
1. O botão inicia a indexação, leva para Fotos e a lateral mostra "Indexando E:\" com a etapa ou "N de M mídias".
2. Ao terminar, o cartão volta a "E:\ conectado · N mídias · atualizado às HH:MM".

### ONB-04 — Falha
1. Falha ao começar: mensagem na própria tela. Falha no meio: cartão "A indexação parou" com o motivo e "Tentar de novo".

### ONB-05 — Design
1. Medidas, textos e cores da artboard `Main` nos dois temas; só tokens; animação de entrada desligada com reduzir movimento.
