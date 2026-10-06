# F07 — EXIF Parser Specification

## Problem Statement

1 885 mídias do `E:` não vêm do Google Fotos e não têm sidecar do Takeout (STATE, open questions). Para elas, a única fonte de data de captura, local, dimensões e orientação é o que está **dentro** do arquivo. O F06 já entrega o que vem do JSON; falta o leitor de metadados embutidos que a F09 vai usar como segunda fonte (`taken_at_source = 'exif'`, `media_locations.source = 'exif'`) e que também preenche `width`, `height`, `orientation` e `media_metadata_raw.exif_json` para todas as fotos.

## Goals

- [ ] De uma foto (JPEG, HEIC/HEIF, PNG, WebP, TIFF/RAW) sai a data de captura com o fuso quando ele é conhecido, o GPS validado, as dimensões, a orientação e um JSON legível de todo o EXIF
- [ ] EXIF ausente ou corrompido nunca derruba a leitura: o que dá para ler é devolvido
- [ ] O arquivo só é lido; nenhum byte da biblioteca muda
- [ ] Testes geram as imagens em memória/temp; nenhum depende do `E:` nem de ferramenta externa

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Metadados de vídeo (`mvhd`/QuickTime, duração, dimensões) | F13 (ffmpeg/ffprobe empacotado) |
| Escolher entre JSON, EXIF e sistema de arquivos; gravar no banco | F09 / F10 |
| Aplicar a orientação (girar a imagem ou trocar largura/altura) | F12/F19; aqui a orientação é só reportada e as dimensões são as armazenadas |
| XMP, IPTC, MakerNote de fabricante | Fora do PRD 9.5; MakerNote é binário proprietário |
| Corrigir EXIF no arquivo | AD-008: escrita só por ação explícita do usuário |
| Hash perceptual | F29 |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Biblioteca EXIF | `kamadak-exif` 0.6 (Rust puro, BSD-2): JPEG, TIFF (DNG/CR2/NEF/ARW são TIFF), HEIF/HEIC, PNG (`eXIf`), WebP | Sem código nativo; licença permissiva; formatos da lista do MVP | y — "continue with defaults" (2026-10-06) |
| Dimensões | `imagesize` 0.15 lê o cabeçalho (JPEG, PNG, GIF, WebP, HEIF). Para RAW (`dng cr2 nef arw`) e quando o cabeçalho falha, `PixelXDimension`/`PixelYDimension` do EXIF | O cabeçalho é a verdade da imagem; em RAW o primeiro IFD costuma ser a miniatura | y |
| Tag da data | `DateTimeOriginal`; se ausente/inválida, `DateTimeDigitized`. `DateTime` (IFD0, data de modificação) nunca | PRD 9.5: data de captura; `DateTime` muda quando um editor regrava | y |
| Formato aceito | `AAAA:MM:DD HH:MM:SS` (também `-` na data); data de calendário real; ano 1800–9999; `0000:00:00 00:00:00`, vazio ou lixo → ausente. `SubSecTime*` (dígitos) vira milissegundos | Câmeras zeradas gravam zeros; mesma faixa de ano do F06 | y |
| Fuso | 1º `OffsetTime*` da mesma tag (`±HH:MM`, até ±14:00). 2º diferença entre a hora local e `GPSDateStamp`+`GPSTimeStamp` (UTC), arredondada a 15 min, se couber em ±14:00. 3º desconhecido | O EXIF guarda hora local de parede; o GPS é UTC | y |
| Saída da data | `taken_at_local` = hora de parede `AAAA-MM-DDTHH:MM:SS.mmm` (sem zona) sempre que há data; `taken_at_offset` (`-03:00`) e `taken_at` UTC (`…Z`, formato do F05/F06) só quando o fuso é conhecido | Não inventar UTC; a F09 decide o que fazer com hora sem fuso | y |
| GPS | `GPSLatitude`/`GPSLongitude` (3 racionais graus/min/seg) + `Ref` obrigatório (`N`/`S`, `E`/`W`); denominador 0, ref ausente, fora da faixa ou `0/0` → sem local. `GPSAltitude` + `GPSAltitudeRef` (1 = abaixo do mar) só com ponto válido | Mesma validação de ponto do F06; sem `Ref` o sinal é um chute (e o usuário está no hemisfério sul) | y |
| Orientação | Tag `Orientation` do IFD principal, só 1–8 | Valores do padrão; o resto é lixo | y |
| JSON bruto | Objeto `{ "NomeDaTag": "valor legível" }` dos campos do IFD principal (inclui Exif e GPS), sem `MakerNote`; cada valor cortado em 1024 caracteres; ausente quando não há EXIF | `media_metadata_raw.exif_json` para diagnóstico e reprocessamento | y |
| EXIF corrompido | Campos EXIF ausentes, `exif_corrupt = true`; dimensões ainda tentadas. Se o EXIF vier parcial, os campos lidos valem | RF-014: um item ruim não para nada; quem grava `corrupt_media` é a F10 | y |
| Arquivo ilegível | `Err(unreadable)` | Mesma semântica do F05 | y |

