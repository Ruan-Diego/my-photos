# F16 — Design

Canvas: [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp) · artboards `Main`, `MainEscuro`.

## Arquivos

| Arquivo | Papel |
| ------- | ----- |
| `src-tauri/src/commands/onboarding.rs` | `library_folder` (raiz, volume, exports) e `start_indexing` (thread + eventos) |
| `src/onboarding/flow.ts` | Quando mostrar, texto da pasta, cartões "Indexando" e "A indexação parou" |
| `src/onboarding/Onboarding.tsx` | A tela da artboard `Main` |
| `src/App.tsx` | Escolhe boas-vindas ou moldura; ouve os eventos da indexação |
| `src/theme/tokens.css` | `--photo-1..4`, lugares de foto iguais nos dois temas |

## Fluxo

1. `library_summary` (F15) diz se é primeiro uso; então `library_folder` preenche o cartão da pasta.
2. "Indexar" chama `start_indexing`; o Rust abre a própria conexão (`db::open_located`) e roda `index_default_library`, emitindo `Progress` (F10) a cada etapa e lote.
3. O app vai para `#/fotos`; a lateral mostra o progresso. `indexing://finished` traz o resumo ou o código do erro e o app relê `library_summary`.

## Movimento (AD-006)

- Motion: os quatro retângulos entram um a um (mola, 120 ms entre eles) e param. Sem loop. `MotionConfig reducedMotion="user"` corta a animação.

## Fora do canvas

- "Trocar pasta" desativado (decisão "Depois").
- Mensagem de falha abaixo do botão (`--warn-bg`/`--warn-fg`).
- Cartão "Indexando E:\" e "A indexação parou" na lateral (a tela própria é a F17).
