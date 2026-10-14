# F11 — Volume Resolver Specification

## Problem Statement

O app abria sempre `E:\.myphotos\library.db` e varria `E:\`. Se o Windows montar o HD com outra letra (outra porta USB, outro PC), o app dizia "disco E: não encontrado" — e uma varredura numa letra nova criaria uma **segunda** biblioteca vazia no mesmo banco, porque `libraries` é procurada por `root_path`. RF-053/054/055 pedem identificar o disco pelo ID do volume e revincular sozinho.

## Goals

- [x] Achar o disco da biblioteca em qualquer letra
- [x] Gravar o serial e o rótulo do volume em `libraries`
- [x] Disco que trocou de letra continua a mesma biblioteca (mesmas linhas, caminhos relativos)

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Navegar com o disco desconectado | AD-011: o banco fica no próprio disco; F27 saiu do MVP |
| Várias bibliotecas em discos diferentes | Fora do MVP |
| Monitorar conexão/desconexão em tempo real | F17/F27 (UI) |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| ID do volume | Serial do Windows (`GetVolumeInformationW`), formato `1A2B-3C4D` como no `dir`; rótulo junto | RF-053 ("Windows serial"); muda só ao formatar | y (ROADMAP) |
| Onde procurar | Todas as letras montadas (`GetLogicalDrives`) com `.myphotos\library.db`; `E:\` vence se tiver; senão a única que tiver | O banco está no disco (AD-011), então o arquivo **é** a marca do disco | n |
| Primeira execução (nenhum banco ainda) | `E:\` se estiver montado (AD-007); senão "disco da biblioteca não encontrado" | Não criar biblioteca num disco qualquer | n |
| Dois discos com biblioteca, nenhum em `E:` | Erro `library_disk_ambiguous`, nada é aberto | Não adivinhar | n |
| Revincular | No início de cada execução, a linha de `libraries` com o mesmo serial passa a ter a letra atual em `root_path`; se outra linha já usa essa letra, nada muda | `media_items` guarda caminhos relativos, então tudo continua valendo | n |
| Mensagem | "disco E: não encontrado" → "disco da biblioteca não encontrado" | A letra não é mais fixa | n |
| Fora do Windows | Sem serial; nada é gravado nem revinculado | Testes no Linux | y |

---

## Requirements

### VOL-01 — Localizar
1. `E:\` com banco → `E:\`, mesmo que outra letra também tenha.
2. Só outra letra tem banco → essa letra.
3. Nenhum banco e `E:\` montado → `E:\` (primeira execução); sem `E:\` → `library_disk_missing`.
4. Dois ou mais sem `E:\` → `library_disk_ambiguous`.

### VOL-02 — Ler o volume
1. Serial `XXXX-XXXX` em hexadecimal maiúsculo; rótulo vazio vira ausente.
2. Letras montadas a partir da máscara de bits do Windows.

### VOL-03 — Revincular
1. Biblioteca com o mesmo serial em outra letra → `root_path` passa a ser a letra atual.
2. Serial diferente ou desconhecido → nada muda.
3. Letra já usada por outra biblioteca → nada muda.

### VOL-04 — Registrar
1. Cada execução grava `volume_id`, `volume_label` e `status = 'active'`; volume desconhecido não apaga o que havia.

## Success Criteria

- [x] `volume::tests` (10 no Linux, 10 no Windows)
- [ ] CI `windows-latest` verde
- [ ] Na próxima indexação no `E:`, `libraries.volume_id` preenchido (verificação manual)