**Open questions:** nenhuma. Defaults aceitos pelo stakeholder antes da escrita ("continue with defaults", 2026-10-06).

### Dimensões implícitas

| Dimension | Resolução |
| --------- | --------- |
| Input validation & bounds | Datas, fuso, GPS, orientação e dimensões validados (EXIF-01..04); valores de texto do JSON cortados |
| Failure / partial-failure | Sem EXIF / EXIF corrompido / parcial (EXIF-05) |
| Idempotency / retry | Função de leitura pura: mesmo arquivo, mesmo resultado |
| Auth boundaries & rate limits | N/A |
| Concurrency / ordering | N/A: uma chamada por arquivo, sem estado |
| Data lifecycle / expiry | N/A: nada persistido |
| Observability | `exif_corrupt` e `Err(unreadable)` para a F10 registrar |
| External-dependency failure | Disco: `unreadable` |

---

## User Stories

### P1: Data de captura com fuso ⭐ MVP

**User Story**: Como usuário com fotos de câmera e celular fora do Google Fotos, quero que elas entrem na timeline na data em que foram tiradas.

**Acceptance Criteria** (EXIF-01, EXIF-02):

1. WHEN `DateTimeOriginal = "2022:08:10 11:37:00"`, `SubSecTimeOriginal = "5"` e `OffsetTimeOriginal = "-03:00"` THEN `taken_at_local` SHALL ser `"2022-08-10T11:37:00.500"`, `taken_at_offset` `"-03:00"`, `taken_at` `"2022-08-10T14:37:00.500Z"` e a tag `DateTimeOriginal`
2. WHEN não há `OffsetTimeOriginal` e há `GPSDateStamp = "2022:08:10"` + `GPSTimeStamp = 14/1 35/1 10/1` THEN o fuso SHALL ser `"-03:00"` (diferença arredondada a 15 min) e `taken_at` `"2022-08-10T14:37:00.000Z"`; WHEN a diferença arredondada passa de ±14:00 THEN o fuso SHALL ser desconhecido
3. WHEN não há fuso por nenhuma fonte THEN `taken_at_local` SHALL ser preenchido e `taken_at_offset`/`taken_at` SHALL ser ausentes
4. WHEN `DateTimeOriginal` falta ou é inválida e `DateTimeDigitized` é válida THEN SHALL usar `DateTimeDigitized` com `OffsetTimeDigitized`/`SubSecTimeDigitized` e reportar essa tag; WHEN só `DateTime` (IFD0) existe THEN não SHALL haver data
5. WHEN a data é `0000:00:00 00:00:00`, `2022:02:30 10:00:00`, `2022:08:10 24:00:00`, ano 1799, vazia ou lixo THEN é inválida; `2022-08-10 11:37:00` (com `-`) SHALL ser aceita
6. WHEN `OffsetTimeOriginal` é inválido (`"+15:00"`, `"3:00"`, `"  "`) THEN SHALL ser ignorado e o GPS SHALL ser tentado

**Independent Test**: JPEG gerado em memória com os campos de cada critério, gravado num diretório temporário.

---

### P1: GPS, dimensões e orientação ⭐ MVP

