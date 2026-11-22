<#
.SYNOPSIS
    Stamps the release version from a git tag into the files that need it.

.DESCRIPTION
    Call from CI on a vX.Y.Z tag. The tag is the only source of truth: Cargo.toml,
    nsh.json and the Inno Setup script are patched in the build checkout, so a
    release needs no manual version-bump commit.
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Version
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$version = $Version -replace '^v', ''
if ($version -notmatch '^\d+\.\d+\.\d+$') {
    throw "Invalid release version '$Version' (expected vX.Y.Z)"
}

$root = Split-Path -Parent $PSScriptRoot

# Write UTF-8 without a BOM and keep the file's existing line endings intact
# (Get-Content -Raw returns the original newlines verbatim). Windows PowerShell
# 5.1's Set-Content would add a BOM, which can break TOML/JSON parsers.
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
function Write-NoBom([string]$Path, [string]$Content) {
    [System.IO.File]::WriteAllText($Path, $Content, $utf8NoBom)
}

# nsh.json: app version (the updater compares this to latest.json).
$appJsonPath = Join-Path $root 'nsh.json'
$appJson = Get-Content -LiteralPath $appJsonPath -Raw
$appJson = [regex]::Replace($appJson, '(?m)^  "version": "[^"]+"', "  `"version`": `"$version`"", 1)
if ($appJson -notmatch "  `"version`": `"$version`"") { throw "Failed to stamp nsh.json" }
Write-NoBom $appJsonPath $appJson

# Cargo.toml: compiled version used by `env!("CARGO_PKG_VERSION")`.
$cargoPath = Join-Path $root 'Cargo.toml'
$cargo = Get-Content -LiteralPath $cargoPath -Raw
$cargo = [regex]::Replace($cargo, '(?m)^version = "[^"]+"', "version = `"$version`"", 1)
if ($cargo -notmatch "version = `"$version`"") { throw "Failed to stamp Cargo.toml" }
Write-NoBom $cargoPath $cargo

# Cargo.lock: keep the workspace package entry in sync (also safe with --locked).
$lockPath = Join-Path $root 'Cargo.lock'
$lock = Get-Content -LiteralPath $lockPath -Raw
$lock = [regex]::Replace($lock, '(?m)(name = "sudocooler"\r?\nversion = ")[^"]+', "`${1}$version", 1)
if ($lock -notmatch "name = `"sudocooler`"\r?`nversion = `"$version`"") { throw "Failed to stamp Cargo.lock" }
Write-NoBom $lockPath $lock

# Inno Setup: installer version.
$issPath = Join-Path $root 'installer\sudocooler.iss'
$iss = Get-Content -LiteralPath $issPath -Raw
$iss = [regex]::Replace($iss, '(?m)^#define AppVersion "[^"]+"', "#define AppVersion `"$version`"", 1)
if ($iss -notmatch "#define AppVersion `"$version`"") { throw "Failed to stamp sudocooler.iss" }
Write-NoBom $issPath $iss

Write-Host "Stamped version $version into nsh.json, Cargo.toml and sudocooler.iss"
