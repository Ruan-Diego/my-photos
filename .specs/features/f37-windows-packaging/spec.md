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

## Decisões que dependem do Ruan

- **Assinatura de código**:
  - Precisa de um certificado de assinatura de código. Ele é pago: OV ou EV, ou Azure Trusted Signing.
  - Sem ele, o instalador funciona, mas o SmartScreen mostra "O Windows protegeu o computador" e é preciso clicar em "Mais informações" e depois em "Executar assim mesmo".
  - O `digestAlgorithm` já é `sha256`. Com o certificado, entram `certificateThumbprint` (ou `signCommand`) e `timestampUrl` no `tauri.conf.json`, e um passo de importar o PFX no `release.yml`.
- **Atualização automática**:
  - O plugin `updater` do Tauri consulta a internet, o que vai contra a regra de "zero rede por padrão" (F36, teste `sec04`). Por isso ele não entrou.
  - Por enquanto, atualizar é baixar o instalador novo do release e rodar por cima. As configurações e a biblioteca ficam.
  - Se o Ruan quiser, há duas opções: o updater desligado por padrão, com uma opção "Procurar atualizações" em Configurações, ou um botão que só abre a página de releases.
- **ffmpeg**:
  - Continua fora do instalador. O app o procura ao lado do executável e depois no `PATH` (F13).
  - Empacotar acrescenta cerca de 80 a 130 MB e traz a licença do ffmpeg (LGPL ou GPL, conforme o build) para o instalador.

## Divergências do canvas

Nenhuma: a F37 não muda nenhuma tela.
