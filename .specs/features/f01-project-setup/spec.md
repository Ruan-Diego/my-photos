# F01 — Project Setup Specification

## Problem Statement

O repositório do My Photos ainda não é um aplicativo: só há documentação. Sem um projeto Tauri 2 + React + TypeScript que abre no Windows, conversa com o backend Rust e prova que o SQLite e as bibliotecas de movimento resolvem, nenhuma feature seguinte (schema, indexação, UI) tem onde existir.

## Goals

- [ ] Um desenvolvedor no Windows 10/11 sobe o app com um comando documentado e vê a janela **My Photos** comprovar o canal Rust ↔ React
- [ ] `rusqlite` abre um SQLite em memória e um teste Rust prova `SELECT 1`
- [ ] `three`, `gsap`, `animejs` e `motion` são dependências reais do frontend e entram no build
- [ ] CI executa os mesmos checks de tipo, build e teste, e falha quando qualquer um falha

## Out of Scope

Explicitamente fora. Documentado para impedir escopo extra.

| Feature | Reason |
| ------- | ------ |
| Esquema SQLite, migrações, FTS5 (F04) | F01 só prova que a conexão abre. O modelo de dados é outra feature |
| Scanner, parsers, matcher, pipeline (F05–F10) | Não há biblioteca de fotos neste setup |
| Telas de produto: shell, onboarding, timeline, viewer (F15+) | A janela desta feature é um comprovante de stack, não a UI do produto |
| Animação, WebGL, `prefers-reduced-motion` | AD-006 governa as telas futuras. F01 não desenha cena nem timeline de movimento |
| i18n completa (F34) | Textos do comprovante ficam em pt-BR num único módulo. O sistema de tradução vem depois |
| Instalador, auto-update, assinatura (F37) | Fora do scaffold de desenvolvimento |
| macOS e Linux | AD-003: Windows é a plataforma do MVP |
| Indexação, thumbnails, ffmpeg, EXIF | Dependências dessas features não entram no scaffold |

---

## Assumptions & Open Questions

Cada ambiguidade está resolvida ou registrada aqui.

| Assumption / decision | Chosen default | Rationale | Confirmed? |
| --------------------- | -------------- | --------- | ---------- |
| Gerenciador de pacotes do frontend | npm | É o padrão do template oficial Tauri + Vite. Um único lockfile (`package-lock.json`) | y |
| Comando de desenvolvimento | `npm run tauri dev`, documentado no README | Alinha o script ao CLI do Tauri 2 (`tauri dev`) | y |
| Comando de build do frontend | `npm run build` executa checagem TypeScript e em seguida o build de produção do Vite | O template React-TS do Vite falha o build quando o tipo não fecha; isso torna o import das libs verificável | y |
| Identificador do app | `com.myphotos.desktop` | Estável, sem domínio de terceiros inventado | y |
| Nome da janela e do produto | `My Photos` | AD-001 | y |
| Driver SQLite | `rusqlite` 0.40.2 com feature `bundled` | AD-002 escolhe `rusqlite`. `bundled` evita depender de um SQLite instalado no Windows ou no runner de CI. Versão publicada em crates.io em 2026-08-08 | y |
| Banco do comprovante | conexão em memória (`:memory:`). Nenhum arquivo `.db` é criado | O banco da biblioteca do usuário é F04/F31. F01 não pode semear dados persistentes | y |
| Comando Tauri | `app_health`, sem argumentos | Um único contrato verificável de ponta a ponta | y |
| Payload de sucesso | `{ "appName": "My Photos", "sqliteOk": true }` | Resultado único, comparável por teste e pela UI | y |
| CI | GitHub Actions em `.github/workflows/ci.yml`, nos eventos `push` e `pull_request` | Repositório já usa git; o roadmap pede CI básico. O workflow não abre a janela (runner sem sessão gráfica confiável) | y |
| Toolchain | Rust stable e Node.js Active LTS. WebView2 é pré-requisito no Windows | Não fixar versão de patch inventada. O README nomeia esses pré-requisitos | y |
| Textos visíveis do comprovante | pt-BR, hardcoded em um módulo | AD-005. F34 substitui esse módulo por tradução de verdade | y |
| A janela não anima | Nenhuma das quatro bibliotecas roda um loop visual no comprovante | AD-006 reserva movimento para telas de produto. Aqui a prova é o import, não o efeito | y |

