$ErrorActionPreference = 'Stop'
$windowsRoot = Split-Path $PSScriptRoot -Parent
$projectRoot = Split-Path $windowsRoot -Parent
& (Join-Path $PSScriptRoot 'cargo.ps1') -CargoArguments @('build', '--release', '--locked')
if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
$packageVersion = [regex]::Match((Get-Content -LiteralPath (Join-Path $windowsRoot 'Cargo.toml') -Raw), '(?m)^version = "([^"]+)"').Groups[1].Value
if (!$packageVersion) { throw 'Package version not found' }
$packageName = "Kilim-Windows-$packageVersion-x64"
$outputDirectory = Join-Path $projectRoot "dist\$packageName"
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $windowsRoot 'target\release\kilim-windows.exe') -Destination (Join-Path $outputDirectory 'Kilim.exe')
Copy-Item -LiteralPath (Join-Path $windowsRoot 'README.md') -Destination (Join-Path $outputDirectory 'README.md')
Copy-Item -LiteralPath (Join-Path $projectRoot 'LICENSE') -Destination (Join-Path $outputDirectory 'LICENSE')
Copy-Item -LiteralPath (Join-Path $windowsRoot 'PARITY.md') -Destination (Join-Path $outputDirectory 'PARITY.md')
$archive = Join-Path $projectRoot "dist\$packageName.zip"
Compress-Archive -LiteralPath $outputDirectory -DestinationPath $archive -Force
$hashBase = Join-Path $projectRoot 'dist'
$hashes = @((Get-FileHash -LiteralPath (Join-Path $outputDirectory 'Kilim.exe') -Algorithm SHA256),
    (Get-FileHash -LiteralPath $archive -Algorithm SHA256)) | ForEach-Object {
    $relativePath = $_.Path.Substring($hashBase.Length + 1).Replace('\', '/')
    "$($_.Hash.ToLowerInvariant())  $relativePath"
}
Set-Content -LiteralPath (Join-Path $projectRoot 'dist\SHA256SUMS-Windows.txt') -Value $hashes -Encoding ascii
Write-Output $archive
