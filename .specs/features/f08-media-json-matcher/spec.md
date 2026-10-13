# F08 — Media↔JSON Matcher Specification

## Problem Statement

O F02 já implementa a cascata mídia↔JSON (`src-tauri/src/takeout/matching.rs`) e passa em 100% das fixtures rotuladas. Faltava o critério de Phase 0 sobre a biblioteca real: **>98% das mídias com o JSON certo**. A primeira medição no `E:` (2026-10-13) deu 69,26% no total, mas o número misturava fontes que nunca têm sidecar e marcava como `heuristic` todos os acertos do Google Fotos, porque o nome padrão `.supplemental-metadata.json` caía no passo heurístico.

## Goals

- [x] Medir o matcher no Takeout real com uma métrica que reflita o que o Google entrega
- [x] O nome padrão, inteiro, `{arquivo}.supplemental-metadata.json` conta como `exact`
- [x] Documentar o que fica sem JSON e por quê

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Gravar o vínculo no banco (`json_match_confidence`) | F10 |
| Escolher entre JSON, EXIF e sistema de arquivos | F09 |
| Mídia do Drive, `Photos-1-001`, `Clipes`, vídeos `WIN_*` soltos | Não são export do Google Fotos; usam EXIF (F07) |
| JSON órfão cuja mídia não veio no export | Lacuna do export; o F02 já reporta como problema `OrphanJson` |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Denominador do >98% | Mídias com uma pasta `Google Fotos` ou `Google Photos` no caminho relativo | As outras fontes do `E:` (1 885 arquivos) nunca têm sidecar; contá-las mede o acervo, não o matcher | n |
| `{arquivo}.supplemental-metadata.json` inteiro | `exact`, de qualquer tamanho; o truncamento continua `heuristic` | É a grafia padrão do Takeout desde 2024 e não é ambígua; com `heuristic`, nenhum export atual teria `exact` | n — card de decisão aberto em 2026-10-13 |
| `{stem}.supplemental-metadata.json` inteiro | `exact`, mesma regra | Simetria com o passo 1 (`{stem}.json` já é `exact`) | n |
| Marcador `(N)`, `-edited`, truncamentos | Continuam `heuristic` | O nome do sidecar não identifica a mídia sozinho | y (F02) |

---

## Medição no `E:` (2026-10-13, antes da mudança)

| Recorte | Mídias | Com JSON | % |
| ------- | ------ | -------- | - |
| Tudo | 6 132 | 4 247 | 69,26 |
| Google Fotos | 4 247 | 4 247 | **100,00** |
| Drive (`takeout-…-2-001/Takeout/Drive`) | 631 | 0 | — |
| `Ruan/Photos-1-001` | 1 196 | 0 | — |
| `Ruan/Clipes` | 55 | 0 | — |
| `WIN_*.mp4` soltos | 3 | 0 | — |

Antes: os 4 247 acertos eram todos `heuristic`, 0 `exact`, 0 ambíguos. Problemas: 5 `OrphanJson` em `takeout-20260921T183943Z-1-001/Takeout/Google Fotos/Fotos de 2025/` (ex.: `IMG_20250719_145300.jpg.supplemental-metadata.json`); as fotos deles não estão no `E:`.

---

## Requirements

### MATCH-01 — Nome supplemental inteiro é exato

1. WHEN a pasta contém `foto.jpg` e `foto.jpg.supplemental-metadata.json` THEN o matcher SHALL associar os dois com `exact`.
2. WHEN o nome supplemental inteiro tem mais de 51 bytes e existe com esse nome THEN SHALL ser `exact`.
3. WHEN só existe o truncamento de 51 bytes do nome supplemental THEN SHALL ser `heuristic`.
4. WHEN o sidecar está em outra parte do mesmo export (`-1-002` vs `-1-001`) com o nome inteiro THEN SHALL ser `exact`.
5. Marcador `(N)` e `-edited` SHALL continuar `heuristic`.

### MATCH-02 — Métrica no Takeout real

1. O teste ignorado `real_takeout_survey` SHALL imprimir `google_photos=N matched=M` e a porcentagem, e falhar se ela for ≤98%.
2. WHEN rodado no `E:` THEN SHALL dar >98% (100% esperado) e os acertos do Google Fotos SHALL ser `exact` em sua maioria.

## Success Criteria

- [ ] CI `windows-latest` verde
- [ ] `real_takeout_survey` no `E:` com `google_photos_matched` >98% (ver `validation.md`)