**Open questions:** nenhuma. Spec confirmada pelo stakeholder em 2026-09-27, com os defaults acima. A pasta da biblioteca `E:\` é AD-007 e permanece fora desta feature.

### Dimensões implícitas

| Dimension | Resolução |
| --------- | --------- |
| Input validation & bounds | N/A — `app_health` não recebe argumentos nem input do usuário |
| Failure / partial-failure | WHEN a abertura do SQLite ou o `SELECT 1` falha THEN `app_health` SHALL retornar erro e a UI SHALL mostrar `Falha: ` seguido da mensagem. WHEN typecheck, build ou `cargo test` falha THEN o CI SHALL falhar o job |
| Idempotency / retry | N/A — não há gravação. Rodar o app ou o CI de novo não cria estado do usuário |
| Auth boundaries & rate limits | N/A — app desktop local, sem rede e sem contas (RNF de privacidade fica para F36) |
| Concurrency / ordering | N/A — um comando síncrono de leitura em conexão recém-aberta |
| Data lifecycle / expiry | N/A para dados de usuário — a conexão é em memória e morre com o comando. Nenhum arquivo de biblioteca é criado |
| Observability | N/A além do retorno do comando e do texto na janela. Sem telemetria e sem rede |
| External-dependency failure | WHEN `three`, `gsap`, `animejs` ou `motion` não resolve THEN `npm run build` SHALL falhar. WHEN `rusqlite` não abre SQLite THEN o teste Rust e `app_health` SHALL falhar |
| State-transition integrity | N/A — o comprovante não tem máquina de estados |

---

## User Stories

### P1: Comprovante da stack no Windows ⭐ MVP

**User Story**: Como desenvolvedor do My Photos, quero abrir o app no Windows e ver o frontend receber um resultado real do Rust, para saber que Tauri, React e SQLite estão operacionais antes de qualquer feature.

**Why P1**: O critério de saída da Phase 0 exige a stack de ponta a ponta. Sem isso, F04 em diante não tem processo hospedeiro.

**Acceptance Criteria**:

1. WHEN o desenvolvedor, no Windows 10 ou 11, com Rust stable, Node.js Active LTS e WebView2, executa `npm run tauri dev` na raiz do repositório THEN o sistema SHALL abrir uma janela de desktop cujo título é exatamente `My Photos`.
2. WHEN a janela termina de carregar THEN a interface SHALL invocar o comando Tauri `app_health` (sem argumentos) e SHALL renderizar o texto exato `My Photos está pronto.`
3. WHEN `app_health` abre uma conexão SQLite em memória com `rusqlite`, executa `SELECT 1` e obtém `1` THEN o comando SHALL retornar `{ "appName": "My Photos", "sqliteOk": true }` e a interface SHALL renderizar o texto exato `SQLite: ok`.
4. WHEN a abertura da conexão ou o `SELECT 1` falha THEN `app_health` SHALL retornar erro e a interface SHALL renderizar `Falha: ` seguido da mensagem de erro, e SHALL NOT renderizar `My Photos está pronto.` nem `SQLite: ok`.
5. WHEN `app_health` é executado THEN o sistema SHALL NOT criar, alterar ou apagar qualquer arquivo fora do diretório do projeto e do temporário do processo. Em particular, SHALL NOT criar um arquivo `.db` de biblioteca.

**Independent Test**: `npm run tauri dev`, conferir o título da janela e os dois textos de sucesso. Derrubar o `SELECT 1` num build local temporário e conferir o prefixo `Falha:`.

---

### P1: Bibliotecas de movimento entram no build ⭐ MVP

**User Story**: Como desenvolvedor, quero `three`, `gsap`, `animejs` e `motion` instaladas e importadas pelo frontend, para as telas futuras não descobrirem uma dependência ausente.

**Why P1**: AD-006 e o roadmap colocam essas quatro dependências no scaffold da F01.

**Acceptance Criteria**:

1. WHEN `package.json` é lido THEN `dependencies` SHALL incluir `three`, `gsap`, `animejs` e `motion`.
2. WHEN o módulo de frontend importado pelo entrypoint do Vite importa cada um desses quatro pacotes e expõe os nomes THEN a janela SHALL renderizar o texto exato `three, gsap, animejs, motion`.
3. WHEN `npm run build` roda THEN o build SHALL concluir com sucesso, incluindo checagem TypeScript. WHEN qualquer um dos quatro pacotes é removido do `package.json` THEN `npm run build` SHALL falhar.
4. WHEN a janela do comprovante está visível THEN o sistema SHALL NOT criar um renderer WebGL nem iniciar um loop de animação com essas bibliotecas.

**Independent Test**: `npm run build` passa; a janela mostra a lista exata; apagar `motion` de `dependencies` faz o build falhar.

---

### P1: Layout do repositório ⭐ MVP

**User Story**: Como desenvolvedor, quero pastas separadas para comandos Rust, acesso a SQLite e UI React, para as features seguintes terem um lugar estável.

**Why P1**: O roadmap define estrutura de pastas como entrega da F01.

**Acceptance Criteria**:

1. WHEN o repositório é inspecionado THEN SHALL existir `src-tauri/` com o backend Rust (Tauri 2) e `src/` com o frontend React + TypeScript + Vite.
2. WHEN o backend é inspecionado THEN SHALL existir `src-tauri/src/commands/` contendo o comando `app_health` e `src-tauri/src/db/` contendo a abertura da conexão SQLite em memória.
3. WHEN `src-tauri/Cargo.toml` é lido THEN SHALL declarar dependência de `tauri` na linha 2.x e de `rusqlite` com a feature `bundled`.
4. WHEN o README na raiz é lido THEN SHALL documentar os pré-requisitos (Windows 10/11, Rust stable, Node.js Active LTS, WebView2) e o comando `npm run tauri dev`.

**Independent Test**: A árvore de diretórios e o README batem com os quatro critérios, sem abrir o app.

---

### P1: CI básico ⭐ MVP

**User Story**: Como desenvolvedor, quero que push e pull request executem typecheck, build do frontend e testes Rust, para o scaffold não quebrar em silêncio.

**Why P1**: O roadmap inclui CI básico na F01, no mesmo nível do scaffold.

**Acceptance Criteria**:

1. WHEN há um `push` ou `pull_request` THEN o workflow `.github/workflows/ci.yml` SHALL executar `npm run build` e `cargo test` no crate em `src-tauri`.
2. WHEN `npm run build` ou `cargo test` retorna código diferente de zero THEN o workflow SHALL terminar com falha.
3. WHEN `cargo test` roda THEN ao menos um teste SHALL abrir SQLite em memória com `rusqlite`, executar `SELECT 1` e afirmar que o valor retornado é `1`.
4. WHEN o workflow roda THEN SHALL NOT lançar a janela desktop.

**Independent Test**: `npm run build` e `cargo test` na máquina local exercem os mesmos comandos do workflow. O teste Rust falha se `SELECT 1` não devolver `1`.

---

## Edge Cases

- WHEN WebView2 não está instalado THEN a janela SHALL não abrir e o erro exibido SHALL ser o do runtime Tauri/SO. F01 SHALL NOT embutir um instalador de WebView2.
- WHEN Rust ou Node.js estão ausentes THEN `npm run tauri dev` SHALL falhar no toolchain, e o README SHALL ser a referência dos pré-requisitos. O projeto SHALL NOT baixar toolchain automaticamente.
- WHEN `app_health` é chamado duas vezes THEN cada chamada SHALL abrir a própria conexão em memória, retornar o mesmo payload de sucesso e SHALL NOT reutilizar arquivo em disco.
- WHEN o frontend recebe `sqliteOk` diferente de `true` dentro de um resultado de sucesso THEN a interface SHALL renderizar exatamente `Falha: sqliteOk inválido` e SHALL NOT renderizar `My Photos está pronto.`, `SQLite: ok` nem `three, gsap, animejs, motion`. O comando, no caminho feliz, só produz sucesso com `sqliteOk: true`.

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status |
| -------------- | ----- | ----- | ------ |
| SETUP-01 | P1: Comprovante da stack | Execute | Verified |
| SETUP-02 | P1: Comprovante da stack | Execute | Verified |
| SETUP-03 | P1: Comprovante da stack | Execute | Verified |
| SETUP-04 | P1: Comprovante da stack | Execute | Verified |
| SETUP-05 | P1: Comprovante da stack | Execute | Verified |
| SETUP-06 | P1: Bibliotecas de movimento | Execute | Verified |
| SETUP-07 | P1: Bibliotecas de movimento | Execute | Verified |
| SETUP-08 | P1: Bibliotecas de movimento | Execute | Verified |
| SETUP-09 | P1: Bibliotecas de movimento | Execute | Verified |
| SETUP-10 | P1: Layout do repositório | Execute | Verified |
| SETUP-11 | P1: Layout do repositório | Execute | Verified |
| SETUP-12 | P1: Layout do repositório | Execute | Verified |
| SETUP-13 | P1: Layout do repositório | Execute | Verified |
| SETUP-14 | P1: CI básico | Execute | Verified |
| SETUP-15 | P1: CI básico | Execute | Verified |
| SETUP-16 | P1: CI básico | Execute | Verified |
| SETUP-17 | P1: CI básico | Execute | Verified |

**Coverage:** 17 requisitos, 0 mapeados para tasks, 17 unmapped. O mapeamento acontece na fase de tasks.

---

## Success Criteria

- [ ] No Windows, `npm run tauri dev` abre a janela `My Photos` com os textos `My Photos está pronto.`, `SQLite: ok` e `three, gsap, animejs, motion`
- [ ] `cargo test` passa e cobre `SELECT 1 = 1` via `rusqlite` bundled, sem criar arquivo `.db`
- [ ] `npm run build` passa com as quatro bibliotecas importadas e falha se uma delas some
- [ ] `.github/workflows/ci.yml` executa esses dois comandos em `push` e `pull_request` e não abre a janela
- [ ] Nenhum arquivo de foto ou de biblioteca do usuário é criado, alterado ou apagado
