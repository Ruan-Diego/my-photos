# F37 — Windows Packaging

Meta do ROADMAP (RNF-018): instalador do Windows (MSI/NSIS via Tauri), auto-update e assinatura de código.

## Requisitos

- **PKG-01 (instalador)**: `npm run tauri build` gera `My Photos_<versão>_x64-setup.exe` (NSIS).
  - Instala por usuário (`currentUser`), sem pedir administrador, e o instalador fala português (PortugueseBR).
  - Usa o ícone do app, cria o atalho no menu Iniciar e aparece em "Aplicativos instalados" com a descrição.
  - O WebView2 já vem no Windows 10 e 11. Se faltar, o instalador traz o bootstrapper embutido (`embedBootstrapper`) para instalá-lo.
- **PKG-02 (uma versão só)**: a versão sai do `package.json` (`"version": "../package.json"` no `tauri.conf.json`). O `Cargo.toml` precisa dizer a mesma, e o teste `pkg01` confere isso.
- **PKG-03 (release)**: o workflow `release.yml` roda numa tag `v*` ou à mão.
  - Ele confere que a tag bate com a versão, compila no Windows e anexa o instalador a um release em rascunho no GitHub.
  - Enquanto o Actions não tiver runner, o mesmo instalador sai de `npm run tauri build` no PC.
- **PKG-04 (biblioteca intacta)**: o app não guarda nada em `%APPDATA%`. O banco, as miniaturas, a lixeira e os backups ficam em `E:\.myphotos`. Desinstalar ou reinstalar não toca na biblioteca.

- **PKG-05 (atualização, decidida pelo Ruan em 2026-10-28: desligada por padrão)**:
  - O plugin `updater` do Tauri só é registrado no build de release (`tauri.release.conf.json`, com a chave pública). O build de desenvolvimento não tem updater.
  - Ele só consulta a internet quando a pessoa clica em "Procurar agora" ou liga "Procurar atualizações ao abrir" em Configurações › Privacidade, que vem **desligado** (`app_settings.update_checks`). É a exceção explícita à regra de zero rede da F36 (`sec04`, `sec07`).
  - A atualização vem assinada (minisign) e é conferida contra a chave pública antes de instalar, no modo passivo, sem pedir administrador. Antes de reiniciar, o banco fecha (`DbState::shutdown`).
  - Com a busca ligada, Privacidade e Problemas › Diagnóstico dizem que existe essa conexão; ao abrir, uma versão nova aparece num cartão da barra lateral.
  - `scripts/release.ps1` gera no PC (sem Actions) o instalador, a `.sig` e o `latest.json`. Passo a passo em `docs/RELEASE.md`.

## Decisões que dependem do Ruan

- **Onde ficam as atualizações**: o `my-photos` é privado, e o updater não baixa de repositório privado. O endereço padrão é `Ruan-Diego/my-photos-releases`, um repositório público só para os instaladores, que ainda precisa ser criado.
- **Chave do updater**: é gerada no PC do Ruan (`npx tauri signer generate`), e a pública vai para `tauri.release.conf.json`. Sem ela, o `release.ps1` para com instrução.

- **Assinatura de código**:
  - Precisa de um certificado de assinatura de código. Ele é pago: OV ou EV, ou Azure Trusted Signing.
  - Sem ele, o instalador funciona, mas o SmartScreen mostra "O Windows protegeu o computador" e é preciso clicar em "Mais informações" e depois em "Executar assim mesmo".
  - O `digestAlgorithm` já é `sha256`. Com o certificado, entram `certificateThumbprint` (ou `signCommand`) e `timestampUrl` no `tauri.conf.json`, e um passo de importar o PFX no `release.yml`.
- **ffmpeg**:
  - Continua fora do instalador. O app o procura ao lado do executável e depois no `PATH` (F13).
  - Empacotar acrescenta cerca de 80 a 130 MB e traz a licença do ffmpeg (LGPL ou GPL, conforme o build) para o instalador.

## Divergências do canvas

Nenhuma: a F37 não muda nenhuma tela.
