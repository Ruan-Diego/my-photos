# My Photos — Design

> **O design do app já está pronto.** Ele vive no canvas
> [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp) e é a
> **fonte da verdade e a prioridade** de toda tela (F15 em diante e qualquer UI futura).
> Decisão registrada em **AD-014** (`STATE.md`).

## 1. Prioridade

1. **O canvas manda.** Antes de escrever qualquer componente, abra a artboard da tela no
   canvas. Layout, hierarquia, textos, cores, espaçamentos, estados e navegação saem de lá.
   O código implementa o design, não reinterpreta.
2. **Em conflito, o canvas vence** o texto do ROADMAP e dos specs de feature. Exemplo: a
   barra lateral do canvas tem Fotos, Pastas, Álbuns, Pessoas, Locais, Favoritos e, em
   "Acervo", Duplicatas, Problemas, Lixeira e Configurações; isso vale sobre a lista curta
   da F15 no ROADMAP. Se o spec precisar divergir, a divergência é registrada no
   `spec.md` da feature e levada ao Ruan antes de codar.
3. **O que o canvas não cobre** (um estado, um diálogo, um tamanho de janela) se resolve
   com os tokens e componentes que ele já tem, no mesmo estilo, e é anotado no
   `design.md` da feature como "fora do canvas".
4. **AD-006 continua valendo** para movimento (Motion, GSAP, anime.js, Three.js). O canvas
   define o que se vê; AD-006 define como se move. Movimento nunca muda layout, cor ou
   texto definidos no canvas.
5. **Cada feature de UI cita a artboard.** O `spec.md` e o `design.md` de F15+ abrem com o
   link do canvas e o nome da artboard (tabela da seção 3). O `validation.md` compara a
   tela pronta com a artboard, em claro e em escuro.

O canvas é lido, não editado pelo time de implementação. Mudança de design se pede ao
Ruan e entra no canvas primeiro.

## 2. Como ler o canvas

- 1440 × 900 por artboard (janela de referência). A barra lateral é um componente à parte
  (240 px de largura) importado pelas telas.
- Quatro linhas: **Primeiro uso**, **Navegar e ver**, **Organizar**, **Cuidar do acervo**.
  Embaixo, as mesmas linhas no **modo escuro** (arquivos `*Escuro.dc.html`).
- **Diretrizes: tema claro e escuro** (`Tema.dc.html`) é a artboard de regras e tokens.
  Leia antes da primeira tela.
- Textos entre colchetes (`[NOME DO VOLUME]`, `[PESSOA 1]`, `[FOTO DE DESTAQUE]`) são
  lugares de dado real; os blocos de cor sólida no lugar das fotos são marcadores.

## 3. Mapa de telas → features

| Artboard (claro / escuro) | Tela | Feature |
|---|---|---|
| `Sidebar` | Barra lateral (componente) | F15 |
| `Main` / `MainEscuro` | Onboarding | F16 |
| `Indexing` / `IndexingEscuro` | Indexação | F17 |
| `Disconnected` / `DisconnectedEscuro` | Disco desconectado | F11 (UI) |
| `Timeline` / `TimelineEscuro` | Fotos — timeline | F18 |
| `Viewer` (só escuro) | Visualizador + detalhes | F19, F21 |
| `Video` (só escuro) | Player de vídeo | F20 |
| `Search` / `SearchEscuro` | Busca e filtros | F22 (motor: F14) |
| `People` / `PeopleEscuro` | Pessoas | F23 |
| `Locations` / `LocationsEscuro` | Locais | F24 |
| `Albums` / `AlbumsEscuro` | Álbuns do Takeout | F25 |
| `Favorites` / `FavoritesEscuro` | Favoritos | F26 |
| `Problems` / `ProblemsEscuro` | Problemas e diagnóstico | F28 |
| `Duplicates` / `DuplicatesEscuro` | Duplicatas | F29 |
| `Settings` / `SettingsEscuro` | Configurações | F31 |
| `Folders` / `FoldersEscuro` | Pastas e operações de arquivo | F48 |
| `Trash` / `TrashEscuro` | Lixeira do app | AD-009 |
| `Tema` | Diretrizes de tema e tokens | todas |

