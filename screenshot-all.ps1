param(
    [string]$OutDir = "img\latest"
)

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$exe = Join-Path $scriptDir "target\debug\bitwarden-desktop-native.exe"
$ErrorActionPreference = "Continue"

New-Item -ItemType Directory -Force -Path (Join-Path $scriptDir $OutDir) | Out-Null

Write-Host "Building..." -ForegroundColor Cyan
Push-Location $scriptDir
cargo build 2>&1 | Out-Null
Pop-Location

Write-Host "Capturing login screen..." -ForegroundColor Cyan
$env:DEV_SCREENSHOT = Join-Path $scriptDir "$OutDir\Login.png"
$env:DEV_SCREEN = $null
& $exe 2>&1 | Out-Null

Write-Host "Capturing vault screen..." -ForegroundColor Cyan
$env:DEV_SCREENSHOT = Join-Path $scriptDir "$OutDir\Main.png"
$env:DEV_SCREEN = "vault"
& $exe 2>&1 | Out-Null

Write-Host "Done! Screenshots in $OutDir/" -ForegroundColor Green
