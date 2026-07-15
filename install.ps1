[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$appName = 'CodexTitlebarMeter'
$sourceDirectory = Split-Path -Parent $MyInvocation.MyCommand.Path
$sourceExe = Join-Path $sourceDirectory 'codex-titlebar-meter.exe'
$sourceUninstaller = Join-Path $sourceDirectory 'uninstall.ps1'
$installDirectory = Join-Path $env:LOCALAPPDATA 'Programs\CodexTitlebarMeter'
$targetExe = Join-Path $installDirectory 'CodexTitlebarMeter.exe'
$targetUninstaller = Join-Path $installDirectory 'uninstall.ps1'
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\CodexTitlebarMeter'

if (-not (Test-Path -LiteralPath $sourceExe -PathType Leaf)) {
    throw "codex-titlebar-meter.exe must be next to install.ps1"
}
if (-not (Test-Path -LiteralPath $sourceUninstaller -PathType Leaf)) {
    throw "uninstall.ps1 must be next to install.ps1"
}

Get-CimInstance Win32_Process -Filter "Name = 'CodexTitlebarMeter.exe' OR Name = 'codex-titlebar-meter.exe'" |
    Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($installDirectory, [StringComparison]::OrdinalIgnoreCase) } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }

New-Item -ItemType Directory -Path $installDirectory -Force | Out-Null
Copy-Item -LiteralPath $sourceExe -Destination $targetExe -Force
Copy-Item -LiteralPath $sourceUninstaller -Destination $targetUninstaller -Force

New-Item -Path $runKey -Force | Out-Null
Set-ItemProperty -Path $runKey -Name $appName -Value ('"{0}"' -f $targetExe)

New-Item -Path $uninstallKey -Force | Out-Null
Set-ItemProperty -Path $uninstallKey -Name DisplayName -Value 'Codex Titlebar Meter'
Set-ItemProperty -Path $uninstallKey -Name DisplayVersion -Value '0.1.0'
Set-ItemProperty -Path $uninstallKey -Name Publisher -Value 'ConfigCrate'
Set-ItemProperty -Path $uninstallKey -Name InstallLocation -Value $installDirectory
Set-ItemProperty -Path $uninstallKey -Name URLInfoAbout -Value 'https://configcrate.com/'
Set-ItemProperty -Path $uninstallKey -Name UninstallString -Value ('powershell.exe -NoProfile -ExecutionPolicy Bypass -File "{0}"' -f $targetUninstaller)
Set-ItemProperty -Path $uninstallKey -Name NoModify -Value 1 -Type DWord
Set-ItemProperty -Path $uninstallKey -Name NoRepair -Value 1 -Type DWord

Start-Process -FilePath $targetExe -WindowStyle Hidden
Write-Host 'Codex Titlebar Meter installed. It will follow Codex automatically.' -ForegroundColor Green

