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
- **Status**: superseded by AD-008 (2026-10-02)

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


### AD-008
- **Decision**: O app **gerencia os arquivos** da biblioteca em `E:\`: mover foto entre pastas, renomear pastas, criar pastas, excluir fotos e pastas. Substitui o AD-004.
- **Reason**: Definido pelo stakeholder. O acervo é organizado dentro do próprio app.
- **Trade-off**: Perde a garantia "zero alterações em originais" do PRD. Toda escrita só acontece por ação explícita do usuário na UI; varredura, indexação, matcher e thumbnails continuam só leitura. Mover ou renomear uma mídia leva junto o JSON do Takeout (`.supplemental-metadata.json`) casado com ela, para data e local não se perderem.
- **Scope**: Nova feature de operações de arquivo; F05/F10 (o índice precisa refletir as mudanças); F36 (a métrica de hash antes/depois passa a valer só para a indexação).
- **Date**: 2026-10-02
- **Status**: active

### AD-009
- **Decision**: Excluir pelo app move para uma **lixeira própria** dentro de `E:\` (pasta oculta), esvaziada depois de **30 dias**. Nunca exclusão permanente direta.
- **Reason**: Definido pelo stakeholder. A Lixeira do Windows nem sempre existe em discos USB.
- **Trade-off**: Ocupa espaço por até 30 dias. A tela de restaurar ainda não está definida. A pasta da lixeira fica fora da varredura.
- **Scope**: Operações de arquivo; scanner (ignorar a pasta da lixeira).
- **Date**: 2026-10-02
- **Status**: active

### AD-010
- **Decision**: A raiz é sempre `E:\` (reafirma o AD-007) e a árvore é exibida **como está no disco**, sem fundir pastas. As partes de um mesmo export (`takeout-<stamp>-N-NNN`) são unidas só por baixo, para casar mídia e JSON. Arquivos não compatíveis (`.zip`, `.html`, etc.) são ignorados sem virar problema. Pastas ocultas de sistema (`$RECYCLE.BIN`, `System Volume Information`) são puladas.
- **Reason**: Definido pelo stakeholder. No Takeout real, 169 de 4247 mídias têm o JSON em outra parte.
- **Trade-off**: Um export diferente nunca empresta JSON a outro, mesmo com a mesma pasta lógica.
- **Scope**: F02 (implementado em `2eb6812`), F05, F10, F15/F18 (navegação por pasta).
- **Date**: 2026-10-02
- **Status**: active

### AD-011
- **Decision**: O banco SQLite fica **dentro do acervo**, em `E:\.myphotos\library.db` (pasta oculta). A lixeira do AD-009 fica em `E:\.myphotos\trash\`. A varredura ignora `E:\.myphotos`.
- **Reason**: Definido pelo stakeholder. O acervo fica autocontido e vai junto com o HD; o `C:` tem pouco espaço.
- **Trade-off**: Com o `E:` desconectado o app não mostra a biblioteca — o modo offline (F27, US-05) sai do MVP. Banco em disco USB exige `synchronous=FULL` e tolerância a desconexão. Thumbnails (F12) provavelmente seguem a mesma pasta.
- **Scope**: F04, F05/F10 (ignorar `.myphotos`), F12, F27, F48.
- **Date**: 2026-10-02
- **Status**: active

## Handoff

- **Feature**: F07 EXIF Parser — concluída (Verifier PASS na iteração 4: EXIF-01..06; 71/77 mutantes mortos, os 6 sobreviventes sem efeito visível no spec). F06 concluída (PASS iteração 3). F05 concluída (PASS iteração 3; SCAN-07 no `E:` PASS em 2026-10-12: 6132 mídias, 38 pastas, 2ª varredura 6132 `unchanged`, 0 hash)
- **Phase / Task**: Phase 1. F07 em `src-tauri/src/photo_meta/` (`read_embedded`): data de captura local + UTC quando o fuso é conhecido (OffsetTime* ou relógio do GPS), GPS validado, dimensões pelo conteúdo (cabeçalho em JPEG/PNG/GIF/WebP; EXIF em HEIF e TIFF/RAW), orientação 1–8, `raw_json` do EXIF, `exif_corrupt`, `unreadable`. Dependências novas, Rust puro: `kamadak-exif` 0.6.1, `imagesize` 0.15.0; `rust-version = "1.87"`
- **Completed**: F01–F07. Defaults da F05/F06/F07 confirmados pelo stakeholder em 2026-10-06 ("continue with defaults"). CI `windows-latest` verde desde o run 4. Relatórios em `.specs/features/f0{5,6,7}-*/validation.md`
- **In-progress**: nenhum
- **Next step**: Próxima feature: F08 Media↔JSON Matcher — o matcher do F02 já cobre a cascata; a F08 provavelmente só mede >98% no Takeout real e fecha lacunas. Depois F09 (resolver JSON > EXIF > sistema de arquivos, usa F06 + F07) e F10 (pipeline que grava tudo)
- **Blockers**: nenhum (`C:` com ~38 GB livres em 2026-10-12). `E:` deu "erro fatal de hardware" uma vez em 2026-10-02 — vigiar
- **Open questions**: riscos da F07 a medir na varredura real (F10): WebP com EXIF prefixado por `Exif\0\0` vira `exif_corrupt`; o crate lê TIFF/RAW inteiro e percorre JPEG sem EXIF até o fim (ler junto com o hash na F10/F32); HEIC/WebP/RAW reais nunca testados; hora EXIF sem fuso fica só local (a F09 decide). Mídia movida fora do app vira nova + `missing_file` (F10/F48). Anomalia do baseline HDD. Lições candidatas da F04 aguardam Python
- **Uncommitted files**: PRD HTML, `.specs/LESSONS.md`, `.specs/lessons.json` (só na máquina local)
- **Branch**: `main` (F05–F07 mescladas pelo PR #2 em 2026-10-11)
