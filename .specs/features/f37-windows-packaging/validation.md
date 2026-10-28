# F37 — Validation

| Requisito | Teste |
| --------- | ----- |
| PKG-01, PKG-02 | `commands::tests::pkg01_*`. O `cargo check` lê o `tauri.conf.json` pelo `tauri-build`, e a configuração nova é aceita. O nome `PortugueseBR` existe nas strings NSIS do `@tauri-apps/cli` 2.12 |
| PKG-03 | `release.yml`. Não rodou, porque o Actions do Windows está sem runner |
| PKG-04 | Nenhum código usa `app_data_dir` ou `%APPDATA%`; o banco é aberto pelo disco (`db::open_located`) |

Pendente no Windows:
- `npm run tauri build`;
- instalar o `x64-setup.exe` sem administrador;
- abrir pelo menu Iniciar e ver a biblioteca do `E:`;
- desinstalar e confirmar que `E:\.myphotos` ficou intacta.
