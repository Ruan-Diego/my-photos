# F12 — Thumbnail Service Specification

## Problem Statement

A linha do tempo (F18) mostra milhares de fotos de uma vez e o visualizador (F19) precisa de algo na tela antes do original de 5–20 MB chegar de um HD USB. Decodificar o original a cada rolagem é lento demais (RNF-001). RF-007 e RF-056/057/058 pedem miniaturas em três tamanhos, geradas em fila assíncrona com prioridade, num cache com limite de tamanho.

## Goals

- [x] Miniaturas JPEG em três tamanhos, em pé (orientação EXIF aplicada)
- [x] Fila assíncrona: o que a UI pede passa na frente; o resto vai das fotos mais novas para as mais antigas
- [x] Cache com limite configurável, despejando o menos usado
- [x] Cada miniatura é feita uma vez e refeita só quando o original muda

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Miniatura de vídeo | F13 (ffmpeg) |
| HEIC, RAW | O decodificador em Rust puro não lê; ficam como `unsupported_format` (ver Open Questions) |
| Comando Tauri / protocolo para a UI | F18 consome `thumbnail_for` e `ThumbService::request` |
| Gerar durante a indexação (F10) | Fica separado; o app pode chamar os dois em sequência |

---

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Tamanhos (maior lado) | `small` 300 px, `medium` 1280 px, `large` 2560 px; nunca amplia | ROADMAP "300px, média, grande"; 1280 cobre o visualizador em notebook, 2560 em tela 1440p/4K escalada | n |
| Formato | JPEG qualidade 82, transparência achatada em branco | Lido por qualquer webview; ~20–40 KB no `small` | n |
| Onde ficam | `<disco>\.myphotos\thumbs\<tamanho>\<id mod 256 em hex>\<id>.jpg` | AD-011: tudo do app fica no disco da biblioteca; o scanner já ignora `.myphotos` | n |
| Quando refazer | "Impressão digital" = tamanho do arquivo + `fs_modified_at` + orientação | Original trocado ou girado → miniatura velha | n |
| Falhas | Gravadas em `thumbnail_failures` com a impressão digital; não tenta de novo até o arquivo mudar | Não decodificar o mesmo HEIC a cada execução | n |
| Prioridade | Pedido da UI > preenchimento; no preenchimento, `taken_at` mais recente primeiro (sem data por último) | RF-057 "priorização por recência" | y (ROADMAP) |
| Limite do cache | `app_settings.thumbnail_cache_limit_bytes`; padrão 4 GiB; valor inválido ou ≤ 0 → padrão | RF-058 "limite configurável" | n |
| Despejo (LRU) | Por `last_accessed_at`: `medium`/`large` primeiro, `small` só quando a UI pede algo; nunca o que acabou de ser feito | O `small` é a grade inteira; despejá-lo para o preenchimento seria trocar seis por meia dúzia | n |
| Cache cheio no preenchimento | Para o preenchimento (`stoppedCacheFull`), mantém o que já fez; pedidos da UI continuam | Não entrar em ciclo despejar/refazer | n |
| Paralelismo | Núcleos da CPU, de 1 a 8 workers; uma thread grava no banco em lotes de até 64 | Decodificar é CPU; banco no USB com `synchronous=FULL` pede poucos commits | n |

**Open question**: quantas fotos do `E:` são HEIC/RAW? Se forem muitas, a F12 precisa de um decodificador do Windows (WIC) ou `libheif`; medir com `f12-thumbs` (`unsupported`).

---

## Requirements

### THUMB-01 — Renderizar
1. Cada tamanho cabe no seu maior lado e mantém a proporção; original menor não é ampliado.
2. Orientação EXIF 1–8 aplicada (6 → largura e altura trocam); valor fora disso é ignorado.
3. Transparência vira branco. A saída é sempre JPEG.

### THUMB-02 — Falhas
1. Formato que o decodificador não reconhece (HEIC) → `unsupported_format`; arquivo que não decodifica → `corrupt_media`; arquivo que não abre → `unreadable`.

### THUMB-03 — Fila
1. Pedidos da UI saem antes do preenchimento; no preenchimento, mais novo primeiro, sem data por último.
2. Um pedido para algo já na fila sobe a prioridade sem duplicar.
3. Cache cheio descarta só o preenchimento.

### THUMB-04 — O que falta
1. Só fotos (não vídeos, não `missing_file`) da biblioteca deste disco.
2. Miniatura com a impressão digital atual não é refeita; falha com a impressão digital atual não é tentada de novo; uma falha vale para todos os tamanhos.
3. Original com tamanho, data de modificação ou orientação diferente volta para a fila.

### THUMB-05 — Gerar a biblioteca
1. Grava o arquivo e a linha em `thumbnails` (largura, altura, bytes, impressão digital); nada é escrito ao lado dos originais.
2. Segunda execução não faz nada. Original consertado é refeito e a falha some.

### THUMB-06 — Servir a UI
1. `thumbnail_for` devolve o caminho só se a miniatura está em dia e o arquivo existe, e marca `last_accessed_at`.
2. `request` gera o que falta passando na frente; pedido de algo em dia só marca o uso.

### THUMB-07 — Limite do cache
1. Despejo LRU conforme a tabela acima; o arquivo e a linha saem juntos.
2. Limite lido de `app_settings`.
3. Cache cheio para o preenchimento sem despejar `small`.

## Success Criteria

- [x] `thumbs::tests` 16/16 no Linux
- [ ] CI `windows-latest` verde
- [ ] `f12-thumbs` no `E:`: tempo, `generated`, `unsupported`, tamanho do cache (verificação manual)
