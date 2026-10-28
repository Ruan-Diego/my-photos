# F37: gera no PC (sem Actions) o instalador do Windows, com o updater, o ffmpeg LGPL e
# o latest.json.
# Na raiz do repositório:
#
#   powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Version 1.0.0
#   powershell -ExecutionPolicy Bypass -File scripts/release.ps1 -Version 1.0.0 -Publish
#
# Passo a passo e a chave do updater em docs/RELEASE.md.
param(
    [Parameter(Mandatory = $true)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [string]$Notes = "",
    [switch]$Publish
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$tauri = Join-Path $root "src-tauri"
Set-Location $root

function Set-Utf8([string]$Path, [string]$Text) {
    [System.IO.File]::WriteAllText($Path, $Text, (New-Object System.Text.UTF8Encoding $false))
}

# 1. Chave do updater: a pública no tauri.release.conf.json, a privada fora do repositório.
$releaseFile = Join-Path $tauri "tauri.release.conf.json"
$releaseConf = Get-Content $releaseFile -Raw -Encoding UTF8 | ConvertFrom-Json
if ([string]::IsNullOrWhiteSpace($releaseConf.plugins.updater.pubkey)) {
    throw "Falta a chave pública do updater em src-tauri/tauri.release.conf.json (plugins.updater.pubkey). Veja docs/RELEASE.md."
}
if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
    $keyFile = Join-Path $HOME ".tauri\myphotos.key"
    if (-not (Test-Path $keyFile)) {
        throw "Defina TAURI_SIGNING_PRIVATE_KEY ou crie a chave em $keyFile (docs/RELEASE.md)."
    }
    $env:TAURI_SIGNING_PRIVATE_KEY = Get-Content $keyFile -Raw
}

# 2. Uma versão só (pkg01): package.json manda, Cargo.toml acompanha.
$package = Join-Path $root "package.json"
Set-Utf8 $package ((Get-Content $package -Raw -Encoding UTF8) -replace '"version": "[^"]+"', "`"version`": `"$Version`"")
$cargo = Join-Path $tauri "Cargo.toml"
Set-Utf8 $cargo ((Get-Content $cargo -Raw -Encoding UTF8) -replace '(?m)^version = "[^"]+"', "version = `"$Version`"")

# 3. ffmpeg LGPL ao lado do app (tauri.release.conf.json › bundle.resources).
if (-not (Test-Path (Join-Path $tauri "binaries\ffmpeg\ffprobe.exe"))) {
    & (Join-Path $PSScriptRoot "fetch-ffmpeg.ps1")
}

# 4. Testes e build com a config de release (updater, ffmpeg e artefatos assinados).
npm ci
if ($LASTEXITCODE -ne 0) { throw "npm ci falhou" }
cargo test --manifest-path src-tauri/Cargo.toml
if ($LASTEXITCODE -ne 0) { throw "os testes do Rust falharam" }
npx tauri build --bundles nsis --config $releaseFile
if ($LASTEXITCODE -ne 0) { throw "tauri build falhou" }

# 5. release/v<versão>: instalador, a assinatura do updater e o latest.json.
$nsis = Join-Path $tauri "target\release\bundle\nsis"
$installer = Get-ChildItem $nsis -Filter "*_${Version}_x64-setup.exe" | Select-Object -First 1
if ($null -eq $installer) { throw "Instalador não encontrado em $nsis" }
$out = Join-Path $root "release\v$Version"
New-Item -ItemType Directory -Force -Path $out | Out-Null
# O GitHub troca espaços por pontos no nome do anexo; sem espaço, a URL é previsível.
$name = "MyPhotos_${Version}_x64-setup.exe"
Copy-Item $installer.FullName (Join-Path $out $name) -Force
Copy-Item "$($installer.FullName).sig" (Join-Path $out "$name.sig") -Force

$endpoint = $releaseConf.plugins.updater.endpoints[0]
if ($endpoint -notmatch '^https://github\.com/([^/]+)/([^/]+)/releases/') {
    throw "O endpoint do updater não é um release do GitHub: $endpoint. Monte o latest.json à mão."
}
$repo = "$($Matches[1])/$($Matches[2])"
$latest = [ordered]@{
    version   = $Version
    notes     = $Notes
    pub_date  = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    platforms = [ordered]@{
        "windows-x86_64" = [ordered]@{
            signature = (Get-Content (Join-Path $out "$name.sig") -Raw).Trim()
            url       = "https://github.com/$repo/releases/download/v$Version/$name"
        }
    }
}
Set-Utf8 (Join-Path $out "latest.json") ($latest | ConvertTo-Json -Depth 5)
Write-Host "Pronto: $out"

# 6. Publicar no repositório de atualizações (precisa do gh logado).
if ($Publish) {
    $body = if ($Notes) { $Notes } else { "My Photos $Version" }
    gh release create "v$Version" (Join-Path $out $name) (Join-Path $out "$name.sig") (Join-Path $out "latest.json") `
        --repo $repo --title "My Photos $Version" --notes $body
    if ($LASTEXITCODE -ne 0) { throw "gh release create falhou" }
    Write-Host "Publicado em https://github.com/$repo/releases/tag/v$Version"
}
