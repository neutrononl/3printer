$ErrorActionPreference = "Stop"
$RootDir = (Resolve-Path (Join-Path $PSScriptRoot ".."))
$AppDir = Join-Path $RootDir "dist\3printer-windows"

Set-Location $RootDir
Write-Host "Building release binary..."
cargo build --release

New-Item -ItemType Directory -Force -Path $AppDir | Out-Null
Copy-Item (Join-Path $RootDir "target\release\3printer.exe") (Join-Path $AppDir "3printer.exe") -Force

Write-Host "Created $AppDir\3printer.exe"
Write-Host "Double-click the EXE to launch without a terminal."
