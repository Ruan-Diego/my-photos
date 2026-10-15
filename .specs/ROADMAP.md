# My Photos — Roadmap

> Biblioteca Local de Fotos e Vídeos para Exportações do Google Takeout
>
> Princípio: **o disco é a fonte da verdade**. Leitura, indexação e thumbnails nunca escrevem na biblioteca; o app só altera arquivos por ação explícita do usuário (AD-008), e exclusão vai para a lixeira do app (AD-009).

## Diretriz de UI/UX (todas as telas)

> **O design já está pronto e tem prioridade (AD-014).** Toda tela segue o canvas
> [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp). Mapa de telas,
> tokens e regras de tema em [`DESIGN.md`](DESIGN.md). Em conflito com este roadmap, vale o canvas.

Toda interface segue **AD-006**. O visual é fluido e moderno; a fotografia é o protagonista. O que se evita é o visual genérico de interface gerada por IA (gradiente roxo padrão, glassmorphism de template, motion decorativo sem função).

| Biblioteca | Papel |
|------------|--------|
| [Motion](https://motion.dev) | Presença, layout e gestos dos componentes React |
| [GSAP](https://gsap.com) | Timelines e sequências coreografadas de tela |
| [anime.js](https://animejs.com) | Microinterações leves (ícones, SVG, contadores) |
| [Three.js](https://threejs.org) | Momentos espaciais pontuais — onboarding e profundidade, nunca no lugar da grade |

Regras para qualquer tela (F15 em diante e features futuras de UI):

- Movimento serve continuidade e navegação. A foto não compete com o efeito.
- `prefers-reduced-motion` desliga ou simplifica a animação.
- A grade da timeline permanece virtualizada e fluida (~60 fps). Three.js não roda em loop sobre ela.
- Cada interação usa uma biblioteca do papel acima. As quatro não animam o mesmo elemento ao mesmo tempo.

---

## Visão Geral das Fases

```
Phase 0 ─ Discovery Técnico          ███░░░░░░░░░░░░░░░░░  ~10%
Phase 1 ─ Core Engine (Backend)      ██████████░░░░░░░░░░  ~35%
Phase 2 ─ UI Foundation              ██████░░░░░░░░░░░░░░  ~25%
Phase 3 ─ Features MVP               ████░░░░░░░░░░░░░░░░  ~20%
Phase 4 ─ Polish & Release (v1.0)    ██░░░░░░░░░░░░░░░░░░  ~10%
─────────────────────────────────────────────────────────── MVP ↑
Phase 5 ─ Pós-MVP (v1.1+)           Futuro
```

---

## Phase 0 — Discovery Técnico

> Validar stack, provar conceitos críticos, estabelecer base do projeto.

| Feature ID | Nome | Descrição | RFs Relacionados | Prioridade |
|------------|------|-----------|------------------|------------|
| **F01** | Project Setup | Scaffold Tauri 2 + React + TypeScript + Vite; SQLite embarcado via `rusqlite`; dependências de UI (`three`, `gsap`, `animejs`, `motion`); estrutura de pastas; CI básico | — | P1 ⭐ |
| **F02** | Takeout Discovery | PoC: parser de JSON do Takeout + matcher mídia↔JSON com fixtures reais; validar todas as variações conhecidas (seção 9.1–9.4 do PRD) | RF-003, RF-004 | P1 ⭐ |
| **F03** | Performance Baseline | Benchmark de varredura de sistema de arquivos + hashing SHA-256 em bibliotecas sintéticas (50k, 150k itens) em HDD e SSD | RNF-001, RNF-002 | P1 ⭐ |

**Critério de saída da Phase 0:** Matcher mídia↔JSON com >98% de acerto em fixtures reais; benchmark de I/O validado; stack operacional end-to-end (Rust ↔ React via Tauri commands).

---

## Phase 1 — Core Engine (Backend Rust)

> Toda a lógica de indexação, parsing e persistência. Sem UI além de logging.

| Feature ID | Nome | Descrição | RFs Relacionados | Prioridade |
|------------|------|-----------|------------------|------------|
| **F04** | Database Schema | Criar esquema SQLite completo (DDL seção 8.3 do PRD); migrações versionadas; índices para timeline e busca | RF-006, RNF-009 | P1 ⭐ |
| **F05** | File Scanner | Varredura recursiva de diretórios; identificação de mídia por extensão; cálculo de hash SHA-256; detecção de novos/modificados/removidos | RF-001, RF-002, RF-010, RF-011, RF-012, RF-013 | P1 ⭐ |
| **F06** | Takeout JSON Parser | Parser robusto dos JSONs de mídia e álbum; normalização de campos; tolerância a JSON corrompido | RF-003, RF-004, RF-063, RF-065 | P1 ⭐ |
| **F07** | EXIF Parser | Extração de metadados EXIF (data, GPS, dimensões, orientação); fallback quando JSON ausente | RF-005 | P1 ⭐ |
| **F08** | Media↔JSON Matcher | Algoritmo em cascata (6 estratégias da seção 9.4); confidence scoring (exact/heuristic/none) | RF-003 | P1 ⭐ |
| **F09** | Metadata Resolver | Hierarquia de resolução: Takeout JSON > EXIF > filesystem (seção 9.5); `taken_at_source` rastreável | RF-004, RF-005, RF-028 | P1 ⭐ |
| **F10** | Indexing Pipeline | Orquestrador do pipeline completo (scan → parse → match → resolve → persist); progresso; indexação incremental; tolerância a falhas | RF-006, RF-007, RF-008, RF-009, RF-010, RF-014, RF-015, RF-063, RNF-004, RNF-015 | P1 ⭐ |
| **F11** | Volume Resolver | Identificação de disco por volume ID (Windows serial); detecção de conectado/desconectado; revinculação automática de caminhos | RF-053, RF-054, RF-055 | P1 ⭐ |
| **F12** | Thumbnail Service | Geração de thumbnails em múltiplas resoluções (300px, média, grande); fila assíncrona; priorização por recência; cache LRU com limite configurável | RF-007, RF-056, RF-057, RF-058 | P1 ⭐ |
| **F13** | Video Thumbnail | Extração de frame de capa via ffmpeg (empacotado); preview de baixa resolução | RF-025 | P1 ⭐ |
| **F14** | Search Engine | Full-text search via FTS5 do SQLite; índices combinados; busca por nome, data, pessoa, local, tipo | RF-034, RNF-001 | P1 ⭐ |

**Critério de saída da Phase 1:** Pipeline de indexação completo passando em testes com fixtures sintéticas e reais; banco SQLite populado corretamente; thumbnails gerados; busca FTS5 funcional.

---

## Phase 2 — UI Foundation

> Interface React com os componentes core: timeline, viewer, navegação.
> Toda tela desta fase implementa a artboard correspondente do canvas de design ([`DESIGN.md`](DESIGN.md), AD-014) e segue a diretriz de movimento (Motion, GSAP, anime.js, Three.js).

| Feature ID | Nome | Descrição | RFs Relacionados | Prioridade |
|------------|------|-----------|------------------|------------|
| **F15** | App Shell & Navigation | Layout principal (sidebar + content area); navegação entre seções (Fotos, Álbuns, Pessoas, Locais, Favoritos, Config); tema pt-BR | — | P1 ⭐ |
| **F16** | Onboarding Flow | Tela de primeiro uso: seleção de pasta, explicação de que só ações do usuário alteram arquivos, início de indexação | RF-001, US-01 | P1 ⭐ |
| **F17** | Indexing Progress UI | Barra de progresso com contagem por tipo, percentual, ETA; uso parcial do app durante indexação | RF-008, RF-009, RF-014 | P1 ⭐ |
| **F18** | Timeline View | Grade virtualizada (react-window/virtua) agrupada por ano/mês; scroll contínuo; lazy loading de thumbnails; skeleton placeholders | RF-016, RF-017, RF-018, RF-019, RF-020, RNF-001 | P1 ⭐ |
| **F19** | Media Viewer | Visualizador fullscreen: zoom, ajuste à tela, tamanho original; navegação anterior/próxima; carregamento progressivo (thumbnail → original) | RF-021, RF-022, RF-024 | P1 ⭐ |
| **F20** | Video Player | Player integrado: play/pause, progresso, volume, tela cheia; indicador de vídeo na timeline (ícone + duração) | RF-023, RF-025 | P1 ⭐ |
| **F21** | Details Panel | Painel lateral: data/hora, local, pessoas, arquivo, dimensões, tamanho, fonte da data; seção técnica expansível | RF-026, RF-027, RF-028 | P1 ⭐ |

**Critério de saída da Phase 2:** Usuário pode completar o fluxo: abrir app → selecionar pasta → indexar → navegar timeline → abrir foto/vídeo → ver detalhes.

---

## Phase 3 — Features MVP

> Funcionalidades que completam a experiência MVP.
> Telas desta fase seguem a mesma diretriz de UI/UX da Phase 2.

| Feature ID | Nome | Descrição | RFs Relacionados | Prioridade |
|------------|------|-----------|------------------|------------|
| **F22** | Search & Filters UI | Campo de busca + painel de filtros combináveis (data, tipo, pessoa, local, pasta); resultados em grade | RF-034–RF-042 | P1 ⭐ |
| **F23** | People View | Grade de pessoas identificadas (do Takeout); navegação para grade de mídias por pessoa | RF-032, RF-033, RF-038 | P1 ⭐ |
| **F24** | Locations View | Exibição de latitude/longitude; mapa básico (tile estático ou embed); agrupamento/filtro por local | RF-029, RF-030, RF-031, RF-037 | P1 ⭐ |
| **F25** | Albums (Takeout) | Reconstrução de álbuns a partir de pastas/metadata.json do Takeout; grade de álbuns com capa e contagem | RF-043, RF-044, RF-045 | P1 ⭐ |
| **F26** | Favorites | Favoritar mídia localmente (sem alterar original); filtro de favoritos na timeline | RF-046 | P1 ⭐ |
| **F27** | Offline Mode | Navegação completa por metadados/thumbnails com disco desconectado; aviso claro ao tentar abrir original | RF-054, US-05 | P1 ⭐ |
| **F28** | Error Report & Diagnostics | Área "Itens não reconhecidos" (caminho, motivo, status); relatório de indexação; painel de diagnóstico | RF-014, RF-063, RF-064, RF-065, RF-062 | P1 ⭐ |
| **F29** | Duplicate Detection | Hash SHA-256 + hash perceptual; agrupamento informativo de duplicatas (sem exclusão automática) | RF-050, RF-051, RF-052 | P2 |
| **F30** | Keyboard Shortcuts | Atalhos de teclado conforme seção 15.4 do PRD (setas, Esc, F, Ctrl+F, Espaço, Ctrl+scroll) | — | P2 |
| **F31** | Settings UI | Gerenciar biblioteca/pastas, localização do banco/cache, preferências de indexação, tema, privacidade | RF-059, RF-060, RF-061 | P1 ⭐ |
| **F48** | File Operations | Mover mídia entre pastas, renomear e criar pastas, excluir para a lixeira do app (AD-008, AD-009); o JSON do Takeout acompanha a mídia; índice atualizado na hora; restaurar da lixeira | — | P1 ⭐ |

**Critério de saída da Phase 3:** Todas as user stories US-01 a US-10 do PRD atendidas; critérios de aceitação da seção 5 passando.

---

## Phase 4 — Polish & Release (v1.0)

> Qualidade, performance, acessibilidade, empacotamento.

| Feature ID | Nome | Descrição | RFs Relacionados | Prioridade |
|------------|------|-----------|------------------|------------|
| **F32** | Performance Tuning | Otimização para 300k+ itens; profiling de scroll, busca, indexação; validação de metas RNF | RNF-001, RNF-002, RNF-003, RNF-017 | P1 ⭐ |
| **F33** | Accessibility | Navegação por teclado completa; contraste AA; aria-labels; leitores de tela | RNF-013 | P2 |
| **F34** | i18n Foundation | Sistema de tradução; pt-BR como default; estrutura pronta para outros idiomas | RNF-020 | P2 |
| **F35** | App Data Backup | Exportação/importação de dados do app (favoritos, notas); backup automático do .db | RF-066 | P2 |
| **F36** | Security Hardening | Validação de entrada; parsing isolado; PRAGMA integrity_check; permissões mínimas; zero rede por padrão | RNF-005, RNF-007, RNF-008 | P1 ⭐ |
| **F37** | Windows Packaging | Instalador Windows (MSI/NSIS via Tauri); auto-update; assinatura de código | RNF-018 | P1 ⭐ |

**Critério de saída da Phase 4:** Build Windows assinado; todos os critérios de sucesso da seção 26 do PRD atendidos; zero alterações em arquivos durante a indexação, verificável por hash.

---

## Phase 5 — Pós-MVP (v1.1+) — Futuro

| Versão | Feature ID | Nome | Descrição |
|--------|------------|------|-----------|
| v1.1 | F38 | Local Albums UI | Álbuns criados pelo usuário na UI (modelo já suportado: RF-049) |
| v1.1 | F39 | Favorites Dedicated View | Tela dedicada de favoritos com grade e filtros |
| v1.1 | F40 | Advanced Map | Mapa interativo com agrupamento geográfico |
| v1.2 | F41 | Advanced Duplicates | Revisão assistida de duplicatas com ações de usuário |
| v1.2 | F42 | Multiple Libraries UI | Suporte a múltiplas bibliotecas na UI (backend já preparado) |
| v1.3 | F43 | macOS Support | Build, testes, volume resolver para macOS 12+ |
| v1.3 | F44 | Linux Support | Build, testes, volume resolver para distros LTS |
| v1.3 | F45 | Config Export/Import | Exportação/importação completa de configuração |
| — | F46 | Hide Media | Ocultar mídia da timeline (RF-047) |
| — | F47 | User Notes | Notas/marcações locais por mídia (RF-048) |

---

## Grafo de Dependências (MVP)

```
F01 (Setup)
 ├── F04 (DB Schema)
 │    └── F10 (Indexing Pipeline)
 │         ├── F05 (Scanner) ──┐
 │         ├── F06 (JSON Parser) ──> F08 (Matcher) ──> F09 (Resolver)
 │         ├── F07 (EXIF Parser) ─┘
 │         ├── F12 (Thumbnails) ── F13 (Video Thumb)
 │         └── F14 (Search Engine)
 │
 ├── F02 (Takeout Discovery) ──> F06, F08
 ├── F03 (Perf Baseline) ──> F05, F10
 │
 ├── F11 (Volume Resolver) ──> F27 (Offline Mode)
 │
 └── F15 (App Shell)
      ├── F16 (Onboarding) ──> F17 (Progress UI)
      ├── F18 (Timeline) ──> F19 (Viewer) ──> F20 (Video Player)
      │                                   └── F21 (Details Panel)
      ├── F22 (Search UI)
      ├── F23 (People View)
      ├── F24 (Locations View)
      ├── F25 (Albums)
      ├── F26 (Favorites)
      ├── F28 (Error Report)
      └── F31 (Settings)
```

---

## Rastreabilidade PRD → Features

### User Stories

| US | Descrição | Features |
|----|-----------|----------|
| US-01 | Apontar pasta e ver biblioteca organizada por data | F16, F10, F18 |
| US-02 | Rolar timeline e saltar para data específica | F18, F19 |
| US-03 | Abrir foto e ver data, local, pessoas | F19, F21 |
| US-04 | 100% offline, nunca enviar dados | F36 (todas as features respeitam) |
| US-05 | HD desconectado, biblioteca navegável | F11, F27 |
| US-06 | Pesquisar por pessoa ou local | F14, F22, F23, F24 |
| US-07 | Só reindexar novos | F05, F10 (incremental) |
| US-08 | JSON corrompido não trava indexação | F06, F10, F28 |
| US-09 | Reconhecer disco após troca de letra | F11 |
| US-10 | Favoritar sem alterar originais | F26 |

### Objetivos do Produto

| Objetivo | Features |
|----------|----------|
| O1 — Timeline cronológica | F09, F10, F18 |
| O2 — Visualização com qualidade | F19, F20, F12 |
| O3 — Localização e pessoas dos metadados | F23, F24, F21 |
| O4 — Busca e filtros rápidos | F14, F22 |
| O5 — Reconstruir álbuns | F25 |
| O6 — Robustez com HD/SSD externo | F11, F27 |
| O7 — Escalar para 300k+ itens | F03, F32, F18 (virtualização) |
| O8 — Privacidade total | F36 (princípio transversal) |

---

## Métricas de Sucesso do MVP

| Métrica | Meta | Verificação |
|---------|------|-------------|
| Mídias com `taken_at_source = takeout_json` | Maximizar | Query no banco após indexação real |
| Mídias com JSON associado (exact/heuristic) | > 98% | Fixture de teste real |
| Falhas que interrompem indexação completa | 0 | Teste de resiliência com fixtures corrompidas |
| Alterações em arquivos durante a indexação | 0 | Hash antes/depois da indexação |
| Timeline visível após abertura | < 2s | Benchmark com biblioteca indexada |
| Scroll da timeline | ~60 fps | Profiling com 300k+ itens |
| Busca/filtro | < 300ms | Benchmark com 500k itens + FTS5 |

---

*Documento gerado a partir do PRD v1.0 (26/09/2026). Próximo passo: especificar F01 (Project Setup) para iniciar a Phase 0.*
