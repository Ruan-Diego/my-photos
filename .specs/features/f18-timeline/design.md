# F18 — Timeline View Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboards **`Timeline`** / **`TimelineEscuro`**.

## Dados (Rust)

- `commands/timeline.rs`, comando `timeline`: uma consulta em `media_items` com a miniatura pequena (`LEFT JOIN thumbnails`), ordem `taken_at IS NULL, taken_at DESC, id DESC`. Devolve um vetor por campo (`ids`, `times`, `offsets`, `ratios`, `durations`, `flags` com bit 1 = vídeo e 2 = favorito): 300 mil itens ficam em poucos MB de JSON.
- Protocolo `thumb` (`register_asynchronous_uri_scheme_protocol`): `http://thumb.localhost/<id>` no Windows. Fora da thread da UI, usa `thumbs::thumbnail_for` (miniatura atual + `last_accessed_at` para o LRU) e devolve o JPEG; 404 sem miniatura. Protocolo não passa pelo ACL de comandos; o comando `timeline` tem `allow-timeline` (teste `acl01`).

## Tela (React)

- `src/timeline/timeline.ts`: agrupar (`group`), anos (`years`), quebrar linhas (`rows`), posicionar tudo (`layout`: mês, dia e linhas de miniaturas com `top` e altura conhecidos), janela visível por busca binária (`visible`) e ano no topo (`yearAt`).
- `src/timeline/TimelineScreen.tsx`: cabeçalho do canvas; área rolável com um bloco da altura total e só as linhas visíveis em posição absoluta; trilho de anos.
- Medidas do canvas: mês 22 px/700 com 16 px acima; dia 15 px/600 + "N itens" em `--muted`; 8 px até a grade; 4 px entre miniaturas; 14 px entre dias; 28 px entre meses; trilho de 64 px.
- Miniatura: fundo `--sunken` como placeholder; a imagem entra com opacidade em 160 ms (sem animação com reduzir movimento). Na troca de tema a grade não anima (regra 10).
- Selo de vídeo e coração em branco sobre a foto, como no canvas (fora dos tokens: ficam sobre a foto, não sobre a moldura).
- `App.tsx`: `useTimeline` lê o comando ao abrir Fotos e de novo quando o resumo do disco muda (fim da indexação).

## Fora do canvas

- Estados vazio e erro: o mesmo bloco `page-empty` das outras seções.
- Grupo "Sem data" com o dia "Data não encontrada no arquivo nem no JSON".
- Busca e Selecionar inativos (cursor normal, texto `--muted`, dica no `title`).
- Selo de vídeo sem duração conhecida diz "Vídeo".
