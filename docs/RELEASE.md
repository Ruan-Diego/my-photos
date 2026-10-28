# Release do My Photos (Windows)

Duas formas de gerar o instalador:

- **No PC**, sem Actions: `scripts/release.ps1`, que também gera o `latest.json` do auto-update.
- **No GitHub Actions** (`release.yml`), por tag `v*`. Para sair com o updater, o workflow ainda precisa do `--config src-tauri/tauri.release.conf.json` e da chave privada nos secrets.

## Uma vez só

### 1. Chave do auto-update

O app só instala uma atualização assinada com esta chave. A chave privada **nunca** vai para o repositório.

```
npx tauri signer generate -w $HOME\.tauri\myphotos.key
```

1. Copie a chave **pública** que o comando imprime para `src-tauri/tauri.release.conf.json`, em `plugins.updater.pubkey`, e faça commit.
2. Guarde a chave privada e a senha num cofre de senhas. Sem elas, as versões já instaladas não aceitam mais nenhuma atualização.
3. Se você usou senha, rode `$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "..."` antes do release.

### 2. Onde ficam as atualizações

`plugins.updater.endpoints` aponta para o `latest.json` do último release de um repositório **público**. O `my-photos` é privado, e o app não tem como se logar no GitHub:

```
https://github.com/Ruan-Diego/my-photos-releases/releases/latest/download/latest.json
```

Crie o repositório público `my-photos-releases` (vazio, só para os instaladores) ou troque o endereço.

## Cada versão

```
powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Version 0.2.0 -Notes "O que mudou"
```

O script:

1. confere a chave pública e a privada;
2. põe a versão no `package.json` e no `Cargo.toml` (o teste `pkg01` exige as duas iguais);
3. roda `npm ci`, os testes do Rust e `tauri build` com `tauri.release.conf.json`;
4. deixa em `release/v0.2.0/` o `MyPhotos_0.2.0_x64-setup.exe`, a `.sig` e o `latest.json`.

Com `-Publish`, ele também cria o release `v0.2.0` no repositório de atualizações (precisa do `gh` logado). Depois, faça commit da versão nova.

## Como o app atualiza

- O build de desenvolvimento (`npm run tauri dev`) não tem updater.
- No instalado, nada sai do computador até a pessoa clicar em Configurações › Privacidade › "Procurar agora" ou ligar "Procurar atualizações ao abrir", que vem desligado.
- Achando uma versão nova, o app mostra "Instalar e reiniciar". Ele baixa, confere a assinatura, fecha o banco e instala no modo passivo, sem pedir administrador. A biblioteca em `E:\.myphotos` não é tocada.
