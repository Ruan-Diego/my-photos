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

## Baselines release com cache frio

Medidos em 2026-10-02 com `src-tauri/target/release/f03-bench.exe --cold` (build `--release`). O modo `--cold` grava o corpus, espera 30 s para o lazy-writer esvaziar o cache e lê cada arquivo com `FILE_FLAG_NO_BUFFERING`, então o hash vai ao dispositivo e não ao cache de arquivos do Windows. O relatório traz `cacheMode: "cold"`. Mesmos volumes e protocolo da tabela acima (`C:` ssd, `D:` hdd, 20 workers).

| Arquivo | volumeClass | itemCount | failureCount | ok | walkElapsedMs | hashElapsedMs |
| --- | --- | --- | --- | --- | --- | --- |
| `ssd-cold-release-50000.json` | ssd | 50000 | 0 | true | 3888 | 2422 |
| `ssd-cold-release-150000.json` | ssd | 150000 | 0 | true | 7965 | 6889 |
| `hdd-cold-release-50000.json` | hdd | 50000 | 0 | true | 6604 | 76999 |
| `hdd-cold-release-150000.json` | hdd | 150000 | 0 | true | 11001 | 56118 |

Ressalvas:

- `walkElapsedMs` continua parcialmente quente: o cache de metadados de diretório (MFT) não é esvaziado sem privilégio de administrador.
- No HDD, 150k itens (56 s) foi mais rápido que 50k (77 s). Não há causa confirmada; pode ser fragmentação, antivírus ou lazy-writer ainda ativo. Repetir antes de tirar conclusões de escala.
- Uma tentativa com `sync_all` por arquivo foi descartada: ~7 arquivos/s no HDD, horas por corrida.
- `E:` (a biblioteca do AD-007) respondeu `unknown` e não foi medido. O bench não escreve nele.
