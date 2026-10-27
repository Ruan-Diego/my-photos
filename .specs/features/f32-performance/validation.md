# F32 — Validation

| Requisito | Teste |
| --------- | ----- |
| PERF-01, PERF-02 | `perf::tests::perf01_every_screen_on_300k_items` (tabela abaixo); `duplicates::tests::dup05_*` confere `similar_pairs` contra todos os pares; os testes de `thumbs` conferem `pending_count` com `pending` |
| PERF-03 | `timeline.test.ts` PERF-01; Playwright com 300 mil itens, 756 px de largura e tamanho 5: área de 30 milhões de px, rolar até o fim mostra "Sem data" (`timeline-300k-*.png` em `/mnt/project-files/f32-performance/`) |
| PERF-04 | medição no Node abaixo |

## Banco com 300 mil itens (Linux, release, melhor de 3)

| Tela | Antes | Depois |
| ---- | ----: | -----: |
| Linha do tempo (comando) | 462 ms | 461 ms |
| Resumo da biblioteca | 529 ms | 297 ms |
| Busca: texto / local / página | 31 / 57 / 143 ms | 34 / 56 / 154 ms |
| Opções da busca | 9.504 ms | 243 ms |
| Pessoas | 327 ms | 300 ms |
| Locais | 610 ms | 624 ms |
| Álbuns | 0 ms | 0 ms |
| Duplicadas | não terminava | 2.348 ms |
| Problemas | — | 627 ms |
| Pastas: árvore / itens | — | 147 / 193 ms |
| Configurações | — | 0 ms |

## Navegador (Node, 300 mil itens)

- `group`: 211 → 77–91 ms; `layout` a 1200 px: 32–45 ms; `visible` e `yearAt`: menos de 1 µs.
- Altura da linha do tempo: 12 milhões de px a 1200 px no tamanho 3; 46 milhões a 800 px no tamanho 5, acima do limite do WebView2. Corrigido com `scrollSpace`.
- Rolagem no Chromium sem GPU: o tempo por passo é parecido com 6 mil e com 300 mil itens, ou seja, o tamanho da biblioteca não pesa. Os fps reais ficam para o Windows.

Pendente no Windows: rolar a linha do tempo do `E:` e ver se fica fluida; abrir Duplicadas e Locais.
