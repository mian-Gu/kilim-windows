param([Parameter(ValueFromRemainingArguments = $true)][string[]]$CargoArguments)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
$localCargo = Join-Path $projectRoot '.tools\cargo\bin\cargo.exe'
$savedCargoHome = $env:CARGO_HOME
$savedRustupHome = $env:RUSTUP_HOME
$savedPath = $env:PATH
Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    if (Test-Path -LiteralPath $localCargo) {
        $env:CARGO_HOME = Join-Path $projectRoot '.tools\cargo'
        $env:RUSTUP_HOME = Join-Path $projectRoot '.tools\rustup'
        $env:PATH = "$(Split-Path $localCargo -Parent);$env:PATH"
        & $localCargo @CargoArguments
    } else {
        & cargo @CargoArguments
    }
    $cargoExitCode = $LASTEXITCODE
} finally {
    Pop-Location
    $env:CARGO_HOME = $savedCargoHome
    $env:RUSTUP_HOME = $savedRustupHome
    $env:PATH = $savedPath
}
exit $cargoExitCode
