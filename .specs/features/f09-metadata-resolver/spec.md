# F09 — Metadata Resolver Specification

## Problem Statement

Cada mídia pode ter até três fontes de data e local: o JSON do Takeout (F06, casado pela F08), o EXIF embutido (F07) e as datas do sistema de arquivos (F05). A linha do tempo precisa de **uma** data por mídia, com a origem rastreável em `taken_at_source` (PRD 9.5, RF-004/005/028). Sem uma regra única, a F10 gravaria datas inconsistentes.

## Goals

- [x] Uma função pura que, dadas as três fontes, devolve os campos de `media_items`/`media_locations` e a origem de cada data
- [x] Hierarquia: Takeout JSON > EXIF > sistema de arquivos
- [x] Hora sem fuso nunca vira UTC sem avisar (`taken_at_offset_assumed`)

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Ler arquivos, gravar no banco | F10 |
| Metadados de vídeo (duração, dimensões) | F13 |
| Pessoas e álbuns do JSON | F10 grava direto do F06 |
| Detectar `photoTakenTime` errado no JSON (igual à data de upload) | Não medido; ver Open Questions |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Data do JSON | `photoTakenTime` (`taken_at`, UTC). `creationTime` (upload) nunca vira data de captura | PRD 9.5; upload não é captura | y (PRD) |
| EXIF com fuso | `taken_at` UTC do F07, `taken_at_timezone` = `-03:00` | O arquivo diz o fuso | y |
| EXIF sem fuso | Hora local + fuso padrão -03:00 (AD-012), `taken_at_timezone` vazio, `taken_at_offset_assumed = true` | Acervo majoritariamente no Brasil | n — card aberto |
| Sistema de arquivos | O **mais antigo** entre `fs_created_at` e `fs_modified_at` | Cópia no Windows zera a criação e mantém a modificação | n |
| Local | JSON (com a origem `takeout_geoData`/`takeout_geoDataExif` do F06) > EXIF (`exif`) | Mesma hierarquia da data | y (PRD) |
| Dimensões, orientação | Só do F07 | O JSON não tem | y |
| Título, descrição | Só do JSON | O EXIF não tem esses campos no F07 | y |
| `photoTakenTime` igual ao upload | Aceito como está | Precisa de medição no `E:` antes de mudar a hierarquia | open |

---

## Requirements

### RES-01 — JSON primeiro
1. WHEN o JSON tem `taken_at` THEN SHALL ser a data, `taken_at_source = takeout_json`, sem fuso, mesmo com EXIF e sistema de arquivos.
2. WHEN o JSON não tem `taken_at` THEN SHALL seguir para o EXIF; `creation_time` nunca é usado.

### RES-02 — EXIF
1. WHEN o EXIF tem fuso THEN `taken_at` SHALL ser o UTC do F07 e `taken_at_timezone` o fuso.
2. WHEN o EXIF só tem hora local THEN `taken_at` SHALL ser essa hora menos o fuso padrão, `taken_at_offset_assumed = true`, `taken_at_timezone` vazio (vira dia/ano corretamente).
3. WHEN a hora local não está no formato `AAAA-MM-DDTHH:MM:SS.mmm` válido THEN SHALL seguir para o sistema de arquivos.

### RES-03 — Sistema de arquivos
1. WHEN não há data no JSON nem no EXIF THEN SHALL usar o mais antigo de `fs_created_at`/`fs_modified_at`, `taken_at_source = filesystem`.
2. WHEN não há nenhuma data THEN `taken_at` e `taken_at_source` SHALL ficar vazios.

### RES-04 — Local
1. WHEN o JSON tem ponto válido THEN SHALL ser o local, com a origem do F06.
2. WHEN só o EXIF tem ponto THEN SHALL ser o local com origem `exif`.

### RES-05 — Demais campos
1. Largura, altura e orientação SHALL vir do F07; título e descrição do JSON.

## Success Criteria

- [x] Testes `resolve::tests` cobrindo RES-01..05 (10 testes)
- [ ] CI `windows-latest` verde
