# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
#
# Authenticode signing (optional). Runs only when a certificate is supplied.
#
#   PFX:   -PfxPath C:\secure\cert.pfx -PfxPassword (Get-Content ...)   [CI secret]
#   Store: -Thumbprint <40 hex chars>                                   [HSM/store]
#
# Secrets are never echoed, never written to disk and never committed.
# Use an RFC 3161 timestamp server so signatures outlive the certificate.

[CmdletBinding(DefaultParameterSetName = "Pfx")]
param(
    [string[]]$Files,
    [Parameter(ParameterSetName = "Pfx")][string]$PfxPath,
    [Parameter(ParameterSetName = "Pfx")][string]$PfxPassword,
    [Parameter(ParameterSetName = "Store")][string]$Thumbprint,
    [string]$TimestampUrl = "http://timestamp.digicert.com",
    [string]$Signtool
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
$DistDir = Join-Path $Root "dist"

if (-not $Files -or $Files.Count -eq 0) {
    $Files = @(
        (Join-Path $DistDir "MacroStat\MacroStat.exe"),
        (Join-Path $DistDir "MacroStat\_internal\macrostat-engine.exe")
    ) | Where-Object { Test-Path -LiteralPath $_ }
    $setup = Get-ChildItem -LiteralPath $DistDir -Filter "MacroStat-Setup-*.exe" -ErrorAction SilentlyContinue
    if ($setup) { $Files += $setup.FullName }
}

if (-not $Files -or $Files.Count -eq 0) {
    throw "Nothing to sign; build the artifacts first (scripts\build.ps1)."
}

function Resolve-Signtool {
    param([string]$Explicit)
    if ($Explicit) {
        if (-not (Test-Path -LiteralPath $Explicit)) { throw "signtool not found: $Explicit" }
        return (Resolve-Path -LiteralPath $Explicit).Path
    }
    $command = Get-Command "signtool.exe" -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }
    $kits = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin" -Directory -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending
    foreach ($kit in $kits) {
        $candidate = Join-Path $kit.FullName "x64\signtool.exe"
        if (Test-Path -LiteralPath $candidate) { return $candidate }
    }
    throw "signtool.exe not found; install the Windows SDK or pass -Signtool <path>."
}

$signtoolPath = Resolve-Signtool -Explicit $Signtool

$common = @("sign", "/fd", "SHA256", "/v")
# Zaman damgası boş bırakılırsa imza sertifikanın geçerlilik süresiyle sınırlıdır.
if ($TimestampUrl) {
    $common += @("/tr", $TimestampUrl, "/td", "SHA256")
}
if ($PSCmdlet.ParameterSetName -eq "Pfx") {
    if (-not $PfxPath -or -not $PfxPassword) {
        throw "PFX signing needs -PfxPath and -PfxPassword (use a CI secret)."
    }
    if (-not (Test-Path -LiteralPath $PfxPath)) { throw "PFX not found: $PfxPath" }
    $common += @("/f", $PfxPath, "/p", $PfxPassword)
} else {
    if (-not $Thumbprint) {
        throw 'Store signing needs -Thumbprint (certificate in CurrentUser\My).'
    }
    $common += @("/sha1", $Thumbprint)
}

foreach ($file in $Files) {
    if (-not (Test-Path -LiteralPath $file)) { throw "Missing artifact: $file" }
    Write-Host "Signing $(Split-Path -Leaf $file)" -ForegroundColor Cyan
    & $signtoolPath @common $file
    if ($LASTEXITCODE -ne 0) { throw "signtool failed for $file ($LASTEXITCODE)." }
}

Write-Host "Signing complete." -ForegroundColor Green
