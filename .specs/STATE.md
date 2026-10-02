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

## Handoff

- **Feature**: Phase 0 concluída (F01, F02, F03)
- **Phase / Task**: critério de saída atingido no Takeout real em `E:\` — 4247/4247 mídias do Google Fotos casadas (100%), 5 JSONs órfãos sem mídia exportada
- **Completed**: matcher aceita `supplemental-metadata` sem truncar (`0406130`) e casa entre partes do mesmo export (`2eb6812`); pastas ilegíveis viram `Problem` (`d55ebbd`); baselines release com cache frio (`1435d5b`, `78ac57f`). Survey só leitura: `TAKEOUT_ROOT=E:\ cargo test real_takeout_survey -- --ignored --nocapture`
- **In-progress**: nenhum
- **Next step**: F04 Database Schema, já considerando AD-008/009/010 (caminho físico mutável, lixeira do app, operações de arquivo da F48)
- **Blockers**: `C:` com ~430 MB livres. `E:` deu "erro fatal de hardware" uma vez em 2026-10-02 e voltou após reconexão — vigiar
- **Open questions**: anomalia do baseline HDD (50k mais lento que 150k); 1885 mídias fora do Google Fotos dependem de EXIF/filesystem (F07/F09)
- **Uncommitted files**: PRD HTML, `.specs/LESSONS.md`, `.specs/lessons.json`
- **Branch**: main
