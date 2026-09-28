# STATE

## Decisions

### AD-001
- **Decision**: O nome do produto é **My Photos** (substituindo o nome de trabalho "Acervo" do PRD)
- **Reason**: Definido pelo stakeholder; nome em inglês para alcance global
- **Trade-off**: Perde a referência cultural em português do nome original
- **Scope**: Todo o projeto — branding, UI, repositório, documentação
- **Date**: 2026-09-27
- **Status**: active

### AD-002
- **Decision**: Stack tecnológica: **Tauri 2 (Rust backend) + React + TypeScript + SQLite** com Vite como bundler
- **Reason**: Conforme recomendação do PRD — menor consumo de memória/disco, melhor desempenho para I/O massivo, binário final pequeno
- **Trade-off**: Curva de aprendizado Rust vs Electron/Node puro; ecossistema menor de plugins Tauri
- **Scope**: Toda a arquitetura — backend, frontend, build, distribuição
- **Date**: 2026-09-27
- **Status**: active

### AD-003
- **Decision**: Windows 10/11 é a **plataforma prioritária** do MVP; macOS/Linux como evolução pós-MVP
- **Reason**: Conforme PRD (RNF-018); foco no público-alvo inicial
- **Trade-off**: Atrasa adoção multiplataforma; volume resolver inicialmente só com Windows APIs
- **Scope**: Build, testes, volume resolver, distribuição
- **Date**: 2026-09-27
- **Status**: active

### AD-004
- **Decision**: Princípio inegociável de **não destrutividade** — o app nunca modifica, move ou apaga arquivos originais
- **Reason**: Diferencial central de proposta de valor do produto (PRD seção 1, 2.4)
- **Trade-off**: Não pode corrigir EXIF nos originais mesmo quando o JSON tem dados melhores
- **Scope**: Todo o sistema — scanner, indexador, qualquer operação de I/O sobre a biblioteca
- **Date**: 2026-09-27
- **Status**: active

### AD-005
- **Decision**: Idioma padrão da UI: **pt-BR**, com arquitetura de i18n pronta para outros idiomas
- **Reason**: Conforme PRD (RNF-020); público-alvo inicial é brasileiro
- **Trade-off**: Textos hardcoded em pt-BR atrasariam i18n futura; precisa de sistema de tradução desde o MVP
- **Scope**: Frontend — toda string de UI visível ao usuário
- **Date**: 2026-09-27
- **Status**: active

### AD-006
- **Decision**: Toda UI/UX do My Photos usa **Three.js**, **GSAP**, **anime.js** e **Motion** (`motion.dev`). O visual é fluido e moderno, com a fotografia como protagonista — sem o visual genérico de interface gerada por IA.
- **Reason**: Definido pelo stakeholder. Motion dá presença, layout e gestos em React; GSAP coreografa timelines e sequências de tela; anime.js cobre microinterações leves; Three.js fica em momentos espaciais pontuais (onboarding, profundidade), nunca no lugar da grade de fotos.
- **Trade-off**: Quatro bibliotecas de movimento aumentam o bundle e o risco de animação competindo com a timeline virtualizada (~60 fps). Cada tela usa a ferramenta do papel dela; `prefers-reduced-motion` é obrigatório; WebGL não roda em loop sobre a grade.
- **Scope**: Todas as features de UI/UX (Phase 2 em diante, e qualquer tela futura). F01 inclui as dependências no scaffold.
- **Date**: 2026-09-27
- **Status**: active

### AD-007
- **Decision**: A pasta da biblioteca de mídia que o My Photos vai indexar é `E:\`.
- **Reason**: Definida pelo stakeholder como a pasta do acervo nesta máquina.
- **Trade-off**: Caminho absoluto e local. Outra máquina precisa de outra pasta. Não é configuração portátil.
- **Scope**: Indexação e onboarding (F05, F16 e qualquer feature que leia a biblioteca). A F01 não varre, não lista e não escreve em `E:\`.
- **Date**: 2026-09-27
- **Status**: active

## Handoff

- **Feature**: F02 Takeout Discovery — concluída
- **Phase / Task**: Validação independente — PASS em `.specs/features/f02-takeout-discovery/validation.md`
- **Completed**: T1–T7 commitados (`66158ff`..`ee16cfb`). O teste do campo extra foi reforçado em `e6fd16c` depois que um mutante sobreviveu. Segunda verificação PASS (54 testes Rust, 8/8 mutantes mortos)
- **In-progress**: nenhum
- **Next step**: F03 Performance Baseline, ou F06 se a ordem do roadmap priorizar o uso deste matcher
- **Blockers**: nenhum
- **Uncommitted files**: PRD HTML continua de fora dos commits
- **Branch**: main
