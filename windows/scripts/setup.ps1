$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$downloadDirectory = Join-Path $projectRoot '.tools\downloads'
New-Item -ItemType Directory -Path $downloadDirectory -Force | Out-Null
$installer = Join-Path $downloadDirectory 'rustup-init.exe'
$downloadUrl = 'https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe'
Invoke-WebRequest -Uri $downloadUrl -OutFile $installer
$checksum = (Invoke-WebRequest -Uri "$downloadUrl.sha256").Content
if ($checksum -is [byte[]]) { $checksum = [Text.Encoding]::UTF8.GetString($checksum) }
$expectedHash = ($checksum.Trim() -split '\s+')[0]
if ((Get-FileHash -LiteralPath $installer -Algorithm SHA256).Hash -ine $expectedHash) {
    throw 'Rustup checksum mismatch'
}
$savedCargoHome = $env:CARGO_HOME
$savedRustupHome = $env:RUSTUP_HOME
try {
    $env:CARGO_HOME = Join-Path $projectRoot '.tools\cargo'
    $env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
    & $installer -y --no-modify-path --profile minimal --default-toolchain stable-x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Rust installation failed' }
    & (Join-Path $env:CARGO_HOME 'bin\rustup.exe') component add rustfmt clippy
    if ($LASTEXITCODE -ne 0) { throw 'Rust component installation failed' }
} finally {
    $env:CARGO_HOME = $savedCargoHome
    $env:RUSTUP_HOME = $savedRustupHome
}
