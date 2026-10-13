# F08 — Media↔JSON Matcher Design

## Mudança no código

`takeout/matching.rs`, `resolve_steps_1_to_4`. O passo 2 do F02 vira dois:

1. `Exact`: `{arquivo}.json` ou `{stem}.json` (sem mudança).
2. `Exact`: `{arquivo}.supplemental-metadata.json` ou `{stem}.supplemental-metadata.json`, nome inteiro.
3. `Heuristic`: truncamento (51 bytes) desses dois, só quando difere do nome inteiro (`truncated_only`).
4–6. Truncamento de `.json`, marcador `(N)`, `-edited`: sem mudança.

Dois candidatos no mesmo passo continuam empatando (`Ambiguous`). Se o nome inteiro e o truncado existirem juntos, o inteiro ganha no passo 2 em vez de empatar.

## Métrica

`takeout/mod.rs`, teste `real_takeout_survey` (ignorado, `TAKEOUT_ROOT`): além do total, conta as mídias cujo caminho relativo tem um componente `Google Fotos` ou `Google Photos` (`in_google_photos`) e falha se a taxa for ≤98%.

## Efeito em outras features

- `json_match_confidence` (F04) não muda de domínio; só mais linhas virão `exact` na F10.
- F02: regra 2 e AC 4 atualizados no spec/design do F02.
