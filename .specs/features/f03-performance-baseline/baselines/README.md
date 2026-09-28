# Baselines locais

Medidos em 2026-09-28 com `src-tauri/target/debug/f03-bench.exe`, o binário de `cargo run --manifest-path src-tauri/Cargo.toml --bin f03-bench` (sem `--release`). Cada raiz era um diretório vazio criado para a corrida e deixou de existir ao terminar.

| Arquivo | volumeClass | itemCount | filesHashed | bytesPerItem | failureCount | ok | volumeRoot | walkElapsedMs | hashElapsedMs |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `ssd-50000.json` | ssd | 50000 | 50000 | 4096 | 0 | true | `C:\` | 1672 | 507 |
| `ssd-150000.json` | ssd | 150000 | 150000 | 4096 | 0 | true | `C:\` | 4010 | 1527 |
| `hdd-50000.json` | hdd | 50000 | 50000 | 4096 | 0 | true | `D:\` | 2058 | 3051 |
| `hdd-150000.json` | hdd | 150000 | 150000 | 4096 | 0 | true | `D:\` | 6186 | 8071 |

`C:` é `ssd`. As corridas usaram um subdiretório vazio em `%TEMP%`, no mesmo volume. `D:` é `hdd`. A pasta da corrida foi `D:\my-photos-f03-baseline\corpus`.

O volume `E:` (Seagate, a biblioteca do AD-007) respondeu `unknown` na consulta de penalidade de seek e não foi usado. As duas classes pedidas, `ssd` e `hdd`, existem nesta máquina e foram medidas.
