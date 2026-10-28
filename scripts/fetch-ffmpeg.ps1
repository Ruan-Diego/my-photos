# F37: baixa o ffmpeg e o ffprobe (build estático LGPL do BtbN, Windows x64) para
# src-tauri/binaries/ffmpeg/, de onde o instalador os põe ao lado do My Photos.exe,
# onde o app procura primeiro (thumbs::locate_ffmpeg). Chamado pelo release.ps1.
#
#   powershell -ExecutionPolicy Bypass -File scripts/fetch-ffmpeg.ps1
#   powershell -ExecutionPolicy Bypass -File scripts/fetch-ffmpeg.ps1 -Url <outro zip LGPL do BtbN>

param(
    [string]$Url = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-lgpl.zip"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$target = Join-Path $root "src-tauri\binaries\ffmpeg"

if ($Url -notmatch '-lgpl[^/]*\.zip$' -or $Url -match 'shared') {
    throw "Use um zip LGPL estático do BtbN (…-win64-lgpl….zip): o GPL não pode ir no instalador."
}

$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("myphotos-ffmpeg-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    $zip = Join-Path $temp "ffmpeg.zip"
    Write-Host "Baixando $Url ..."
    Invoke-WebRequest -Uri $Url -OutFile $zip -UseBasicParsing
    Expand-Archive -Path $zip -DestinationPath $temp
    $folder = Get-ChildItem -Path $temp -Directory | Select-Object -First 1
    $ffmpeg = Join-Path $folder.FullName "bin\ffmpeg.exe"
    $ffprobe = Join-Path $folder.FullName "bin\ffprobe.exe"

    # A licença vem do próprio binário: o configure de um build LGPL não tem
    # --enable-gpl nem --enable-nonfree.
    $text = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($ffprobe))
    $configure = [regex]::Match($text, '--prefix=[^\x00]*').Value
    if ([string]::IsNullOrEmpty($configure)) { throw "Não achei a linha do configure no ffprobe.exe." }
    if ($configure -match '--enable-gpl|--enable-nonfree') { throw "Este ffmpeg não é LGPL: $configure" }
    # Versão no formato do BtbN: N-128122-g215351c1c3-20261027 (master) ou n7.1.2-15-g…-20261027.
    $version = [regex]::Match($text, '\b(N-\d+-g[0-9a-f]+-\d{8}|n\d+\.\d+(\.\d+)?-\d+-g[0-9a-f]+-\d{8})\b').Value
    if ([string]::IsNullOrEmpty($version)) { $version = "(versão não encontrada)" }

    New-Item -ItemType Directory -Force -Path $target | Out-Null
    Copy-Item $ffmpeg, $ffprobe $target -Force
    Copy-Item (Join-Path $folder.FullName "LICENSE.txt") $target -Force
    $hashes = foreach ($file in "ffmpeg.exe", "ffprobe.exe") {
        "$file  SHA-256 $((Get-FileHash (Join-Path $target $file) -Algorithm SHA256).Hash)"
    }
    # O que a LGPL pede junto do binário: licença, versão e onde está o código-fonte.
    $readme = @(
        "FFmpeg $version, build LGPL do BtbN/FFmpeg-Builds, usado pelo My Photos para a capa e a duração dos vídeos."
        "Licença: GNU LGPL (LICENSE.txt ao lado). O My Photos só chama ffmpeg.exe e ffprobe.exe como programas separados."
        "Código-fonte do FFmpeg: https://git.ffmpeg.org/ffmpeg.git (versão $version)"
        "Scripts do build: https://github.com/BtbN/FFmpeg-Builds"
        "Baixado de: $Url"
        ""
        "Configure: $configure"
        ""
    ) + $hashes
    Set-Content -Path (Join-Path $target "README.txt") -Value $readme -Encoding UTF8
    $hashes | ForEach-Object { Write-Host $_ }
    Write-Host "Pronto: $target ($version, LGPL)"
} finally {
    Remove-Item -Recurse -Force $temp -ErrorAction SilentlyContinue
}
