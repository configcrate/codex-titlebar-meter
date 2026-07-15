[CmdletBinding()]
param(
    [switch]$PurgeSettings
)

$ErrorActionPreference = 'Stop'
$appName = 'CodexTitlebarMeter'
$installDirectory = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'Programs\CodexTitlebarMeter'))
$localRoot = [IO.Path]::GetFullPath($env:LOCALAPPDATA)
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CodexTitlebarMeter'

if (-not $installDirectory.StartsWith($localRoot, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to remove a directory outside LOCALAPPDATA.'
}

Get-CimInstance Win32_Process -Filter "Name = 'CodexTitlebarMeter.exe' OR Name = 'codex-titlebar-meter.exe'" |
    Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($installDirectory, [StringComparison]::OrdinalIgnoreCase) } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }

Remove-ItemProperty -Path $runKey -Name $appName -ErrorAction SilentlyContinue
Remove-Item -LiteralPath $uninstallKey -Recurse -Force -ErrorAction SilentlyContinue

if (Test-Path -LiteralPath $installDirectory) {
    Remove-Item -LiteralPath $installDirectory -Recurse -Force
}
if ($PurgeSettings) {
    $dataDirectory = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'ConfigCrate\CodexTitlebarMeter'))
    if ($dataDirectory.StartsWith($localRoot, [StringComparison]::OrdinalIgnoreCase) -and (Test-Path -LiteralPath $dataDirectory)) {
        Remove-Item -LiteralPath $dataDirectory -Recurse -Force
    }
}

Write-Host 'Codex Titlebar Meter uninstalled.' -ForegroundColor Green

