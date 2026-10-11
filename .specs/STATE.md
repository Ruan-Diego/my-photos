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

- **Feature**: F05 File Scanner — implementada, aguarda CI no Windows e confirmação das premissas do spec
- **Phase / Task**: Phase 1. `scanner::scan(conexão, raiz)` grava `folders`, `media_items`, `indexing_runs`, `indexing_errors`; incremental por tamanho + `mtime`; movido por fora mantém o `id` (mesmo SHA-256); sumido vira `missing_file`
- **Completed**: F01–F04; F05 T1–T4 (`.specs/features/f05-file-scanner/`). 17 testes da F05 passam no Linux; os testes do F04/F03 só rodam no Windows
- **In-progress**: nenhum
- **Next step**: confirmar as premissas do spec da F05; depois F06 JSON Parser ou F10 (chamar `scanner::scan` a partir de um comando)
- **Blockers**: `C:` com ~430 MB livres. `E:` deu "erro fatal de hardware" uma vez em 2026-10-02 e voltou após reconexão — vigiar
- **Open questions**: premissas da F05 (spec, coluna "Confirmed?"); anomalia do baseline HDD (50k mais lento que 150k); 1885 mídias fora do Google Fotos dependem de EXIF/filesystem (F07/F09); 6 lições candidatas da F04 em `validation.md`; recuperação do WAL sem teste automático
- **Branch**: claude/project-thread-7sqrg0
