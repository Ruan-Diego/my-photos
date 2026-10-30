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

### AD-012
- **Decision**: Hora EXIF sem fuso é convertida para UTC com um fuso padrão de **-03:00 (Brasília)**, configurável (`DEFAULT_FALLBACK_OFFSET_MINUTES`). `taken_at_timezone` fica vazio nesse caso; só guarda o fuso que o próprio arquivo informou.
- **Reason**: A maior parte do acervo foi fotografada no Brasil; assim essas fotos ficam ao lado das datadas pelo JSON (UTC) na linha do tempo.
- **Trade-off**: Fotos tiradas no exterior ou em horário de verão antigo (até 2019) podem ficar até 1–2 h deslocadas.
- **Scope**: F09, F10, F18.
- **Date**: 2026-10-13
- **Status**: proposed (card de decisão aberto)

### AD-013
- **Decision**: Miniaturas ficam no disco da biblioteca, em `.myphotos\thumbs\`, como JPEG q82 em três tamanhos (300/1280/2560 px no maior lado), com cache LRU de 4 GiB por padrão (`app_settings.thumbnail_cache_limit_bytes`). Decodificador em Rust puro (`image`), sem HEIC/RAW.
- **Reason**: AD-011 (o que é do app vive no disco); JPEG é lido por qualquer webview; sem dependência nativa no build do Windows.
- **Trade-off**: HEIC e RAW ficam sem miniatura (`unsupported_format`) até haver um decodificador do Windows (WIC) ou `libheif`. Disco desconectado = sem miniaturas, como já é sem banco.
- **Scope**: F12, F13, F18, F19.
- **Date**: 2026-10-14
- **Status**: proposed

### AD-014
- **Decision**: O design de todas as telas já está pronto no canvas [MyPhotos — Telas](https://claude.ai/artifact/CDKz8D5PcgHkmGmkfbsmFp) (claro e escuro). Ele é a fonte da verdade e a prioridade de toda UI (F15 em diante): layout, textos, tokens, estados e navegação saem de lá. Resumo e mapa de telas em `.specs/DESIGN.md`.
- **Reason**: Definido pelo stakeholder antes de começar as telas.
- **Trade-off**: Em conflito com o ROADMAP ou um spec, vale o canvas; divergência necessária é registrada no spec da feature e levada ao Ruan. Mudança de design entra no canvas primeiro. Movimento continua em AD-006.
- **Scope**: F11 (UI), F15–F26, F28, F29, F31, F48, lixeira (AD-009) e qualquer tela futura.
- **Date**: 2026-10-15
- **Status**: active

### AD-015
- **Decision**: O auto-update (F37) entra **desligado por padrão**. O plugin só existe no build de release; ele consulta a internet quando a pessoa clica em "Procurar agora" ou liga "Procurar atualizações ao abrir" (Configurações › Privacidade). É a única exceção à regra de zero rede da F36.
- **Reason**: Definido pelo Ruan em 2026-10-28 ("porte o updater desligado por padrão").
- **Trade-off**: Com a opção desligada, ninguém fica sabendo de versão nova sozinho. As atualizações precisam de um repositório público (o `my-photos` é privado) e da chave do updater, gerada no PC do Ruan.
- **Scope**: F36 (`sec04`, `sec07`), F37.
- **Date**: 2026-10-28
- **Status**: active

### AD-016
- **Decision**: O instalador de release leva o **ffmpeg LGPL** (build estático do BtbN) ao lado do app, com a licença e o README da fonte. As atualizações saem dos Releases do próprio `my-photos`, que ficou público.
- **Reason**: Definido pelo Ruan em 2026-10-28 ("empacote o ffmpeg LGPL no instalador"; "o repositório agora tá público").
- **Trade-off**: O instalador cresce uns 85 MB. O build `master-latest` do BtbN é um snapshot de desenvolvimento, e a versão exata fica registrada no `ffmpeg-README.txt`. Um build estável pode ser escolhido com `-Url`.
- **Scope**: F13, F37.
- **Date**: 2026-10-28
- **Status**: active

### AD-017
- **Decision**: A assinatura de código usa um **certificado autoassinado** ("Ruan Diego (My Photos)", RSA 3072, válido até 2031-10-30, impressão digital `E165E7D0C39C8E9C82927DB638F887CFAC5BA566`). O release sai do GitHub Actions (`release.yml`, runner Windows), assinado com esse certificado e com a chave do updater, e é publicado com o `latest.json`.
- **Reason**: Definido pelo Ruan em 2026-10-30 ("pode ser autoassinado"). O repositório ficou público, e o Actions voltou a ter minutos.
- **Trade-off**: Só os PCs que importam o `.cer` confiam no editor. Nos outros, o SmartScreen avisa "Editor desconhecido". Trocar por um certificado de autoridade é só substituir dois secrets e a impressão digital. As chaves privadas foram geradas na sessão do Claude e entregues ao Ruan para os secrets e o cofre de senhas; não estão no repositório.
- **Scope**: F37.
- **Date**: 2026-10-30
- **Status**: active

## Handoff

- **Feature**: MVP (F01–F37) completo no código. F27 pulada (AD-011). Phase 4: F32–F37 na `main`, incluindo a segunda parte da F34 (PR #37), o auto-update desligado por padrão (PR #38, AD-015) e o ffmpeg LGPL no instalador (PR #39, AD-016)
- **Phase / Task**: Phase 4, critério de saída. Falta o primeiro build assinado (AD-017) e as medições no `E:`
- **Completed**: F01–F26, F28–F37. CI `windows-latest` de volta e verde desde 2026-10-28 (repositório público; o PR #39 corrigiu o `pkg01` com CRLF)
- **In-progress**: release 1.0.0 (versão já em 1.0.0 no `package.json` e no `Cargo.toml`). Chave do updater e certificado autoassinado gerados em 2026-10-30; a parte pública está no repositório (`tauri.release.conf.json`, `src-tauri/certs/myphotos-codesign.cer`)
- **Next step**:
  1. Ruan: criar os 4 secrets do `release.yml` (`docs/RELEASE.md` › Secrets) e guardar os arquivos num cofre.
  2. Disparar o release `v1.0.0` (tag ou "Run workflow").
  3. No PC: importar o `.cer` (Root e TrustedPublisher), instalar sem administrador, conferir a assinatura nas Propriedades e o ffmpeg "disponível" em Problemas, abrir no `E:`, ver o backup em `E:\.myphotos\backups`, desinstalar e ver `E:\.myphotos` intacta.
  4. Testes manuais: mover, renomear e lixeira com uma foto de teste; rolar a linha do tempo do `E:`; Narrador no visualizador; atualizar de 1.0.0 para 1.0.1 pela tela.
  5. Métricas do MVP no `E:` (ROADMAP): JSON pareado > 98%, timeline < 2 s, ~60 fps, busca < 300 ms e zero alterações nos arquivos durante a indexação, conferido por hash.
  6. Investigar com o banco do `E:`: os 406 vídeos datados de 23/set, os 169 "JSON em outra parte do export", HEIC sem miniatura (AD-013) e `libraries.volume_id` (F11).
  7. Confirmar AD-012 (fuso -03:00) e AD-013 (HEIC/RAW sem miniatura), ainda "proposed".
- **Blockers**: nenhum no código. O release precisa dos secrets
- **Open questions**: as da F07 continuam (WebP com `Exif\0\0`, HEIC/WebP/RAW reais nunca testados). Mídia atrás de junção que sai do `E:` não abre no visualizador (F36, `sec06`). O ffmpeg vem do `master-latest` do BtbN (snapshot), com a versão registrada no `ffmpeg-README.txt`
- **Uncommitted files**: PRD HTML, `.specs/LESSONS.md`, `.specs/lessons.json` (só na máquina local)
- **Branch**: `main`
