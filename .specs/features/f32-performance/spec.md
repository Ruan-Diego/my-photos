# F32 — Performance Tuning

Metas do ROADMAP: linha do tempo visível em menos de 2 s, rolagem ~60 fps e busca/filtro em menos de 300 ms, com 300 mil itens ou mais (RNF-001, RNF-002, RNF-003, RNF-017).

## Requisitos

- **PERF-01**: Cada tela que lê o banco responde em menos de 1 s com 300 mil itens, medida num banco sintético com a mesma forma do real (pastas do Takeout, pessoas, locais, vídeos, favoritos, cópias idênticas e hashes perceptuais). Exceção aceita: Duplicadas, aberta raramente, em menos de 3 s.
- **PERF-02**: Busca e filtros da busca abaixo de 300 ms.
- **PERF-03**: A linha do tempo chega ao fim em qualquer tamanho e largura de janela, mesmo acima do limite de altura do WebView2 (~33,5 milhões de px).
- **PERF-04**: Agrupar e montar a linha do tempo de 300 mil itens no navegador leva menos de 200 ms.

Fora do escopo: indexação (medida na F10/F12 contra o `E:` real) e fps reais, que só o Windows com GPU mede.
