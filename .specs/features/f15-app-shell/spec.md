# F15 — App Shell & Navigation Specification

> Design: canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp), artboards **`Sidebar`** e **`Tema`** (AD-014, `.specs/DESIGN.md`). Aparência em **`Settings`**.

## Problem Statement

O app abre numa tela de saúde do F01. Antes de qualquer tela real (F16–F31) é preciso a moldura do canvas: barra lateral com as seções, área de conteúdo, tokens de cor, fonte e tema claro/escuro.

## Goals

- [x] Barra lateral do canvas: logo, seis seções principais, grupo "Acervo" com quatro, cartão do disco no rodapé
- [x] Navegação entre seções por endereço (`#/fotos`, `#/pessoas`…), item ativo invertido
- [x] Tokens do canvas como variáveis CSS, nos dois temas; Hanken Grotesk empacotada
- [x] Tema: seguir o Windows (padrão), Claro ou Escuro, sem flash na abertura

## Out of Scope

| Feature | Reason |
| ------- | ------ |
| Conteúdo de cada seção | F16–F31, F48; cada uma mostra "em construção" até lá |
| Resto de Configurações (biblioteca, cache, indexação, privacidade, animações) | F31 |
| Tela de disco desconectado, indexação | F11 (UI), F17 |
| Contador de Problemas na lateral | F28 |
| Atalhos de teclado | F30 |

## Assumptions & Open Questions

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Lista de seções | A do canvas (10), não a curta do ROADMAP | AD-014: o canvas vence | n |
| Rotas | Hash (`#/pessoas`), sem biblioteca de rotas | Dez seções planas; nada a instalar | n |
| Onde guardar o tema | `localStorage` (`myphotos.theme`) do WebView2 | Precisa ser lido antes da primeira pintura, antes do banco abrir; não é dado do acervo | n |
| Fonte | `@fontsource/hanken-grotesk` 400–700 no bundle | App offline (DESIGN.md) | n |
| Cartão do disco | Comando `library_summary`: raiz, mídias visíveis, última indexação | Texto do canvas: "E:\ conectado · 4.247 mídias · atualizado às 14:02" | n |
| Ações do cartão ("Procurar arquivos novos", "Procurar de novo") | Texto sem ação | As telas de destino são F17 e F11 | n |
| Janela | 1280×800, mínimo 960×600 | Artboards são 1440×900; 800×600 não cabe a lateral + grade | n |

## Requirements

### SHELL-01 — Cartão do disco
1. Conectado: ponto `--ok-dot`, "E:\ conectado", "N mídias · atualizado às HH:MM" (ou "em D mmm", ou "ainda não indexado"). Conta só o que a timeline mostra (sem ocultas e sem `missing_file`).
2. Desconectado ou erro: ponto `--accent-fill`, "HD desconectado" e o motivo em português.

### SHELL-02 — Barra lateral
1. Ordem e grupos do canvas: Fotos, Pastas, Álbuns, Pessoas, Locais, Favoritos; Acervo: Duplicatas, Problemas, Lixeira, Configurações.
2. Item ativo com fundo `--ink`, texto `--ground` e `aria-current="page"`; o destaque desliza entre itens (Motion), parado com reduzir movimento.

### SHELL-03 — Navegação
1. Cada seção tem um endereço próprio; endereço vazio ou desconhecido abre Fotos; voltar/avançar do WebView funcionam.

### SHELL-04 — Tema
1. Seguir o Windows por padrão, acompanhando a troca sem reiniciar; Claro e Escuro fixos em Configurações › Aparência.
2. Aplicado antes da primeira pintura; a troca anima só cor e fundo por 200 ms, seca com reduzir movimento.
3. Armazenamento bloqueado não quebra: volta a seguir o Windows.

### SHELL-05 — Tokens
1. Toda cor da interface sai de `var(--token)`; hex só em `tokens.css` (e no fundo pré-pintura do `index.html`).
