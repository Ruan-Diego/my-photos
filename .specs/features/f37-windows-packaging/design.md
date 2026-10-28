# F37 — Design

- `tauri.conf.json`:
  - `version` vem de `../package.json`;
  - em `bundle`: `targets: ["nsis"]`, `publisher`, `copyright`, `category: Photography` e as descrições curta e longa;
  - em `bundle.windows`: `digestAlgorithm: sha256` e `webviewInstallMode: embedBootstrapper`;
  - em `bundle.windows.nsis`: `installMode: currentUser`, `languages: ["PortugueseBR"]`, sem seletor de idioma e `installerIcon` igual ao ícone do app.
  - O MSI (WiX) ficou de fora. Ele exige administrador, só instala por máquina e não aceita o modo por usuário.
- `Cargo.toml`: descrição e autor reais, que antes eram os do modelo do Tauri.
- `.github/workflows/release.yml`:
  - roda numa tag `v*` ou por `workflow_dispatch`;
  - confere a tag contra o `package.json`;
  - usa `tauri-apps/tauri-action@v0` com `--bundles nsis` e `releaseDraft: true`.
- Teste `commands::tests::pkg01_*`: uma versão só, só NSIS, por usuário, em português.
