# F37 — Validation

| Requisito | Teste |
| --------- | ----- |
| PKG-01, PKG-02 | `commands::tests::pkg01_*`. O `cargo check` lê o `tauri.conf.json` pelo `tauri-build`, e a configuração nova é aceita. O nome `PortugueseBR` existe nas strings NSIS do `@tauri-apps/cli` 2.12 |
| PKG-03 | `release.yml`. Não rodou, porque o Actions do Windows está sem runner |
| PKG-04 | Nenhum código usa `app_data_dir` ou `%APPDATA%`; o banco é aberto pelo disco (`db::open_located`) |
| PKG-05 | `commands::tests::sec07_*` (registro só pela config de release, interruptor desligado, endpoint HTTPS, instalação passiva); `sec04` continua barrando os outros crates de rede; `settings.test.ts` UPD-01 e `problems.test.ts` (Privacidade e Diagnóstico dizem quando há rede). `tauri build --no-bundle` com a config de release e uma chave de teste: aceito, e o binário sobe sem pânico. Playwright com IPC simulado: bloco Atualizações em claro e escuro, cartão "Versão X disponível" na lateral, axe sem violações. `scripts/release.ps1` sem erro no parser do PowerShell 7.4 |

Pendente no Windows:
- gerar a chave, rodar `scripts/release.ps1 -Version 0.1.1` e depois `-Version 0.1.2`, instalar a primeira e atualizar pela tela;
- `npm run tauri build`;
- instalar o `x64-setup.exe` sem administrador;
- abrir pelo menu Iniciar e ver a biblioteca do `E:`;
- desinstalar e confirmar que `E:\.myphotos` ficou intacta.
