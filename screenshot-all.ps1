param(
    [string]$OutDir = "img\latest",
    [string]$WindowTitle = "Bitwarden [Next]"
)

$ErrorActionPreference = "Continue"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$screenshotScript = Join-Path $scriptDir "screenshot.ps1"

# Ensure output directory exists
New-Item -ItemType Directory -Force -Path (Join-Path $scriptDir $OutDir) | Out-Null

# Build
Write-Host "Building..." -ForegroundColor Cyan
Push-Location $scriptDir
cargo build 2>&1 | Out-Null
Pop-Location

# Screenshot login screen
Write-Host "Launching app (login screen)..." -ForegroundColor Cyan
$proc = Start-Process -FilePath (Join-Path $scriptDir "target\debug\bitwarden-desktop-native.exe") -PassThru
Start-Sleep -Seconds 5
& $screenshotScript -OutputPath (Join-Path $scriptDir "$OutDir\Login.png") -WindowTitle $WindowTitle
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
Start-Sleep -Seconds 1

# Screenshot vault screen
Write-Host "Launching app (vault screen)..." -ForegroundColor Cyan
$env:DEV_SCREEN = "vault"
$proc = Start-Process -FilePath (Join-Path $scriptDir "target\debug\bitwarden-desktop-native.exe") -PassThru
Start-Sleep -Seconds 5
& $screenshotScript -OutputPath (Join-Path $scriptDir "$OutDir\Main.png") -WindowTitle $WindowTitle
Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
$env:DEV_SCREEN = $null

Write-Host "Done! Screenshots in $OutDir/" -ForegroundColor Green
