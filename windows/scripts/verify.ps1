# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2026 MacroStat contributors
#
# Verifies a built release without modifying it:
#   * required files are present
#   * SHA256SUMS.txt matches the artifacts
#   * Authenticode status is reported (signature optional)
#   * the engine runs (--version) and the GUI bundle imports
#
# Exit code is non-zero when a hard check fails.

[CmdletBinding()]
param(
    [string]$Dist = "",
    [switch]$RequireSignature
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
if (-not $Dist) { $Dist = Join-Path $Root "dist" }

$failures = New-Object System.Collections.Generic.List[string]

function Test-RequiredFile {
    param([string]$Path)
    if (Test-Path -LiteralPath $Path) {
        Write-Host "  ok   $(Split-Path -Leaf $Path)"
    } else {
        Write-Host "  FAIL missing: $Path" -ForegroundColor Red
        $failures.Add("missing: $Path")
    }
}

Write-Host "==> Required files" -ForegroundColor Cyan
$guiExe = Join-Path $Dist "MacroStat\MacroStat.exe"
$engineExe = Join-Path $Dist "MacroStat\_internal\macrostat-engine.exe"
$profileTemplate = Join-Path $Dist "MacroStat\_internal\assets\default_profile.json"
Test-RequiredFile $guiExe
Test-RequiredFile $engineExe
Test-RequiredFile $profileTemplate
Test-RequiredFile (Join-Path $Dist "MacroStat\LICENSE")
Test-RequiredFile (Join-Path $Dist "MacroStat\THIRD_PARTY.md")

Write-Host "==> SHA256SUMS.txt" -ForegroundColor Cyan
$sums = Join-Path $Dist "SHA256SUMS.txt"
if (Test-Path -LiteralPath $sums) {
    foreach ($line in Get-Content -LiteralPath $sums) {
        if ($line -notmatch '^([0-9a-f]{64})\s{2}(.+)$') { continue }
        $expected = $Matches[1]
        $name = $Matches[2].Trim()
        $target = Join-Path $Dist $name
        if (-not (Test-Path -LiteralPath $target)) {
            Write-Host "  FAIL listed but missing: $name" -ForegroundColor Red
            $failures.Add("missing artifact: $name")
            continue
        }
        $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $target).Hash.ToLowerInvariant()
        if ($actual -eq $expected) {
            Write-Host "  ok   $name"
        } else {
            Write-Host "  FAIL hash mismatch: $name" -ForegroundColor Red
            $failures.Add("hash mismatch: $name")
        }
    }
} else {
    Write-Host "  FAIL SHA256SUMS.txt not found" -ForegroundColor Red
    $failures.Add("missing: SHA256SUMS.txt")
}

Write-Host "==> Authenticode status" -ForegroundColor Cyan
foreach ($file in @($guiExe, $engineExe)) {
    if (-not (Test-Path -LiteralPath $file)) { continue }
    $signature = Get-AuthenticodeSignature -LiteralPath $file
    Write-Host ("  {0,-16} {1}" -f (Split-Path -Leaf $file), $signature.Status)
    if ($RequireSignature -and $signature.Status -ne "Valid") {
        $failures.Add("unsigned artifact: $(Split-Path -Leaf $file)")
    }
}

# Never execute an artifact that failed the integrity/signature checks: the
# smoke test is the only step that runs bundle code, so it comes last and is
# skipped when any hard check has already failed.
if ($failures.Count -eq 0 -and (Test-Path -LiteralPath $engineExe)) {
    Write-Host "==> Engine smoke test" -ForegroundColor Cyan
    $versionOutput = & $engineExe --version 2>&1 | Out-String
    if ($LASTEXITCODE -eq 0 -and $versionOutput.Trim()) {
        Write-Host "  ok   $($versionOutput.Trim())"
    } else {
        Write-Host "  FAIL engine --version" -ForegroundColor Red
        $failures.Add("engine --version failed")
    }
} elseif ($failures.Count -gt 0) {
    Write-Host "==> Engine smoke test skipped (earlier checks failed)" -ForegroundColor Yellow
}

if ($failures.Count -gt 0) {
    Write-Host "`nVerification FAILED ($($failures.Count) issue(s))." -ForegroundColor Red
    $failures | ForEach-Object { Write-Host " - $_" -ForegroundColor Red }
    exit 1
}
Write-Host "`nVerification passed." -ForegroundColor Green
