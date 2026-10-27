# F29 — Duplicate Detection Design

> Canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboard **`Duplicates`**.

## Rust

- `duplicates/mod.rs`: `dhash` (9×8 cinza, `image`), `fill_perceptual_hashes(conn, root, stop)` para fotos com miniatura pequena e sem hash, `duplicates(conn)`: agrupa por `file_hash_sha256` e, por união de pares com distância de Hamming ≤ `SIMILAR_BITS`, os hashes perceptuais.
- `index::index_and_thumbnail_default_library` chama `fill_perceptual_hashes` depois das miniaturas.
- `scan`: ao gravar um SHA-256 diferente, zera `perceptual_hash`.
- `commands/duplicates.rs`: `duplicates_list`; permissão `allow-duplicates-list`.

## React

- `src/duplicates/duplicates.ts`: filtros, título, "repetidos", pasta, meta, itens do visualizador, texto vazio.
- `src/duplicates/DuplicatesScreen.tsx`: cabeçalho com o grupo segmentado, nota, grupos (até 200), `#/duplicatas/<id>` abre o visualizador; "Mostrar na pasta" desativado até a F48.
- `navigation.ts`: base `duplicatas` ("Voltar para Duplicatas").
