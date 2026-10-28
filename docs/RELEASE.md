# Release do My Photos (Windows)

Duas formas de gerar o instalador:

- **No PC**, sem Actions: `scripts/release.ps1`, que também gera o `latest.json` do auto-update.
- **No GitHub Actions** (`release.yml`), por tag `v*`. Para sair com o updater e o ffmpeg, o workflow ainda precisa rodar o `fetch-ffmpeg.ps1`, usar `--config src-tauri/tauri.release.conf.json` e ter a chave privada nos secrets.

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

`plugins.updater.endpoints` aponta para o `latest.json` do último release publicado do próprio `my-photos`, que é público:

```
https://github.com/Ruan-Diego/my-photos/releases/latest/download/latest.json
```

Um release em rascunho ou marcado como pré-release não conta como "latest". Só o publicado chega aos apps instalados.

### 3. ffmpeg

O instalador de release leva o `ffmpeg.exe` e o `ffprobe.exe` ao lado do `My Photos.exe`, que é onde o app procura primeiro. Eles fazem a capa e a duração dos vídeos.

- A fonte é o build **LGPL** estático do [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) (`ffmpeg-master-latest-win64-lgpl.zip`).
- O `release.ps1` baixa os dois com o `scripts/fetch-ffmpeg.ps1` para `src-tauri/binaries/ffmpeg/`, que fica fora do git.
- O script recusa um build GPL: ele lê o configure gravado no próprio `ffprobe.exe` e para se achar `--enable-gpl` ou `--enable-nonfree`.
- Junto vão o `ffmpeg-LICENSE.txt` (LGPL) e o `ffmpeg-README.txt`, com a versão exata, o configure, o SHA-256 dos dois executáveis e onde está o código-fonte. É o que a LGPL pede para redistribuir.
- O app só chama o ffmpeg como programa separado, sem ligar o código dele ao seu.
- Custo: os dois executáveis têm 139 MB cada, e o instalador cresce uns 85 MB.
- Para trocar de build (uma versão estável, por exemplo): apague `src-tauri/binaries/ffmpeg/` e rode `scripts/fetch-ffmpeg.ps1 -Url <zip LGPL do BtbN>`.
- O build de desenvolvimento não leva o ffmpeg e continua achando o do `PATH`.

## Cada versão

```
powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Version 0.2.0 -Notes "O que mudou"
```

O script:

1. confere a chave pública e a privada;
2. põe a versão no `package.json` e no `Cargo.toml` (o teste `pkg01` exige as duas iguais);
3. baixa o ffmpeg LGPL, se ainda não estiver em `src-tauri/binaries/ffmpeg/`;
4. roda `npm ci`, os testes do Rust e `tauri build` com `tauri.release.conf.json`;
5. deixa em `release/v0.2.0/` o `MyPhotos_0.2.0_x64-setup.exe`, a `.sig` e o `latest.json`.

Com `-Publish`, ele também cria e publica o release `v0.2.0` no `my-photos` (precisa do `gh` logado). Depois, faça commit da versão nova.

## Como o app atualiza

- O build de desenvolvimento (`npm run tauri dev`) não tem updater.
- No instalado, nada sai do computador até a pessoa clicar em Configurações › Privacidade › "Procurar agora" ou ligar "Procurar atualizações ao abrir", que vem desligado.
- Achando uma versão nova, o app mostra "Instalar e reiniciar". Ele baixa, confere a assinatura, fecha o banco e instala no modo passivo, sem pedir administrador. A biblioteca em `E:\.myphotos` não é tocada.
