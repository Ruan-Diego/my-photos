# F07 EXIF Parser Validation

**Date**: 2026-10-06
**Spec**: `.specs/features/f07-exif-parser/spec.md`
**Design**: `.specs/features/f07-exif-parser/design.md`
**Diff range**: `9c40f58..0975d0c` (one commit, `0975d0c`: spec, design, tasks, `photo_meta/{mod.rs,date.rs,tests.rs}`, `time.rs` (`days_from_civil`, `iso_from_unix_millis`), `lib.rs` (`mod photo_meta`), `Cargo.toml`/`Cargo.lock` (`kamadak-exif` 0.6.1, `imagesize` 0.15.0), plus wording-only edits to the F05/F06 specs)
**Verifier**: independent sub-agent (author ≠ verifier)
**Environment**: Linux cloud container. `cargo test --lib -- photo_meta:: time::` → **33 passed, 0 failed** (photo_meta integration 19, photo_meta::date 7, time 7). Regression: `cargo test --lib -- scan:: takeout::` → 107 passed, 1 failed (the known Windows-only icacls test), 1 ignored, which is the same as before F07.

**Verdict**: ❌ FAIL

Every AC has a test, and most of them pin the spec outcome well (51 of 62 mutants killed). The FAIL has two causes:

1. **Two real defects**, both shown with temporary probes (see [Probes](#probes)):
   - **B1. HEIC dimensions come back rotated.** `imagesize` swaps width and height when the HEIF `irot` property is 90°/270°. A HEIF whose `ispe` is 4032×3024 with `irot=1` comes back as `width 3024, height 4032`. The spec says "a orientação é só reportada e as dimensões são as armazenadas" (Out of Scope, F12/F19 row). iPhone HEICs usually carry both `irot` and EXIF `Orientation=6`. A consumer that applies the orientation, as the spec intends, would rotate twice.
   - **B2. A read error after a successful open is reported as `Ok { exif_corrupt: true }`, not `Err(unreadable)`.** `read_exif` maps every `exif::Error::Io` to "corrupt" (`mod.rs:120`). On Linux, `read_embedded(<directory>)` returns `Ok` with `exif_corrupt = true` and no dimensions. On Windows the same call returns `Err(Unreadable)`, because `File::open` fails on a directory. The spec rows "Arquivo ilegível → `Err(unreadable)`" and "External-dependency failure: Disco → `unreadable`" are broken for an I/O failure in the middle of a read (bad sector, drive unplugged during a scan of `E:`). F10 would then record `corrupt_media` for a disk fault.
2. **7 spec-visible mutants survive**: M02 (partial EXIF thrown away), M15 (latitude exactly +90 rejected), M37 (GPS offset below −14:00 accepted), D12 (GPS offset floored, not rounded), D11 (`±HH?MM` with any separator accepted), M25 (thumbnail IFD1 merged into `raw_json`), M31 (longitude −180 rejected).

---

## Task Completion

| Task | Commit | Status | Notes |
| ---- | ------ | ------ | ----- |
| T1 `time` helpers | `0975d0c` | ✅ Done | `time.rs:21-39`. `days_from_civil` is Hinnant's algorithm, round-trip tested over every day from 1800 to 9999 (`time.rs:136-148`). `iso_from_unix_millis` checks the range and `millis ≤ 999` (`:150-160`). `iso_from_unix_seconds` now delegates with `millis = 0`, same output. The F05/F06 `time::` tests are unchanged and green, and `scan::`/`takeout::` are unchanged |
| T2 Date, sub-second, offset parsing | `0975d0c` | ⚠️ Done, weak tests | `date.rs`. Unit tests cover the format rules. Not pinned: GPS offset rounding vs floor (D12), the negative ±14 h bound (M37), the `:` in `±HH:MM` (D11) |
| T3 `read_embedded` | `0975d0c` | ⚠️ Done, defects + gaps | `mod.rs`. Defects B1 and B2 (above). No test for partial EXIF with usable fields (M02), latitude +90 (M15), longitude −180 (M31), or thumbnail fields kept out of `raw_json` (M25) |

`tasks.md` marks T1–T3 `[x]`. The code exists for each one, but see the gaps above.

---

## Spec-Anchored Acceptance Criteria

Test lines are in `src-tauri/src/photo_meta/tests.rs` unless another file is named. Every integration test writes a JPEG/PNG/GIF it generates itself into a `Scratch` temp dir. None uses `E:` or an external tool.

### P1: Data de captura com fuso (EXIF-01, EXIF-02)

| Criterion | Spec outcome | `file:line` + assertion | Result |
| --------- | ------------ | ----------------------- | ------ |
| AC1 `DateTimeOriginal` + `SubSecTimeOriginal "5"` + `OffsetTimeOriginal "-03:00"` | local `2022-08-10T11:37:00.500`, offset `-03:00`, UTC `2022-08-10T14:37:00.500Z`, tag `DateTimeOriginal` | `:163-172` all four asserted with `assert_eq!`. `:173-174` `as_tag()` strings. `:175` `!exif_corrupt` | ✅ PASS (M13, D07, D16, D18, T04 killed) |
| AC2 offset from GPS (`2022:08:10` + `14/1 35/1 10/1`); beyond ±14:00 → unknown | offset `-03:00`, UTC `…14:37:00.000Z`; out of range → none | `:184` `-03:00`, `:185-188` UTC. `:191-198` local 05:10 next day → difference +14:30 → `(None, None)` with local kept. `date.rs:286` exactly +14:00 → `Some(840)`. `date.rs:282` crossing midnight → `+180` | ⚠️ PASS for the positive side. **Not pinned**: (a) the negative bound. Mutant M37 (`minutes <= 840` with no `abs`) accepts −14:30 and survives. (b) "arredondada a 15 min". D12 (`floor` instead of `round`) survives, because both test differences (−11.88 and +0.46 quarters) round and floor the same way |
| AC3 no zone from any source → local only | `taken_at_local` set; offset/UTC absent | `:204-210` `taken_at_local == "2022-08-10T11:37:00.000"`, `(offset, taken_at) == (None, None)`, tag `Original` | ✅ PASS |
| AC4 Original missing/invalid → Digitized with **its** subsec/offset; only `DateTime` → no date | tag `DateTimeDigitized` | `:216-242`. Original absent, `0000:00:00 00:00:00`, `"lixo"`. The fixture also holds `SubSecTimeOriginal "9"` / `OffsetTimeOriginal "-05:00"` as decoys, so the test proves the Digitized subsec (`.250`) and offset (`+01:00`) are the ones used: UTC `2021-12-31T21:00:00.250Z`. `:243-247` `DateTime` only → `(None, None)` | ✅ PASS (M11, M12 killed). Discriminating |
| AC5 invalid dates; `-` separators accepted | `0000…`, `02:30`, `24:00:00`, 1799, empty, junk → absent; `2022-08-10 11:37:00` OK | `:253-263` the spec list (no empty string here) → `taken_at_local`/`tag` `None`. `:264-268` dashed date accepted. `date.rs:211-235` adds empty, blanks, 2023-02-29, 1900-02-29, 04-31, month 0/13, day 0, minute/second 60, mixed separators, `/`, `T`, short, trailing `Z`, `+022` | ✅ PASS (D01–D06, D19, T01, T02, T05 killed) |
| AC6 invalid `OffsetTimeOriginal` (`+15:00`, `3:00`, `"  "`) ignored, GPS tried | GPS offset `-03:00` | `:274-289` each → offset `-03:00`, UTC `…14:37:00.000Z`. `date.rs:256-269` adds `+14:01`, `+3:00`, `03:00`, `+03:60`, `+0300`, `Z`, empty | ⚠️ PASS for the listed values. **Not pinned**: the `:` at position 3. D11 (separator check removed) survives, so `+03-00` or `+03.00` would be accepted |

### P1: GPS, dimensões e orientação (EXIF-03, EXIF-04)

| Criterion | Spec outcome | `file:line` + assertion | Result |
| --------- | ------------ | ----------------------- | ------ |
| AC1 DMS + `S`/`W` | ≈ −23.55505 / −46.63330, error < 1e-6 | `:334-335` `abs() < 1e-6`. `:338-343` `N`/`E` give positive values | ✅ PASS (M20, M23 killed) |
| AC2 altitude 760, ref 1 → −760; ref 0 / absent → 760; denominator 0 → absent, point kept | | `:349-360` `(Some(1), -760)`, `(Some(0), 760)`, `(None, 760)`. `:361-365` `760/0` → `altitude None`, `latitude.is_some()` | ✅ PASS (M19, M22, M29 killed) |
| AC3 ref missing, denominator 0, lat > 90, lon > 180, `0/0` → lat/lon/alt absent | | `:371-430`: no lat ref, no lon ref, ref `X`, `33/0`, lat 90°0′1″, lon 180°0′1″, `0/0` (N/E), 2-value latitude. Each asserts `(None, None, None)` with a valid altitude present | ✅ PASS (M16, M18, M21 killed) |
| AC4 header 4032×3024 wins over EXIF (JPEG/PNG) | | `:470-477` EXIF `100×50`, JPEG SOF and PNG IHDR `4032×3024` → `(4032, 3024)` for both | ✅ PASS for JPEG/PNG. See B1 for HEIC (not testable with generated files; probe below) |
| AC5 RAW (`.dng .cr2 .nef .arw`) or unreadable header → `PixelX/YDimension`; without them → absent | | `:485-493` four RAW names (one upper case, `b.CR2`) with a 640×480 SOF → EXIF `6000×4000`. `:494-498` JPEG without SOF → EXIF. `:499-500` neither → `(None, None)` | ✅ PASS (M05, M06, M08 killed) |
| AC6 `Orientation` 1..8 copied; 0, 9, absent → absent | | `:506-509` 1..=8 copied. `:510-517` 0, 9 → `None`. `:518` absent → `None` | ✅ PASS (M09, M10 killed) |

### P1: Arquivo estranho não derruba nada (EXIF-05, EXIF-06)

| Criterion | Spec outcome | `file:line` + assertion | Result |
| --------- | ------------ | ----------------------- | ------ |
| AC1 no EXIF (PNG without `eXIf`, GIF, JPEG without APP1) | `Ok`, EXIF fields absent, `raw_json` absent, `exif_corrupt = false`, header size | `:526-542` the **whole struct** is compared with `assert_eq!` against `{width, height, ..default()}` for GIF 10×5, PNG 32×16, JPEG 640×480 | ✅ PASS (M01 killed). Strong assertion |
| AC2 EXIF block is garbage | `Ok`, `exif_corrupt = true`, EXIF fields absent, header size | `:548-558` three payloads (bad byte order, truncated IFD offset, IFD count past the end): `exif_corrupt`, size `640×480`, `taken_at_local None`, `raw_json None` | ✅ PASS (M03, M04, M27 killed). The "partial" assumption has no test. See below |
| AC3 cannot open → `Err(unreadable)` | | `:564-566` missing path → `Err(Unreadable)`, `as_code() == "unreadable"` | ✅ PASS for "cannot be opened" (M34 killed). **B2**: a read failure after the open is not `unreadable` |
| AC4 `raw_json`: object, text values, no `MakerNote`, ≤ 1024 chars | | `:580-597` parses as an object. `DateTimeOriginal` contains `2022`. `GPSLatitude`/`GPSLatitudeRef` are strings. No `MakerNote` key. Every value ≤ 1024 chars. A 3000-char `ImageDescription` → exactly 1024 | ⚠️ PASS for the listed items (M24, M26, M32 killed). **Not pinned**: "campos do IFD principal". M25 (thumbnail IFD1 fields merged in, overwriting same-named primary tags such as `Compression`/`XResolution`) survives, because no fixture has an IFD1 |
| AC5 bytes, size, mtime unchanged (EXIF-06) | | `:606-616` `(bytes, modified)` before == after two reads. `:617` no new file in the dir | ✅ PASS (M35 killed: appending one byte fails the test). Size is implied by the byte equality |

### Assumption rows with no AC but stated outcomes

| Row | Evidence | Result |
| --- | -------- | ------ |
| "Se o EXIF vier parcial, os campos lidos valem" (EXIF corrompido) + design table "EXIF parcial → campos lidos valem, `exif_corrupt`" | `mod.rs:115-119` keeps the partial `Exif` when it has fields. A probe confirms it: a valid IFD0 with a bad next-IFD offset → `orientation 6`, `taken_at_local` set, `raw_json` set, `exif_corrupt true` | ❌ **No test**. M02 (`(None, true)` for every partial result) survives. The garbage fixtures only produce partial results with zero fields |
| `GPSAltitudeRef` only with a valid point | `:422-429` altitude present but point invalid → altitude `None` | ✅ |

---

## Edge Cases

| Edge case | Evidence | Result |
| --------- | -------- | ------ |
| `SubSecTimeOriginal "123456"` → `.123`; junk → `.000` | `:296-304` `"123456"` → `2022-01-01T01:00:00.123`. `:309-316` `"abc"` → `.000`. `date.rs:240-246` adds `"5"`, `"05"`, `" 42 "`, `""`, `"1a"`, `"-5"`, `"1.5"` | ✅ PASS (D07–D09 killed) |
| `2022:01:01 01:00:00 +03:00` → `2021-12-31T22:00:00.000Z` | `:305-308` (with `.123` millis) `2021-12-31T22:00:00.123Z`. `date.rs:310-313` the exact spec value `…22:00:00.000Z` | ✅ PASS (D16 killed) |
| Latitude exactly 90, longitude 180 → valid | `:436-445` `(90°S, 180°E)` → `(-90, 180)`. `:446-455` a one-zero coordinate `(0, 10)` is kept | ⚠️ PASS for −90 and +180 only. **Not pinned**: latitude **+90** (N), which is the spec's literal "exatamente 90". M15 (range end-exclusive at +90) survives. Longitude −180 (W): M31 survives |

---

## Implementation Review

Confirmed by reading the code and the crate sources (`~/.cargo/registry/src/*/kamadak-exif-0.6.1`, `imagesize-0.15.0`):

- **Date parsing** (`date.rs:33-70`). Exactly 19 bytes. Separator at `[4]` is `:` or `-`, and `[7]` must match it. Then space, `:`, `:`. Digits only. Real calendar with Gregorian leap rules (`:170-177`). Hours ≤ 23, minutes and seconds ≤ 59, years 1800–9999. There is no trimming, so a value padded with a trailing space (`"2022:08:10 11:37:00 "`) is rejected (probe). That matches the strict format in the spec.
- **Sub-seconds** (`:73-83`). ASCII whitespace is trimmed, the value must be all digits, and the first 3 digits are taken, right-padded. Max 999, so `iso_from_unix_millis` never sees 1000.
- **OffsetTime** (`:86-102`). `[+-]HH:MM`, minutes ≤ 59, total ≤ 840. `+14:00` and `-14:00` are accepted, `+14:01` is rejected.
- **GPS offset** (`:106-134`). `local_as_if_utc − gps_utc` is rounded half away from zero to 15-minute steps and accepted if `|·| ≤ 840`. GPS seconds are floored. The time ranges reject NaN. The date must be `AAAA:MM:DD` and a real calendar date. Crossing midnight works because both sides are absolute seconds.
- **UTC** (`:137-139`). Wall time minus the offset, then `iso_from_unix_millis`. If the UTC instant falls outside 1800–9999, `mod.rs:181-186` drops both the offset and `taken_at` and keeps the local time. That is consistent.
- **Fallback** (`mod.rs:146-187`). `find_map` picks the first tag whose **date** parses, and the matching SubSec/Offset tags travel with it. An invalid `OffsetTimeOriginal` does not make it switch to `OffsetTimeDigitized`, which is correct ("da mesma tag"). The IFD0 `DateTime`/`OffsetTime` are never read.
- **GPS** (`mod.rs:190-215`). Exactly 3 rationals. Any zero denominator voids the tag (`:229-237`). The ref is the first ASCII byte, `N`/`S` or `E`/`W`, and anything else voids the point. Range and `0/0` checks come after the sign is applied. Altitude is read only for a valid point. Ref `1` → negative, any other value or none → positive.
- **Dimensions** (`mod.rs:80-93`, `:134-142`). RAW is decided by extension, case-insensitive, and RAW never calls `imagesize`. The EXIF fallback runs when either header value is missing. Zero sizes are not filtered: a JPEG SOF with height 0 (DNL) gives `height Some(0)` (probe).
- **Orientation** `get_uint(0)` accepts SHORT or LONG, then the 1..=8 filter is applied.
- **raw_json** (`mod.rs:243-254`). IFD0 + Exif + GPS (kamadak gives sub-IFD fields `In::PRIMARY`). `MakerNote` is skipped. Values are kamadak's `display_value().with_unit()`, cut by `chars()`. Note: these are display strings, not the raw bytes. Dates are reformatted to `2022-08-10 11:37:00`, and ASCII values are quoted (`"\"X\""`). That fits "valor legível".
- **Container dispatch** (`reader.rs:138-158`). GIF and other unknown formats give `InvalidFormat("Unknown image format")`, which `mod.rs:112` treats as "no EXIF". Matching on that exact `&'static str` is brittle: `Cargo.toml` allows any `0.6.x`, and `Cargo.lock` pins 0.6.1.
- **Read-only**. `File::open` and `imagesize::size` both open read-only. Nothing writes.

### Defects

| # | Severity | Where | Finding |
| - | -------- | ----- | ------- |
| **B1** | High (HEIC, the iPhone format) | `mod.rs:89` → `imagesize-0.15.0/src/container/heif.rs:66-91` | `imagesize`'s HEIF reader applies `irot`: when the rotation is 1 or 3 it swaps width and height. The result is the **displayed** size, not the stored one. Probe: `ispe 4032×3024` + `irot 1` → `width 3024, height 4032`. The spec says dimensions are "as armazenadas" and orientation is only reported. iPhone HEICs carry `irot` and EXIF `Orientation` 6/8, so a consumer that applies `orientation` would rotate twice. It also picks the largest `ispe` by area, which is fine for grids. The generated-file tests cannot reach this path. **Fix**: for HEIF, read `ispe` without `irot` (or undo the swap), or change the spec/F12 contract to say HEIF sizes are already display-oriented. Confirm on a real iPhone HEIC either way |
| **B2** | Medium | `mod.rs:103-121` (`Err(_) => (None, true)`) | `exif::Error::Io` (a read failure after a successful open) becomes `exif_corrupt = true` inside an `Ok`. The spec says "Arquivo ilegível → `Err(unreadable)`" and "Disco: `unreadable`". Probe on Linux: `read_embedded(dir)` → `Ok { exif_corrupt: true, width: None, … }`. On Windows the same call is `Err(Unreadable)`. A bad sector or a drive unplugged mid-read would be logged by F10 as corrupt media. Note: kamadak already turns `UnexpectedEof` into `InvalidFormat("Broken … file")` for JPEG/PNG/WebP, so a remaining `Error::Io` is a real I/O fault. Mapping it to `Err(Unreadable)` is enough |

### Risks and observations (not counted toward the verdict)

| # | Severity | Where | Finding |
| - | -------- | ----- | ------- |
| R1 | Medium | `kamadak webp.rs:69-73` | A WebP `EXIF` chunk that starts with `Exif\0\0` gives "Invalid TIFF byte order", so `exif_corrupt = true` and no data. Some encoders write that prefix. A plain-TIFF WebP EXIF chunk works: the probe got the date, orientation 6, and 4032×3024 from VP8X. Possible fix: strip a leading `Exif\0\0` and retry with `read_raw` |
| R2 | Low–Medium (performance) | `kamadak reader.rs:143`; `jpeg.rs:69-97` | The design says both reads "param no começo do arquivo". That is not true. For TIFF/RAW (`.dng .cr2 .nef .arw .tif`), `read_from_container` reads the **whole file** into memory. For a JPEG without EXIF it scans the full entropy-coded data up to EOI. This costs time and memory on the F10 scan of `E:` (tens of MB per RAW) |
| R3 | Low | `mod.rs:120` via `jpeg.rs:56` | A truncated JPEG **with no EXIF block** (no EOI) is flagged `exif_corrupt = true` (probe). The spec says corrupt means "o bloco EXIF existe mas é lixo". Arguably fine, since the file is broken, but it is not what the AC says |
| R4 | Low | `mod.rs:90-91`, `:139-140` | Width/height 0 (JPEG DNL, `PixelXDimension 0`) is passed through. The grid would divide by 0 unless F12 guards against it |
| R5 | Low | `imagesize webp.rs:52-58` | Simple lossy WebP (VP8): the 14-bit size is read as u16 without masking the 2 scale bits. Non-zero scale bits, which are rare, inflate the size |
| R6 | Info | `mod.rs:112` | Matching on `InvalidFormat("Unknown image format")` depends on a string inside the crate. A 0.6.x update that rewords it would turn every GIF into `exif_corrupt`. M01 would catch that |
| R7 | Info | `mod.rs:210` | The GPS ref is compared byte for byte (`N`, not `n`). This is stricter than needed, but it matches the spec |

---

## Probes

These are not part of the suite. I appended a temporary test to `tests.rs`, ran it with `--nocapture`, and restored the file with `git checkout -- src-tauri/src/photo_meta/tests.rs`.

| Input | `read_embedded` result |
| ----- | ---------------------- |
| HEIF (`ftyp heic/mif1`, `meta/iprp/ipco{ispe 4032×3024, irot 1}`) | `width 3024, height 4032`, `exif_corrupt false` → **B1** |
| WebP VP8X 4032×3024 + `EXIF` chunk (plain TIFF: DTO + Orientation 6) | date, orientation 6, 4032×3024, `raw_json` set, not corrupt ✅ |
| Same with an `Exif\0\0` prefix in the chunk | `exif_corrupt true`, no EXIF fields → R1 |
| JPEG, IFD0 valid (Orientation 6, Make, DTO) with next-IFD offset `0xFFFFFF00` | `orientation 6`, `taken_at_local` set, `raw_json` set, `exif_corrupt true` ✅ (but no test; M02) |
| A directory named `dir.jpg` (Linux) | `Ok { exif_corrupt: true, everything None }` → **B2** |
| JPEG with no APP1, EOI removed | `exif_corrupt true`, 640×480 → R3 |
| `DateTimeOriginal "2022:08:10 11:37:00 "` (trailing space) | `taken_at_local None` (strict format) |
| JPEG SOF 640×0 | `height Some(0)` → R4 |

---

## Discrimination Sensor (mutation log)

Each mutant was applied alone with a literal `perl` substitution (script in the session scratchpad) and checked as applied (`git diff` non-empty). Then `cargo test --lib -- photo_meta:: time::` was run, and the file was restored with `git checkout -- <file>`. Baseline: 33 passed, 0 failed. A mutant is **killed** when ≥ 1 test fails. Afterwards `git status --short` showed only this report.

### `photo_meta/mod.rs`

| ID | Line | Mutation | Result | Killed by / note |
| -- | ---- | -------- | ------ | ---------------- |
| M01 | 112 | drop the `"Unknown image format"` arm (GIF → corrupt) | ✅ Killed | `exif05_photos_without_exif…` |
| **M02** | 118 | partial result → `(None, true)` (discard partial fields) | ❌ **Survived** | — Assumptions "Se o EXIF vier parcial, os campos lidos valem" |
| M03 | 117 | `usable = true` (empty partial → `raw_json "{}"`) | ✅ Killed | `exif05_garbage…` |
| M04 | 118 | partial → `corrupt = false` | ✅ Killed | `exif05_garbage…` |
| M05 | 88 | RAW also uses the header | ✅ Killed | `exif04_raw…` |
| M06 | 86 | RAW extension case-sensitive | ✅ Killed | `exif04_raw…` (`b.CR2`) |
| M07 | 134 | fallback `\|\|` → `&&` | ⚪ Survived, equivalent in practice | `imagesize` returns both values or neither; a single `None` needs a size > u32::MAX |
| M08 | 134 | no EXIF dimension fallback | ✅ Killed | `exif04_raw…` |
| M09 | 132 | orientation `0..=8` | ✅ Killed | `exif04_orientation…` |
| M10 | 132 | orientation `1..=9` | ✅ Killed | `exif04_orientation…` |
| M11 | 158 | Digitized uses `OffsetTimeOriginal` | ✅ Killed | `exif01_digitized…` |
| M12 | 157 | Digitized uses `SubSecTimeOriginal` | ✅ Killed | `exif01_digitized…` |
| M13 | 175 | `OffsetTime*` ignored | ✅ Killed | `exif01_original…`, `exif01_digitized…`, `edge_long_subsec…` |
| M14 | 176 | no GPS fallback | ✅ Killed | `exif02_offset_comes_from_gps…`, `exif02_invalid_offset…` |
| **M15** | 193 | latitude range `-90.0..90.0` (+90 rejected) | ❌ **Survived** | — Edge case "latitude exatamente 90 → válida"; only −90 is tested |
| M16 | 193 | longitude range ±181 | ✅ Killed | `exif03_unusable…` |
| M17 | 194 | `0/0` → `\|\|` (one zero invalidates) | ✅ Killed | `edge_gps_range_ends…` (equator) |
| M18 | 194 | `0/0` check removed | ✅ Killed | `exif03_unusable…` |
| M19 | 200 | altitude ref `0` negates | ✅ Killed | `exif03_altitude…` |
| M20 | 212 | `S`/`W` not negated | ✅ Killed | `exif03_degrees…`, `edge_gps…` |
| M21 | 213 | unknown ref accepted as positive | ✅ Killed | `exif03_unusable…` (`X`, missing ref) |
| M22 | 233 | zero denominators allowed | ✅ Killed | `exif03_altitude…` (`760/0` → ∞) |
| M23 | 209 | seconds `/ 60` | ✅ Killed | `exif03_degrees…` |
| M24 | 246 | `MakerNote` kept | ✅ Killed | `exif05_raw_json…` |
| **M25** | 246 | thumbnail IFD1 fields kept in `raw_json` | ❌ **Survived** | — Assumptions "campos do IFD principal"; IFD1 tags (`Compression`, `XResolution`, …) would overwrite the primary values under the same key |
| M26 | 250 | no truncation | ✅ Killed | `exif05_raw_json…` |
| M27 | 78 | `exif_corrupt` always false | ✅ Killed | `exif05_garbage…` |
| M28 | 179 | GPS hour/minute swapped | ✅ Killed | `exif02_*` |
| M29 | 200 | any altitude ref negates | ✅ Killed | `exif03_altitude…` |
| M30 | 177 | GPS seconds dropped | ⚪ Survived, near-equivalent | Absorbed by 15-minute rounding, except within seconds of a 7′30″ boundary. Not spec-visible in practice |
| **M31** | 193 | longitude range `-179.5..=180.0` (−180 rejected) | ❌ **Survived** | — Edge case "longitude 180 → válida"; only +180 (E) is tested |
| M32 | 250 | truncate at 1023 | ✅ Killed | `exif05_raw_json…` |
| M33 | 223 | `values.first()` → `last()` | ⚪ Survived, equivalent | EXIF date/offset/ref tags hold one string |
| M34 | 75 | missing file → `Ok(default)` | ✅ Killed | `exif05_missing_file…` |
| M35 | 98 | append one byte to the file after reading | ✅ Killed | `exif06_reading_never_changes_the_file` |
| M36 | 211 | GPS ref accepted case-insensitively | ⚪ Survived, benign | Accepting `n`/`s` is lenient. The spec does not list lowercase as invalid |

### `photo_meta/date.rs`

| ID | Line | Mutation | Result | Killed by / note |
| -- | ---- | -------- | ------ | ---------------- |
| D01 | 38 | only `:` separator | ✅ Killed | `exif_datetime_accepts…`, `exif01_invalid_dates…` |
| D02 | 39 | mixed separators allowed | ✅ Killed | `exif_datetime_rejects…` |
| D03 | 52 | year ≥ 1799 | ✅ Killed | `exif_datetime_rejects…`, `exif01_invalid_dates…` |
| D04 | 55 | day ≤ 31 for every month | ✅ Killed | same |
| D05 | 172 | leap year = `% 4` only | ✅ Killed | `exif_datetime_rejects…` (1900-02-29) |
| D06 | 56 | hour ≤ 24 | ✅ Killed | `exif_datetime_rejects…`, `exif01_invalid_dates…` |
| D07 | 79 | 2 sub-second digits | ✅ Killed | `subsec…`, `exif01_original…`, `edge_long_subsec…` |
| D08 | 75 | sub-second junk check removed | ✅ Killed | `subsec…`, `edge_long_subsec…` |
| D09 | 74 | sub-second not trimmed | ✅ Killed | `subsec…` (`" 42 "`) |
| D10 | 98 | offset bound `hours > 14` (`+14:30` accepted) | ✅ Killed | `offsets_are_bounded…` (`+14:01`) |
| **D11** | 87 | `text[3] != ':'` check removed | ❌ **Survived** | — Assumptions "`±HH:MM`"; `+03-00` / `+03.00` accepted |
| **D12** | 108 | GPS offset `round` → `floor` | ❌ **Survived** | — EXIF-02 AC2 "arredondada a 15 min"; e.g. +2:58 becomes `+02:45`, not `+03:00` |
| D13 | 108 | `round` → `trunc` | ✅ Killed | `gps_offset_rounds…`, `exif02_*` |
| D14 | 110 | `< 840` (exactly 14 h rejected) | ✅ Killed | `gps_offset_rounds…` |
| D15 | 132 | GPS seconds rounded, not floored | ✅ Killed | `gps_date_and_time_must_be_real` (59.9 s) |
| D16 | 138 | offset added, not subtracted | ✅ Killed | 6 tests |
| D17 | 143 | `offset_text(0)` → `-00:00` | ✅ Killed | `offsets_are_bounded…` |
| D18 | 26 | millis not zero-padded | ✅ Killed | 4 tests |
| D19 | 58 | second ≤ 60 | ✅ Killed | `exif_datetime_rejects…` |
| D20 | 121 | GPS hour 24 allowed | ✅ Killed | `gps_date_and_time_must_be_real` |
| **M37** | 110 | `minutes.abs() <= 840` → `minutes <= 840` | ❌ **Survived** | — EXIF-02 AC2 "passa de ±14:00 → desconhecido"; −14:30 accepted. Only the positive side is tested |

### `time.rs`

| ID | Line | Mutation | Result | Killed by |
| -- | ---- | -------- | ------ | --------- |
| T01 | 31 | `month <= 2` → `< 2` | ✅ Killed | `days_from_civil_inverts…` |
| T02 | 38 | epoch constant −1 | ✅ Killed | 9 tests |
| T03 | 22 | `millis > 1000` | ✅ Killed | `millis_are_formatted_and_bounded` |
| T04 | 25 | millis ignored | ✅ Killed | 5 tests |
| T05 | 37 | century term dropped | ✅ Killed | `days_from_civil_inverts…`, `utc_conversion…` |

**Totals**: 62 mutants: 36 in `mod.rs` (M01–M36), 21 in `date.rs` (D01–D20, plus M37), 5 in `time.rs`. **51 killed**. **11 survived**: 4 equivalent or benign (M07, M30, M33, M36) and **7 spec-visible (M02, M15, M25, M31, M37, D11, D12)**.

---

## Not Run

| Item | Reason |
| ---- | ------ |
| Real HEIC/HEIF, WebP, DNG/CR2/NEF/ARW files | Cannot be generated in tests (design Risks). HEIF and WebP paths were checked with hand-built minimal containers (Probes) and by reading the crate code |
| Windows behaviour of B2 | Linux container. From the std docs: on Windows `File::open(dir)` fails → `Err(Unreadable)` |
| `takeout::tests::unreadable_subdirectory…` | Needs Windows `icacls`; known, not F07 |
| Full `cargo test` | 42 known Linux-only failures in `bench::`/`db::`/`takeout::`; not F07 |

---

## Required to reach PASS

**Production fixes** (the defects):

1. **B1 (HEIC dims).** Make HEIF dimensions the stored ones. Either parse `ispe` without applying `irot`, or undo the swap when `irot` is 1 or 3. Or, if the stakeholder prefers display dimensions for HEIF, amend the spec and the F12 contract so that HEIF `orientation` is not applied twice. Add a test with a hand-built HEIF (`ftyp` + `meta/iprp/ipco{ispe, irot}`, as in the probe).
2. **B2 (I/O error).** In `read_exif`, return a distinct outcome for `exif::Error::Io(_)` that `read_embedded` maps to `Err(EmbeddedError::Unreadable)`. Test it with `read_embedded(<directory>)` under `#[cfg(unix)]`, asserting `Err(Unreadable)`.

**Tests** (one assertion each):

3. **M02.** Use the probe fixture (valid IFD0, next-IFD offset past the end): assert `exif_corrupt`, `orientation == Some(6)`, `taken_at_local` set, `raw_json` set.
4. **M15 / M31.** Assert `(90°N, 180°W)` → `(Some(90.0), Some(-180.0))`.
5. **M37.** Add a GPS case 14:30 behind the wall clock → offset `None` (unit or integration).
6. **D12.** Add a GPS case with a positive difference just below a quarter (e.g. local 17:33, UTC 14:35:10, +2:57:50) → `+03:00`.
7. **D11.** Add `b"+03-00"` and `b"+03.00"` to the `parse_offset` rejection list.
8. **M25.** Write a TIFF with an IFD1 field (`Field { ifd_num: In::THUMBNAIL, … }`, e.g. `Compression` or `XResolution` with a value different from IFD0). Assert that `raw_json` holds the IFD0 value, or no such key when IFD0 lacks it.

Recommended, not blocking: R1 (`Exif\0\0`-prefixed WebP), R2 (whole-file reads for TIFF/RAW, important for the F10 scan), R4 (zero dimensions).

## Requirement Status

| Requirement | Status |
| ----------- | ------ |
| EXIF-01 | ✅ Verified |
| EXIF-02 | ❌ Needs tests: negative ±14 h bound (M37), rounding (D12), offset separator (D11) |
| EXIF-03 | ⚠️ Needs a test: +90 latitude / −180 longitude (M15, M31) |
| EXIF-04 | ❌ B1: HEIC dimensions come back rotated (`irot` applied) |
| EXIF-05 | ❌ B2: I/O error → `exif_corrupt`, not `unreadable`. Partial EXIF untested (M02). `raw_json` IFD1 not pinned (M25) |
| EXIF-06 | ✅ Verified |

**Overall**: ❌ FAIL. The date, offset, GPS and dimension logic is careful, and 51 of 62 mutants are killed. Two defects remain: HEIC dimensions rotated by `irot` (B1), and mid-read I/O errors reported as corrupt EXIF (B2). Seven spec-visible mutants survive. All of them can be fixed with small, local changes (see above).

---

## Iteration 2 (re-verification)

**Diff range**: `0975d0c..c544197` (`c544197`). It changes `photo_meta/mod.rs` (header sizes now come from an extension allowlist `HEADER_SIZE_EXTENSIONS = jpg jpeg png gif webp`; `read_exif` returns `Err(Unreadable)` for `exif::Error::Io` other than `UnexpectedEof`; sizes must be > 0 and come as a pair; `#![allow(dead_code)]`), adds 9 tests to `tests.rs` and two bad offsets in `date.rs`, rewrites the leap-year test with `is_multiple_of` (same logic), and edits `spec.md` and `design.md`.
**Gate**: `cargo test --lib -- photo_meta:: time::` → **42 passed, 0 failed** (33 before, plus 9). `scan::` + `takeout::` → 107 passed, 1 failed (the known icacls test), the same as before. `cargo clippy --lib --tests`: no warnings in `photo_meta` (the 7 remaining are in `bench::`/`db::`). Note: `is_multiple_of` on unsigned integers needs Rust ≥ 1.87. The local toolchain is 1.97, and `Cargo.toml` sets no `rust-version`.

### Iteration-1 defects

| # | Fix | Evidence | Result |
| - | --- | -------- | ------ |
| B1 HEIC sizes rotated | `.heic`/`.heif` are no longer in the header allowlist, so their size comes from EXIF `PixelX/YDimension` (`mod.rs:22-25`, `:86-101`) | The same hand-built HEIF as before (`ispe 4032×3024`, `irot 1`) saved as `a.heic` → `(None, None)` (no EXIF, so no size; no rotated size). The new test `exif04_heif_size_comes_from_exif_not_the_header` (`tests.rs:623`) uses a JPEG body named `.heic`/`.HEIF`: EXIF `4032×3024` beats the 640×480 header. N03 (adding `heic` to the allowlist) is killed | ✅ Fixed for HEIF files with a HEIF extension. **Residual**: the choice is made by extension, while `imagesize` detects the format by content. A HEIF saved as `.jpg` still comes back rotated (probe: `b.jpg` with the HEIF bytes → `(3024, 4032)`). See R8 |
| B2 disk error → corrupt | `mod.rs:111-134`: `Error::Io(e)` with `e.kind() != UnexpectedEof` → `Err(Unreadable)` | `exif05_a_disk_error_after_opening_is_unreadable` (`tests.rs:651`): `read_embedded(<directory>)` → `Err(Unreadable)`. N01 (arm disabled) is killed. `exif05_truncated_photo_is_flagged_corrupt` (`:659`): JPEG without EOI → `exif_corrupt`, `raw_json None` | ✅ Fixed. Note: the `UnexpectedEof` guard can never trigger. kamadak's JPEG/PNG/WebP/HEIF readers already turn `UnexpectedEof` into `InvalidFormat("Broken … file")` (`jpeg.rs:56`, `png.rs:43`, `webp.rs:43`, `isobmff.rs:51`), and the TIFF path uses `read_to_end`, which never returns it. So N02 (guard removed) is equivalent. Truncated files stay `exif_corrupt` because of the `InvalidFormat` mapping, not because of the guard |
| R4 zero sizes | `positive()` for the header, `> 0` filters for EXIF; the header pair must be complete | `exif04_zero_sizes_are_absent` (`:639`): PNG 0×16 → `(None, None)`; DNG with EXIF `0×3024` → `(None, None)`. N04, N05, N07 killed | ✅ Mostly pinned. N06 (Y filter only) survives, see below |

### Iteration-1 survivors

| Mutant | New test (`tests.rs`) | Result |
| ------ | --------------------- | ------ |
| M02 partial EXIF discarded | `:669` `exif05_partly_readable_exif_keeps_what_was_read`: the `Make` value points out of the block → `exif_corrupt`, `taken_at_local` set, `raw_json` contains `Modelo Bom` and not `Fabricante` | ✅ Killed (also M04) |
| M15 lat +90 rejected | `:699` `edge_north_pole_and_antimeridian_west_are_valid`: `(90 N, 180 W)` → `(90.0, -180.0)` | ✅ Killed |
| M31 lon −180 rejected | same | ✅ Killed |
| M25 IFD1 fields in `raw_json` | `:714` `exif05_raw_json_ignores_thumbnail_fields`: IFD1 `Make "Miniatura"` and `Software "SoMiniatura"` are absent; IFD0 `Make "Principal"` is present | ✅ Killed |
| M37 −14:30 accepted | `:735` `exif02_gps_offset_beyond_minus_fourteen_hours_is_unknown`: local 00:05 vs UTC 14:35 → `(None, None)` | ✅ Killed |
| D12 floor | `:745` `exif02_gps_offset_rounds_to_the_nearest_quarter_hour`: +2:58 → `+03:00`, UTC `14:33:10.000Z`. Also D21 (`ceil`) killed | ✅ Killed |
| D11 offset separator | `date.rs:268-269` `+03-00`, `+03.00` rejected | ✅ Killed |

### Full mutation set, re-run

All 62 iteration-1 mutants were re-applied with the same script. M05, M06 and D05 were re-targeted at the new code: `if header_size`, `known.eq_ignore_ascii_case`, `is_multiple_of`. I added 11 mutants for the new code (N01–N10, D21). Each was applied alone and checked as applied, the gate was run, and the file was restored with `git checkout -- <file>`.

| ID | Mutation | Result | Note |
| -- | -------- | ------ | ---- |
| M01–M05, M08–M29, M31, M32, M34, M35, M37, D01–D21, T01–T05 | (as in iteration 1; M02, M15, M25, M31, M37, D11, D12 now killed) | ✅ Killed | |
| M07, M30, M33, M36 | (as in iteration 1) | ⚪ Survived, equivalent/benign | Unchanged reasoning |
| **M06** | header extension match case-sensitive (`*known == extension`) | ❌ **Survived** | Killed in iteration 1 by `b.CR2`, but RAW is no longer in the matched list. Every header test uses lower case (`.jpg`, `.png`, `.gif`). Cameras write `IMG_0001.JPG`. With the mutant, such a file takes EXIF `PixelX/Y` over its header. EXIF-04 AC4. The code is correct (probe: `C.JPG` → 4032); only the test is missing |
| N01 | `Io` → `Unreadable` arm disabled | ✅ Killed | `exif05_a_disk_error…` |
| N02 | `UnexpectedEof` guard removed | ⚪ Survived, equivalent | No container reader can return `Io(UnexpectedEof)` (see B2 above) |
| N03 | `heic`, `heif` added to the header list | ✅ Killed | `exif04_heif_size…` |
| N04 | header zero filter removed | ✅ Killed | `exif04_zero_sizes…` |
| N05 | EXIF X > 0 filter removed | ✅ Killed | `exif04_zero_sizes…` |
| **N06** | EXIF Y > 0 filter removed | ❌ **Survived** | The test only has X = 0. With `PixelX 4032, PixelY 0` the mutant gives `(4032, 0)`. Spec Assumptions: "Largura e altura vêm juntas e maiores que 0" |
| N07 | header sizes set one by one (no pair rule) | ✅ Killed | `exif04_zero_sizes…` |
| **N08** | `webp` removed from the header list | ❌ **Survived** | No WebP test. The spec says `imagesize` reads the WebP header. A WebP would fall back to EXIF or no size. Probe from iteration 1 shows the code reads VP8X correctly |
| **N09** | `jpeg` removed from the header list | ❌ **Survived** | No `.jpeg` test. EXIF-04 AC4 covers JPEG by format, not by one spelling |
| N10 | `tif`, `tiff` added to the header list | ⚪ Survived, not spec-visible | The spec does not say how non-RAW TIFF gets its size (see R9) |

**Totals**: 73 mutants (62 re-run + 11 new). **63 killed**. **10 survived**: 6 equivalent or benign (M07, M30, M33, M36, N02, N10) and **4 spec-visible (M06, N06, N08, N09)**. All 4 are in the extension allowlist and size-pair code that iteration 2 added, and each is test-only to close.

### New residual risks and doc consistency

| # | Severity | Where | Finding |
| - | -------- | ----- | ------- |
| R8 | Low–Medium | `mod.rs:86-101` | The format is chosen by **extension**, while `imagesize` and kamadak detect it by **content**. A HEIF with a `.jpg`/`.jpeg` name (renamed or mislabelled exports) still gets the `irot`-rotated size (probe: `(3024, 4032)`). Content-based fix: skip the header when `imagesize::image_type(path)` is `ImageType::Heif(_)` (and when the content is TIFF), whatever the extension. That also makes M06/N09 moot |
| R9 | Low (spec gap) | `spec.md` Assumptions "Dimensões"; `mod.rs:25` | Non-RAW TIFF (`.tif`/`.tiff`), which the Goals list, is in neither branch. With no `PixelXDimension` (normal for plain TIFF) its size is absent, even though IFD0 `ImageWidth`/`ImageLength` are right there (probe: `e.tif` with 800×600 in IFD0 → `(None, None)`, `raw_json` shows `ImageWidth "800 pixels"`). Either add `tif`/`tiff` to the header list (`imagesize` reads IFD0) or state in the spec that TIFF sizes come only from EXIF |
| R10 | Info | `design.md:16`, `:76`, `:96` | Not updated for iteration 2. The flowchart still says "dimensões: imagesize (não-RAW)". The component note still says "RAW = extensão `dng cr2 nef arw`", but the code now uses a header allowlist. The error table has no "erro de disco durante a leitura → `Err(Unreadable)`" row. `spec.md` is consistent: Assumptions "Dimensões" and "Arquivo ilegível", EXIF-04 AC5 and EXIF-05 AC3 all match the code |

The spec edits are consistent with the code, with one exception: "Para HEIC/HEIF … `PixelXDimension`" is enforced by extension only (R8).

Afterwards `git status --short` showed only this report modified. (The iteration-1 report was committed by the author in `c544197`.)

### Required to reach PASS

Test-only, one assertion each:

1. **M06 / N09.** In `exif04_header_size_wins_over_exif_dimensions`, add `C.JPG` and `d.jpeg` (EXIF 100×50, header 4032×3024 → 4032×3024).
2. **N08.** Add a WebP VP8X fixture (header 4032×3024, an `EXIF` chunk with `PixelX/Y` 100×50) → 4032×3024. The iteration-1 probe has a ready builder.
3. **N06.** In `exif04_zero_sizes_are_absent`, add EXIF `4032×0` → `(None, None)`.

Recommended, not blocking: R8 (choose the size source by content), R9 (state how TIFF gets its size), R10 (update design.md).

### Requirement status (iteration 2)

| Requirement | Iteration 1 | Iteration 2 |
| ----------- | ----------- | ----------- |
| EXIF-01 | ✅ | ✅ Verified |
| EXIF-02 | ❌ | ✅ Verified (M37, D11, D12 killed) |
| EXIF-03 | ⚠️ | ✅ Verified (M15, M31 killed) |
| EXIF-04 | ❌ (B1) | ⚠️ B1 fixed for `.heic/.heif`. Header allowlist not pinned for `.JPG`, `.jpeg`, `.webp` (M06, N09, N08); EXIF Y = 0 not pinned (N06). R8 residual |
| EXIF-05 | ❌ (B2, M02, M25) | ✅ Verified (B2 fixed; M02, M25, N01 killed) |
| EXIF-06 | ✅ | ✅ Verified |

### Summary (iteration 2)

**Overall**: ❌ FAIL, narrowly. B1 and B2 are fixed and verified with probes and new tests. All 7 iteration-1 spec-visible survivors are killed. 63 of 73 mutants are killed, and the 6 other survivors are equivalent. The FAIL rests on 4 spec-visible survivors in the new header allowlist code: `.JPG` upper case (M06), `.jpeg` (N09), `.webp` (N08), and EXIF height 0 alone (N06). No production bug is open. All four close with test-only additions. R8 (a HEIF with a `.jpg` name) is a residual risk I recommend fixing by choosing the size source by content.
