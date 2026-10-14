# F10 — Indexing Pipeline Specification

## Problem Statement

F05 grava caminhos e hashes; F02/F06/F08 casam e leem o JSON do Takeout; F07 lê o EXIF; F09 decide data e local. Nada disso chegava ao banco além do F05. A F10 junta tudo numa execução só (RF-006..010, RF-014/015, RF-063), incremental e tolerante a falhas, para que a timeline (F18) tenha data, local, pessoas e dimensões.

## Goals

- [x] Uma execução: scan → match/parse → EXIF → resolve → gravação, num único `indexing_runs`
- [x] Incremental: só lê de novo o que mudou
- [x] Um arquivo ou JSON ruim nunca para a execução; vira `indexing_errors`
- [x] Progresso por etapa para a F17
- [x] Rodar no `E:` real

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Álbuns do Takeout (`albums`, `media_albums`) | F25 |
| Thumbnails, metadados de vídeo | F12, F13 |
| UI de progresso, cancelamento | F17 |
| JSON editado com o mesmo nome | Detectado só quando a mídia muda ou o par muda; ver Open Questions |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| O que reprocessar | Mídia lida pelo scan (nova/modificada/sem hash), nunca resolvida (`taken_at_source` vazio) ou cujo par JSON (caminho ou confiança) mudou | EXIF é a parte cara; o JSON de todas as mídias é lido em toda execução (rápido) | n |
| Fuso padrão | -03:00 (AD-012) | F09 | n — card aberto |
| Erros de sidecar | `invalid_json`, `orphan_json`, `unreadable` da descoberta; nada dentro de `.myphotos` (lixeira) | Pasta/nomes ruins já são do F05 | y |
| EXIF corrompido | `corrupt_media` em `indexing_errors`; a mídia continua `ok` e é resolvida com o que deu para ler | F07: "quem grava `corrupt_media` é a F10" | y |
| Pessoas | `people` por `normalized_name` (primeira grafia vence) + `media_people`, substituídos a cada reprocessamento | F06 | y |
| Vídeo | Sem EXIF; data do JSON ou do sistema de arquivos | F13 cuida de vídeo | y |
| `files_indexed` | Mídias resolvidas nesta execução | | n |
| JSON editado com o mesmo nome | Não reprocessa | Raro no Takeout; uma comparação de conteúdo custaria guardar hash do JSON | open |

---

## Requirements

### IDX-01 — Primeira execução
1. Toda mídia ganha `taken_at`, `taken_at_source`, `taken_at_timezone`, `has_takeout_json`, `json_relative_path`, `json_match_confidence` (`none` sem JSON), `width/height/orientation` (fotos), `title/description`.
2. `media_locations`, `media_metadata_raw`, `people`/`media_people` preenchidos.
3. Só fotos vão ao leitor de EXIF.
4. `indexing_runs`: `completed`, `files_scanned`, `files_indexed`.
5. Progresso: `scanning`, `matching`, `resolving` (0/N e após cada lote).

### IDX-02 — Incremental
1. Sem mudança: nada é lido nem reescrito.
2. JSON que aparece ou some: só a mídia dele é reprocessada.
3. Arquivo modificado: é relido.

### IDX-03 — Reprocessar substitui
1. Local, JSON bruto e pessoas de uma mídia são trocados, nunca acumulados.

### IDX-04 — Problemas de sidecar
1. JSON órfão e inválido viram `indexing_errors`; dentro de `.myphotos` não.
2. A mídia de um JSON inválido é indexada sem JSON.

### IDX-05 — Falhas de EXIF
1. EXIF corrompido vira `corrupt_media` e a mídia é resolvida; EXIF ilegível segue sem EXIF.

### IDX-06 — Mídia sumida
1. `missing_file` não é reprocessada e mantém os metadados.

## Success Criteria

- [x] `index::tests` (8) cobrindo IDX-01..06
- [ ] CI `windows-latest` verde
- [x] `f10-index` no `E:`: 6 132 resolvidas (4 247 JSON, 850 EXIF, 1 035 arquivo, 0 sem data); 2ª execução 0 resolvidas em 1,4 s
