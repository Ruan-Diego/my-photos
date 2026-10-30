# Release do My Photos (Windows)

O release sai do GitHub Actions (`.github/workflows/release.yml`), num runner Windows. O repositório é público, então os minutos são gratuitos. O workflow:

- baixa o ffmpeg LGPL;
- assina o `My Photos.exe` e o instalador com o certificado de código;
- assina a atualização com a chave do updater;
- publica o release com o `latest.json` que o auto-update dos apps instalados lê.

## Uma vez só

### 1. Secrets do repositório

As chaves foram geradas em 2026-10-30 (AD-017). Só a parte pública está no repositório:

- a chave pública do updater, em `src-tauri/tauri.release.conf.json`;
- a impressão digital do certificado, no mesmo arquivo;
- o certificado público, em `src-tauri/certs/myphotos-codesign.cer`.

As partes privadas ficam em 4 secrets. Crie cada um em **Settings › Secrets and variables › Actions › New repository secret**:

| Secret | Conteúdo |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | o texto do arquivo `myphotos-updater.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | o texto de `updater-key-password.txt` |
| `WINDOWS_CERTIFICATE` | o texto de `myphotos-codesign.pfx.base64.txt` (o `.pfx` em base64) |
| `WINDOWS_CERTIFICATE_PASSWORD` | o texto de `codesign-pfx-password.txt` |

Guarde também os quatro arquivos num cofre de senhas. Sem a chave do updater e a senha dela, as versões já instaladas não aceitam mais nenhuma atualização. O workflow para com uma mensagem clara se faltar algum secret, e confere que o certificado do secret é o mesmo da impressão digital da config.

### 2. Confiar no certificado neste PC

O certificado é **autoassinado** (AD-017). Só confiam nele os computadores onde ele for importado. Para o Windows mostrar "Ruan Diego (My Photos)" como editor verificado no seu PC, rode uma vez no PowerShell, na raiz do repositório:

```
Import-Certificate -FilePath src-tauri\certs\myphotos-codesign.cer -CertStoreLocation Cert:\CurrentUser\Root
Import-Certificate -FilePath src-tauri\certs\myphotos-codesign.cer -CertStoreLocation Cert:\CurrentUser\TrustedPublisher
```

O primeiro comando abre uma confirmação do Windows. Em outros computadores, o SmartScreen continua avisando "Editor desconhecido" ("Mais informações" › "Executar assim mesmo"). Para isso sumir, só com um certificado de uma autoridade (Certum, Sectigo, Azure Trusted Signing), que entra no lugar dos dois secrets do certificado.

### 3. Onde ficam as atualizações

`plugins.updater.endpoints` aponta para o `latest.json` do último release publicado do próprio `my-photos`:

```
https://github.com/Ruan-Diego/my-photos/releases/latest/download/latest.json
```

Um release em rascunho ou pré-release não conta como "latest". Só o publicado chega aos apps instalados.

## ffmpeg

O instalador de release leva o `ffmpeg.exe` e o `ffprobe.exe` ao lado do `My Photos.exe`, que é onde o app procura primeiro. Eles fazem a capa e a duração dos vídeos.

- A fonte é o build **LGPL** estático do [BtbN/FFmpeg-Builds](https://github.com/BtbN/FFmpeg-Builds) (`ffmpeg-master-latest-win64-lgpl.zip`).
- O workflow (e o `release.ps1`) baixa os dois com o `scripts/fetch-ffmpeg.ps1` para `src-tauri/binaries/ffmpeg/`, que fica fora do git.
- O script recusa um build GPL: ele lê o configure gravado no próprio `ffprobe.exe` e para se achar `--enable-gpl` ou `--enable-nonfree`.
- Junto vão o `ffmpeg-LICENSE.txt` (LGPL) e o `ffmpeg-README.txt`, com a versão exata, o configure, o SHA-256 dos dois executáveis e onde está o código-fonte. É o que a LGPL pede para redistribuir.
- O app só chama o ffmpeg como programa separado, sem ligar o código dele ao seu.
- Custo: os dois executáveis têm 139 MB cada, e o instalador cresce uns 85 MB.
- Para trocar de build (uma versão estável, por exemplo): apague `src-tauri/binaries/ffmpeg/` e rode `scripts/fetch-ffmpeg.ps1 -Url <zip LGPL do BtbN>`.
- O build de desenvolvimento não leva o ffmpeg e continua achando o do `PATH`.

## Cada versão

1. Ponha a versão nova no `package.json` (`npm version 1.1.0 --no-git-tag-version`) e no `Cargo.toml`. O teste `pkg01` exige as duas iguais. Faça o merge na `main`.
2. Crie a tag e envie: `git tag v1.1.0 && git push origin v1.1.0`. Também dá para usar Actions › release › "Run workflow".
3. Em uns 15 minutos, o release `v1.1.0` aparece publicado com o `My Photos_1.1.0_x64-setup.exe`, a `.sig` e o `latest.json`. Os apps com a busca de atualizações ligada já enxergam a versão nova.

### Sem Actions, no PC

`scripts/release.ps1 -Version 1.1.0 -Publish` faz o mesmo localmente. Antes, importe o `.pfx` no repositório de certificados do Windows (duplo clique no arquivo, "Usuário atual") e deixe a chave em `$HOME\.tauri\myphotos.key`, com `$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD` definido.

## Como o app atualiza

- O build de desenvolvimento (`npm run tauri dev`) não tem updater.
- No instalado, nada sai do computador até a pessoa clicar em Configurações › Privacidade › "Procurar agora" ou ligar "Procurar atualizações ao abrir", que vem desligado.
- Achando uma versão nova, o app mostra "Instalar e reiniciar". Ele baixa, confere a assinatura, fecha o banco e instala no modo passivo, sem pedir administrador. A biblioteca em `E:\.myphotos` não é tocada.