## 4. Fundamentos (resumo do canvas)

Resumo para consulta rápida. Se este arquivo e o canvas discordarem, vale o canvas.

### Tipografia

- **Hanken Grotesk** 400/500/600/700, com fallback `'Segoe UI', system-ui, sans-serif`.
  O app é offline: a fonte vai empacotada no bundle (não via Google Fonts).
- Código e caminhos: `'Cascadia Mono', Consolas, monospace`.
- Escala usada nas artboards: título de boas-vindas 52 px/700 (−0.03em); título de tela
  26 px/700; título de mês 22 px/700; seção 18 px; corpo 15–16 px; apoio 14 px; rótulo
  de seção 12–13 px/600, caixa alta, +0.06em.
- Números de contagem, duração e coordenadas usam `font-variant-numeric: tabular-nums`.

### Tokens de cor

Uma tela nunca escreve cor direto: tudo sai de `var(--token)`. Hex só na definição.

| Token | Uso | Claro | Escuro |
|---|---|---|---|
| `--ground` | Fundo da janela | `#f4f0e8` | `#161513` |
| `--side` | Barra lateral | `#ece5d8` | `#1b1a17` |
| `--side-card` | Cartão de status na lateral | `#e2dacb` | `#26231f` |
| `--side-line` | Borda da lateral | `#ddd3c2` | `#2a2723` |
| `--surface` | Cartões, campos, tabelas | `#fbf8f2` | `#201e1b` |
| `--sunken` | Grupos de botões, notas, código | `#ebe4d7` | `#2a2723` |
| `--line` | Divisórias | `#e2dbcd` | `#2e2b27` |
| `--line-2` | Borda de campos e cartões | `#ddd3c2` | `#36322d` |
| `--line-strong` | Borda de botões secundários | `#cfc5b3` | `#4a453e` |
| `--ink` | Texto principal; fundo da ação principal | `#1c1915` | `#f1ece3` |
| `--ink-2` | Texto de apoio longo | `#4f483f` | `#cfc6b8` |
| `--muted` | Legendas, metadados | `#5f574c` | `#a89f91` |
| `--accent` | Links e texto de destaque | `#a8471f` | `#ec9068` |
| `--accent-hover` | Link ao passar o mouse | `#7d3416` | `#f5b597` |
| `--accent-fill` | Progresso, seleção, selos, pins | `#a8471f` | `#e07a4c` |
| `--warn-bg` / `--warn-fg` | Aviso, pendente | `#f7e6dc` / `#7d3416` | `#3a2419` / `#f3b593` |
| `--ok-bg` / `--ok-fg` | Sucesso, resolvido | `#dfeadf` / `#2c5a37` | `#1e3326` / `#a9d8b5` |
| `--ok-dot` | Indicador "conectado" | `#3d7a4f` | `#5fb07a` |
| `--neutral-bg` | Selo neutro, ignorado | `#e6e0d5` | `#2e2b27` |
| `--console` | Log e painel do onboarding | `#1c1915` | `#0d0c0a` |
| `--map` / `--map-grid` / `--map-land` | Mapa | `#e4ddd0` / `#d8d0c1` / `#d4ccbb` | `#1f1d1a` / `#2a2723` / `#2b2824` |
| `--shadow` | Sombra de diálogos e barras flutuantes | `rgba(28,25,21,0.24)` | `rgba(0,0,0,0.55)` |

Contraste medido no canvas: todo texto ≥ 4.5:1 (ex.: `--ink`/`--ground` 15.4:1 claro,
15.5:1 escuro; `--muted`/`--sunken` 5.6:1 e 5.7:1).

### Regras do tema (artboard `Tema`)

1. **A foto nunca muda.** Miniatura, capa e visualizador sem filtro, opacidade, tinta ou
   borda colorida. Só a moldura do app troca de tema.
2. **Visualizador e player são sempre escuros**, nos dois temas; o painel de detalhes
   acompanha.
