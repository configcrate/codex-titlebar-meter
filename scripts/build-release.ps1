[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$repository = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$dist = Join-Path $repository 'dist'
$package = Join-Path $dist 'codex-titlebar-meter-v0.1.4-windows-x64'

Push-Location $repository
try {
    cargo test
    if ($LASTEXITCODE -ne 0) { throw 'cargo test failed' }
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'cargo build --release failed' }

    $archive = "$package.zip"
    if ((Test-Path -LiteralPath $package) -or (Test-Path -LiteralPath $archive)) {
        throw "Release output already exists: $package"
    }
    New-Item -ItemType Directory -Path $package -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repository 'target\release\codex-titlebar-meter.exe') -Destination (Join-Path $package 'codex-titlebar-meter.exe')
    Copy-Item -LiteralPath (Join-Path $repository 'install.ps1') -Destination $package
    Copy-Item -LiteralPath (Join-Path $repository 'uninstall.ps1') -Destination $package
    Copy-Item -LiteralPath (Join-Path $repository 'README.md') -Destination $package
    Copy-Item -LiteralPath (Join-Path $repository 'LICENSE') -Destination $package

    Compress-Archive -Path (Join-Path $package '*') -DestinationPath $archive -CompressionLevel Optimal
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    Set-Content -LiteralPath "$archive.sha256" -Value "$hash  $(Split-Path -Leaf $archive)" -Encoding ascii
    Write-Host "Created $archive" -ForegroundColor Green
}
finally {
    Pop-Location
}