**User Story**: Como usuário, quero ver onde a foto foi tirada e que a grade use a proporção certa.

**Acceptance Criteria** (EXIF-03, EXIF-04):

1. WHEN `GPSLatitude = 23/1 33/1 1818/100` `S` e `GPSLongitude = 46/1 37/1 5988/100` `W` THEN latitude SHALL ser ≈ `-23.55505` e longitude ≈ `-46.63330` (erro < 1e-6)
2. WHEN `GPSAltitude = 760/1` e `GPSAltitudeRef = 1` THEN altitude SHALL ser `-760`; com `Ref = 0` ou ausente SHALL ser `760`; WHEN o denominador da altitude é 0 THEN altitude SHALL ser ausente e o ponto continua valendo
3. WHEN o `Ref` de latitude ou longitude falta, um denominador é 0, a latitude passa de 90, a longitude de 180, ou o ponto é `0/0` THEN latitude, longitude e altitude SHALL ser ausentes
4. WHEN a imagem é um JPEG/PNG cujo cabeçalho diz 4032×3024 THEN `width`/`height` SHALL ser 4032/3024, mesmo que o EXIF diga outra coisa
5. WHEN o arquivo é RAW (`.dng .cr2 .nef .arw`) ou o cabeçalho não é legível THEN SHALL usar `PixelXDimension`/`PixelYDimension`; sem eles, ausentes
6. WHEN `Orientation` é 1..8 THEN SHALL ser copiada; 0, 9 ou ausente → ausente

**Independent Test**: JPEG/PNG gerados com SOF/IHDR e EXIF escolhidos.

---

### P1: Arquivo estranho não derruba nada ⭐ MVP

**User Story**: Como usuário com fotos antigas e editadas, quero que um EXIF quebrado só deixe aquele dado de fora.

**Acceptance Criteria** (EXIF-05, EXIF-06):

1. WHEN a foto não tem EXIF (PNG sem `eXIf`, GIF, JPEG sem APP1) THEN SHALL devolver `Ok` com campos EXIF ausentes, `raw_json` ausente, `exif_corrupt = false` e dimensões do cabeçalho
2. WHEN o bloco EXIF existe mas é lixo THEN SHALL devolver `Ok` com `exif_corrupt = true`, campos EXIF ausentes e dimensões do cabeçalho
3. WHEN o arquivo não pode ser aberto THEN SHALL devolver `Err` com código `unreadable`
4. WHEN o EXIF é lido THEN `raw_json` SHALL ser um objeto JSON com `DateTimeOriginal`, `GPSLatitude` etc. como texto, sem `MakerNote`, e nenhum valor com mais de 1024 caracteres
5. WHEN a leitura termina THEN os bytes, o tamanho e a data de modificação do arquivo SHALL ser os de antes (EXIF-06)

**Independent Test**: bytes de lixo dentro do APP1; caminho inexistente; snapshot do arquivo antes/depois.

---

## Edge Cases

- WHEN `SubSecTimeOriginal` tem mais de 3 dígitos (`"123456"`) THEN usa os 3 primeiros (`.123`); com lixo, `.000`
- WHEN o fuso converte a data para o dia anterior/seguinte (`2022:01:01 01:00:00 +03:00`) THEN `taken_at` SHALL ser `"2021-12-31T22:00:00.000Z"`
- WHEN a latitude é exatamente 90 ou a longitude 180 THEN válida

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| EXIF-01 | Data e tag | Design | Pending |
| EXIF-02 | Fuso (OffsetTime, GPS) | Design | Pending |
| EXIF-03 | GPS | Design | Pending |
| EXIF-04 | Dimensões e orientação | Design | Pending |
| EXIF-05 | Sem EXIF, corrompido, ilegível, JSON bruto | Design | Pending |
| EXIF-06 | Só leitura | Design | Pending |

**Coverage:** 6 total, 0 mapped to tasks, 6 unmapped.

---

## Success Criteria

- [ ] Todos os critérios passam em `cargo test` com imagens geradas no próprio teste
- [ ] Nenhuma dependência nativa nova (só crates Rust puros)
