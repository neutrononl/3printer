param([string]$Target)
$ErrorActionPreference = "Stop"
$RootDir = (Resolve-Path (Join-Path $PSScriptRoot ".."))
$AppDir = Join-Path $RootDir "dist\3printer-windows"
$BuildDir = if ($Target) { "target\$Target\release" } else { "target\release" }

Set-Location $RootDir
Write-Host "Building release binary..."
if ($Target) {
	cargo build --release --target $Target
} else {
	cargo build --release
}
if ($LASTEXITCODE -ne 0) { throw "Cargo build failed" }

if (Test-Path $AppDir) { Remove-Item -Recurse -Force $AppDir }
New-Item -ItemType Directory -Force -Path $AppDir | Out-Null
Copy-Item (Join-Path $RootDir "$BuildDir\3printer.exe") (Join-Path $AppDir "3printer.exe") -Force

Write-Host "Created $AppDir\3printer.exe"
Write-Host "Double-click the EXE to launch without a terminal."