3. **Nenhuma cor direto na tela.**
4. **Ação principal e item ativo invertem:** fundo `--ink`, texto `--ground`.
5. **O destaque clareia no escuro:** `--accent` para texto, `--accent-fill` para
   preenchimento, sempre com texto `--ground` por cima.
6. **Profundidade por superfície, não por sombra.** Sombra só em diálogos e barras
   flutuantes.
7. **Escuro quente, sem preto puro.** Nada de `#000` ou `#fff` em áreas grandes.
8. **Controles nativos seguem o tema** via `color-scheme`.
9. **Estados têm par próprio** (`--warn-*`, `--ok-*`); não misture fundo de um com texto
   do outro.
10. **Troca de tema sem espetáculo:** 200 ms só em cor e fundo; com reduzir movimento, troca
    seca; a grade da timeline nunca anima na troca.
11. **Mapa e console têm tokens próprios**; pins sempre `--accent-fill`.
12. **Teste os dois temas**: toda tela nova é revisada em claro e escuro, incluindo vazio,
    erro e seleção.

### Escolha do tema

- Padrão: seguir o Windows (`prefers-color-scheme`), acompanhando a troca sem reiniciar.
- Claro ou Escuro fixo em Configurações › Aparência, aplicado na hora em todas as janelas.
- Aplicado antes da primeira pintura (sem flash claro → escuro).
- O React grava a escolha em `document.documentElement.dataset.theme` (`"light"`, `"dark"`
  ou ausente para seguir o Windows). Componentes só leem `var(--token)`.

```css
:root { color-scheme: light; --ground: #f4f0e8; --ink: #1c1915; /* … */ }
:root[data-theme="dark"] { color-scheme: dark; --ground: #161513; --ink: #f1ece3; /* … */ }
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) { /* mesmos valores do escuro */ }
}
```

### Componentes e medidas recorrentes

- **Raios:** botões e itens da lateral 10–12 px; cartões e campos 14 px; selos 999 px;
  miniaturas 4 px; foto no visualizador 2 px.
- **Alvos de clique:** mínimo 40 px de altura na lateral, 44 px em botões e campos, 48 px
  na ação principal do onboarding e nas setas do visualizador.
- **Grupo segmentado** (filtros Tudo/Fotos/Vídeos/Favoritos, zoom Ajustar/100%): fundo
  `--sunken`, item ativo invertido (`--ink`/`--ground`), `aria-pressed`.
- **Barra lateral:** logo, seções principais, grupo "Acervo" com contador (selo
  `--accent-fill`) e, no rodapé, o cartão do disco (`--side-card`, ponto `--ok-dot`,
  contagem de mídias e "Procurar arquivos novos"). Item ativo com `aria-current="page"`.
- **Timeline:** cabeçalho com título, busca (`Ctrl F`), filtros, tamanho e "Selecionar";
  grupos por mês e dia (com local e contagem); linhas justificadas de 176 px de altura
  respeitando a proporção da foto; vídeo com selo de duração; favorito com coração; trilho
  de anos à direita (ano atual em `--accent-fill`).
- **Visualizador:** cabeçalho com voltar, data e local, "3 de 14", zoom, favoritar,
  detalhes e mais ações; setas laterais; tira de miniaturas embaixo; painel Detalhes
  (data com a fonte da data, mapa, pessoas, arquivo e a seção "Técnico" recolhível).
- **Ícones:** traço 1.7–1.8 px, pontas arredondadas, 20–22 px, `currentColor`.
- **Acessibilidade:** `aria-label` em botões só de ícone, `role="group"` nos grupos,
  `aria-hidden` em ícones e ilustrações decorativas.

## 5. Fluxo de trabalho para uma tela

1. Abrir a artboard (claro e escuro) no canvas e listar o que ela mostra.
2. Escrever `spec.md`/`design.md` da feature citando a artboard; anotar o que fica fora do
   canvas e qualquer divergência (levada ao Ruan).
3. Implementar com os tokens de `src/` (F15 cria o arquivo de tokens a partir da tabela
   acima e do canvas).
4. Validar lado a lado com a artboard, nos dois temas, com estados vazio, erro e seleção,
   e com reduzir movimento ligado.
