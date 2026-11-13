# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
#
# Builds the complete Windows release:
#   1. cargo build --release --locked   (MSVC target, engine)
#   2. PyInstaller onedir bundle        (GUI + engine + assets)
#   3. portable ZIP
#   4. Inno Setup installer (optional)
#   5. SHA256SUMS.txt
#
# Usage (repository root):
#   powershell -ExecutionPolicy Bypass -File windows\scripts\build.ps1
#   powershell ... -File build.ps1 -SkipInstaller -Version 1.0.0
#
# Tools are resolved explicitly; nothing is executed from the current
# directory and the PATH is only used through Get-Command with a clear error.

[CmdletBinding()]
param(
    [string]$Version,
    [string]$Cargo,
    [string]$Python,
    [string]$Iscc,
    [switch]$SkipInstaller,
    [switch]$SkipTests
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$ScriptRoot = $PSScriptRoot
$Root = Split-Path -Parent $ScriptRoot          # windows/
$RepoRoot = Split-Path -Parent $Root            # repository root
$EngineDir = Join-Path $Root "engine"
$GuiDir = Join-Path $Root "gui"
$DistDir = Join-Path $Root "dist"
$WorkDir = Join-Path $Root "build\pyinstaller"
$BundleDir = Join-Path $DistDir "MacroStat"

function Resolve-Tool {
    param([string]$Name, [string]$Explicit, [string[]]$Candidates)
    if ($Explicit) {
        if (-not (Test-Path -LiteralPath $Explicit)) {
            throw "$Name not found at '$Explicit'."
        }
        return (Resolve-Path -LiteralPath $Explicit).Path
    }
    foreach ($candidate in $Candidates) {
        if ($candidate -and (Test-Path -LiteralPath $candidate)) {
            return (Resolve-Path -LiteralPath $candidate).Path
        }
    }
    $command = Get-Command $Name -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    throw "$Name not found. Pass its full path explicitly (-Cargo/-Python/-Iscc)."
}

if (-not $Version) {
    $Version = (Get-Content -LiteralPath (Join-Path $Root "VERSION") -Raw).Trim()
}
if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Invalid version '$Version' (expected MAJOR.MINOR.PATCH)."
}

# The engine version and windows/VERSION must not drift apart.
$cargoToml = Get-Content -LiteralPath (Join-Path $EngineDir "Cargo.toml") -Raw
if ($cargoToml -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw "Could not read the version from engine/Cargo.toml."
}
if ($Matches[1] -ne $Version) {
    throw "engine/Cargo.toml version ($($Matches[1])) != windows/VERSION ($Version)."
}

$cargoPath = Resolve-Tool -Name "cargo" -Explicit $Cargo -Candidates @(
    (Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe")
)
$pythonPath = Resolve-Tool -Name "python" -Explicit $Python -Candidates @(
    (Join-Path $env:LOCALAPPDATA "Programs\Python\Python312\python.exe")
)

Push-Location $RepoRoot
try {
    Write-Host "==> Engine release build ($Version)" -ForegroundColor Cyan
    & $cargoPath build --release --locked --manifest-path (Join-Path $EngineDir "Cargo.toml")
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)." }

    $engineExe = Join-Path $EngineDir "target\release\macrostat-engine.exe"
    if (-not (Test-Path -LiteralPath $engineExe)) {
        throw "Engine binary missing after build: $engineExe"
    }

    if (-not $SkipTests) {
        Write-Host "==> GUI unit tests" -ForegroundColor Cyan
        $env:QT_QPA_PLATFORM = "offscreen"
        # Yalnızca birim testleri: E2E gerçek kanca/enjeksiyon çalıştırır ve
        # paketleme sırasında masaüstüne girdi göndermemelidir.
        & $pythonPath -m unittest discover -s (Join-Path $Root "tests") -p "test_gui*.py" -v
        if ($LASTEXITCODE -ne 0) { throw "GUI tests failed ($LASTEXITCODE)." }
    }

    Write-Host "==> PyInstaller bundle" -ForegroundColor Cyan
    & $pythonPath -m PyInstaller --version | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "PyInstaller is not installed for $pythonPath (pip install pyinstaller)."
    }
    if (Test-Path -LiteralPath $DistDir) { Remove-Item -Recurse -Force -LiteralPath $DistDir }
    if (Test-Path -LiteralPath $WorkDir) { Remove-Item -Recurse -Force -LiteralPath $WorkDir }
    & $pythonPath -m PyInstaller --clean --noconfirm `
        --distpath $DistDir --workpath $WorkDir `
        (Join-Path $GuiDir "MacroStat.spec")
    if ($LASTEXITCODE -ne 0) { throw "PyInstaller failed ($LASTEXITCODE)." }

    Copy-Item -LiteralPath (Join-Path $Root "LICENSE") -Destination $BundleDir
    Copy-Item -LiteralPath (Join-Path $Root "THIRD_PARTY.md") -Destination $BundleDir
    Copy-Item -LiteralPath (Join-Path $Root "VERSION") -Destination $BundleDir

    Write-Host "==> Portable ZIP" -ForegroundColor Cyan
    $zipPath = Join-Path $DistDir "MacroStat-$Version-portable.zip"
    Compress-Archive -Path (Join-Path $BundleDir "*") -DestinationPath $zipPath -CompressionLevel Optimal

    $artifacts = @($zipPath)

    if (-not $SkipInstaller) {
        Write-Host "==> Inno Setup installer" -ForegroundColor Cyan
        $isccPath = Resolve-Tool -Name "ISCC" -Explicit $Iscc -Candidates @(
            "C:\Program Files (x86)\Inno Setup 6\ISCC.exe",
            "C:\Program Files\Inno Setup 6\ISCC.exe"
        )
        & $isccPath "/DMyAppVersion=$Version" (Join-Path $Root "installer\MacroStat.iss")
        if ($LASTEXITCODE -ne 0) { throw "ISCC failed ($LASTEXITCODE)." }
        $setupPath = Join-Path $DistDir "MacroStat-Setup-$Version.exe"
        if (-not (Test-Path -LiteralPath $setupPath)) {
            throw "Installer output missing: $setupPath"
        }
        $artifacts += $setupPath
    }

    Write-Host "==> SHA256SUMS.txt" -ForegroundColor Cyan
    $lines = foreach ($file in $artifacts) {
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant()
        "{0}  {1}" -f $hash, (Split-Path -Leaf $file)
    }
    Set-Content -LiteralPath (Join-Path $DistDir "SHA256SUMS.txt") -Value $lines -Encoding ascii

    Write-Host "Build complete. Artifacts in $DistDir" -ForegroundColor Green
    Get-ChildItem -LiteralPath $DistDir | Select-Object Name, Length | Format-Table
}
finally {
    Pop-Location
}
