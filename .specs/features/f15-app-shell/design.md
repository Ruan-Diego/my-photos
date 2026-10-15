# F15 — Design

Canvas: [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp) · artboards `Sidebar`, `Tema`, `Settings` (Aparência).

## Arquivos

| Arquivo | Papel |
| ------- | ----- |
| `src/theme/tokens.css` | Tokens do canvas; `:root` claro, `[data-theme="dark"]` e `prefers-color-scheme` escuro |
| `src/theme/theme.ts` | Escolha (`system`/`light`/`dark`), leitura e gravação, `data-theme` no `<html>` |
| `index.html` | Script inline aplica o tema salvo e pinta `--ground` antes do bundle |
| `src/shell/sections.ts` | As dez seções (rótulo, rota, grupo, ícone, feature) e a rota por hash |
| `src/shell/library.ts` | Texto do cartão do disco a partir de `library_summary` |
| `src/shell/Sidebar.tsx` | Barra lateral; destaque ativo com `motion` `layoutId` |
| `src/shell/SectionPage.tsx` | Cabeçalho 26 px/700 e o estado "em construção" |
| `src/shell/Appearance.tsx` | Cartão Aparência › Tema de Configurações |
| `src/App.tsx`, `src/App.css` | Moldura, `MotionConfig reducedMotion="user"`, medidas do canvas |
| `src-tauri/src/commands/library.rs` | `library_summary` |

## Movimento (AD-006)

- Motion: o fundo do item ativo desliza para o novo item (mola curta). Com reduzir movimento, `MotionConfig` corta a animação.
- Troca de tema: classe `theme-fade` no `<html>` por 250 ms liga `transition` de cor, fundo, borda e preenchimento (200 ms). Não é aplicada com reduzir movimento. A grade da F18 poderá excluir-se da classe.

## Fora do canvas

- Estado "em construção" das seções ainda sem tela: só texto, `--muted`, sem ilustração.
- Cartão "HD desconectado" na lateral: o canvas tem a tela inteira (F11), não o cartão; usa o par de cores do aviso (`--accent-fill` no ponto, `--warn-fg` na ação).
