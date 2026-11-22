<#
.SYNOPSIS
    Builds and stages a portable Windows release of sudocooler.

.DESCRIPTION
    1. Builds the optimized NSH app bundle (nsh build --release) and packages the
       Windows runner (nsh package --target windows). NSH 0.5 has no --release
       flag on 'nsh package'; the release build is done by 'nsh build --release'
       plus the package command's own release build of the runner.
    2. Builds the daemon from the git submodule (sudokooler/Cargo.toml) in release mode.
    3. Downloads the signed PawnIO installer and modules (AMDFamily17.bin for AMD,
       IntelMSR.bin for Intel) into the BUILD OUTPUT only - never into git.
    4. Stages everything plus docs and the install/uninstall scripts into
       dist\sudocooler-portable\ and zips it to dist\sudocooler-portable.zip.

    No administrator rights are required. Downloaded assets live only in dist/
    and a temporary directory; both are cleaned up / gitignored.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\installer\build-release.ps1

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\installer\build-release.ps1 -SkipPawnIO
    Build without downloading PawnIO assets (offline use; CPU temp/power may be 0).

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\installer\build-release.ps1 -AppPackageDir C:\pkg\sudocooler -DaemonBinary C:\daemon\sudokooler.exe
    Reuse already-built binaries.
#>

[CmdletBinding()]
param(
    [string]$OutDir = 'dist',
    [switch]$SkipPawnIO,
    [string]$AppPackageDir = '',
    [string]$DaemonBinary = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$AppExeName = 'sudocooler.exe'
$DaemonExeName = 'sudokooler.exe'

function Write-Step {
    param([Parameter(Mandatory)][string]$Message)
    Write-Host ''
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Write-Detail {
    param([Parameter(Mandatory)][string]$Message)
    Write-Host "    $Message"
}

function Add-ToolLocationToPath {
    # Make the documented NSH / Cargo install locations available to this process.
    $candidates = @(
        (Join-Path $env:USERPROFILE '.cargo\bin'),
        (Join-Path $env:LOCALAPPDATA 'nsh\bin')
    )
    foreach ($dir in $candidates) {
        if ([string]::IsNullOrEmpty($dir)) { continue }
        if (Test-Path -LiteralPath $dir) {
            $parts = $env:Path -split ';'
            if ($parts -notcontains $dir) {
                $env:Path = "$dir;$env:Path"
            }
        }
    }
}

function Invoke-NativeChecked {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @(),
        [Parameter(Mandatory)][string]$Description
    )
    Write-Detail "$FilePath $($Arguments -join ' ')"
    # Native tools such as nsh write progress to stderr. In PowerShell 5.1 with
    # ErrorActionPreference=Stop that becomes a terminating NativeCommandError,
    # so temporarily relax it and rely on $LASTEXITCODE instead.
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $FilePath @Arguments 2>&1 | ForEach-Object { Write-Host "    $_" }
        $code = $LASTEXITCODE
    }
    finally {
        $ErrorActionPreference = $previous
    }
    if ($code -ne 0) {
        throw "$Description failed with exit code $code."
    }
}

function Get-GitHubReleaseAsset {
    param(
        [Parameter(Mandatory)][string]$Repo,
        [Parameter(Mandatory)][string]$Pattern
    )
    $headers = @{ 'User-Agent' = 'sudocooler-release-bundler' }
    if ($env:GITHUB_TOKEN) {
        $headers['Authorization'] = "Bearer $($env:GITHUB_TOKEN)"
    }
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers $headers -TimeoutSec 60
    $asset = $release.assets | Where-Object { $_.name -like $Pattern } | Select-Object -First 1
    if (-not $asset) {
        throw "Could not find an asset matching '$Pattern' in the latest release of $Repo."
    }
    return [pscustomobject]@{
        Tag  = [string]$release.tag_name
        Name = [string]$asset.name
        Url  = [string]$asset.browser_download_url
    }
}

function Save-GitHubAsset {
    param(
        [Parameter(Mandatory)][string]$Url,
        [Parameter(Mandatory)][string]$Destination
    )
    $previous = $ProgressPreference
    $ProgressPreference = 'SilentlyContinue'
    try {
        Invoke-WebRequest -Uri $Url -OutFile $Destination -UseBasicParsing -TimeoutSec 300
    }
    finally {
        $ProgressPreference = $previous
    }
    if (-not (Test-Path -LiteralPath $Destination)) {
        throw "Download to '$Destination' did not produce a file."
    }
}

# ---------------------------------------------------------------------------
# Resolve paths
# ---------------------------------------------------------------------------
$repoRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not [System.IO.Path]::IsPathRooted($OutDir)) {
    $OutDir = Join-Path $repoRoot $OutDir
}
$OutDir = [System.IO.Path]::GetFullPath($OutDir)
$stageDir = Join-Path $OutDir 'sudocooler-portable'
$zipPath = Join-Path $OutDir 'sudocooler-portable.zip'
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ('sudocooler-build-' + [guid]::NewGuid().ToString('N'))

Write-Host "Repository : $repoRoot"
Write-Host "Output     : $OutDir"

Add-ToolLocationToPath

$hasNsh = Get-Command -Name 'nsh' -ErrorAction SilentlyContinue
$hasCargo = Get-Command -Name 'cargo' -ErrorAction SilentlyContinue
if (-not $hasNsh) { throw "The 'nsh' CLI was not found. Install NSH 0.5 (PATH hint: `$env:LOCALAPPDATA\nsh\bin)." }
if (-not $hasCargo) { throw "The 'cargo' toolchain was not found. Install Rust (PATH hint: `$env:USERPROFILE\.cargo\bin)." }

New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

try {
    if (Test-Path -LiteralPath $stageDir) { Remove-Item -LiteralPath $stageDir -Recurse -Force }
    if (Test-Path -LiteralPath $zipPath) { Remove-Item -LiteralPath $zipPath -Force }
    New-Item -ItemType Directory -Path $stageDir -Force | Out-Null

    # -----------------------------------------------------------------------
    # 1. App: NSH Windows package (runner exe + bundle + assets)
    # -----------------------------------------------------------------------
    if ([string]::IsNullOrEmpty($AppPackageDir)) {
        Write-Step '[1/4] Building optimized NSH app (nsh build --release) ...'
        Push-Location $repoRoot
        try {
            Invoke-NativeChecked -FilePath 'nsh' -Arguments @('build', '--release') -Description 'nsh build --release'
        }
        finally { Pop-Location }

        Write-Step '[2/4] Packaging the Windows runner (nsh package --target windows) ...'
        $packageOut = Join-Path $tempDir 'nsh-package'
        New-Item -ItemType Directory -Path $packageOut -Force | Out-Null
        Push-Location $repoRoot
        try {
            Invoke-NativeChecked -FilePath 'nsh' -Arguments @('package', '--target', 'windows', '--out', $packageOut) -Description 'nsh package --target windows'
        }
        finally { Pop-Location }
        $AppPackageDir = Join-Path $packageOut 'sudocooler'

        # Keep the NSH package archive under the name nsh update-manifest needs.
        # The daemon is added to it after it is built below (the nsh package
        # itself only contains the app runner), so the self-updater ships the
        # daemon too.
        $appVersion = (Get-Content -LiteralPath (Join-Path $repoRoot 'nsh.json') -Raw | ConvertFrom-Json).version
        $updateZip = Join-Path $OutDir "sudocooler-$appVersion-windows-x86_64.zip"
    }
    else {
        Write-Step '[1/4] Using the supplied app package directory (skipping nsh build/package).'
    }

    if (-not [string]::IsNullOrEmpty($AppPackageDir)) {
        $AppPackageDir = [System.IO.Path]::GetFullPath($AppPackageDir)
    }
    $appExeSource = Join-Path $AppPackageDir $AppExeName
    if (-not (Test-Path -LiteralPath $appExeSource)) {
        throw "The NSH app package '$AppPackageDir' does not contain '$AppExeName'."
    }
    Write-Detail "Staging app package from $AppPackageDir"
    Get-ChildItem -LiteralPath $AppPackageDir -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $stageDir -Recurse -Force
    }

    # -----------------------------------------------------------------------
    # 2. Daemon: release build of the git submodule
    # -----------------------------------------------------------------------
    Write-Step '[3/4] Building the daemon (sudokooler submodule, release) ...'
    if ([string]::IsNullOrEmpty($DaemonBinary)) {
        $manifest = Join-Path $repoRoot 'sudokooler\Cargo.toml'
        if (-not (Test-Path -LiteralPath $manifest)) {
            throw "The daemon submodule is missing at '$manifest'. Run: git submodule update --init --recursive"
        }
        Invoke-NativeChecked -FilePath 'cargo' -Arguments @('build', '--release', '--manifest-path', $manifest) -Description 'cargo build --release for the daemon'
        $DaemonBinary = Join-Path $repoRoot 'sudokooler\target\release\sudokooler.exe'
    }
    else {
        $DaemonBinary = [System.IO.Path]::GetFullPath($DaemonBinary)
    }
    if (-not (Test-Path -LiteralPath $DaemonBinary)) {
        throw "The daemon binary was not found at '$DaemonBinary'."
    }
    Copy-Item -LiteralPath $DaemonBinary -Destination (Join-Path $stageDir $DaemonExeName) -Force

    # The self-update archive is the NSH app package, which does not include the
    # daemon. Stage sudokooler.exe beside sudocooler.exe inside it so an updated
    # install can still read telemetry (cli.rs looks for the daemon next to the app).
    # The archive is written after PawnIO modules are staged below.
    if (-not [string]::IsNullOrEmpty($updateZip)) {
        $packagedAppDir = Join-Path $packageOut 'sudocooler'
        $packagedDaemon = Join-Path $packagedAppDir $DaemonExeName
        Copy-Item -LiteralPath $DaemonBinary -Destination $packagedDaemon -Force
    }

    # -----------------------------------------------------------------------
    # 3. PawnIO assets (build output / temp only, never committed)
    # -----------------------------------------------------------------------
    $setupTag = 'not bundled'
    $modulesTag = 'not bundled'
    $pawnioDir = Join-Path $stageDir 'pawnio'
    $installerAssetsDir = Join-Path $stageDir 'installer'
    New-Item -ItemType Directory -Path $pawnioDir -Force | Out-Null
    New-Item -ItemType Directory -Path $installerAssetsDir -Force | Out-Null

    if ($SkipPawnIO) {
        Write-Warning 'Skipping PawnIO downloads (-SkipPawnIO). CPU temperature/package power will require a manual PawnIO setup.'
    }
    else {
        Write-Step 'Downloading PawnIO installer and signed modules from GitHub Releases ...'
        $setupAsset = Get-GitHubReleaseAsset -Repo 'namazso/PawnIO.Setup' -Pattern 'PawnIO_setup*.exe'
        $setupDst = Join-Path $installerAssetsDir 'PawnIO_setup.exe'
        Save-GitHubAsset -Url $setupAsset.Url -Destination $setupDst
        $setupTag = $setupAsset.Tag
        Write-Detail "PawnIO_setup.exe ($($setupAsset.Tag))"

        $modulesAsset = Get-GitHubReleaseAsset -Repo 'namazso/PawnIO.Modules' -Pattern 'release_*.zip'
        $modulesZip = Join-Path $tempDir $modulesAsset.Name
        $modulesExtract = Join-Path $tempDir 'modules'
        Save-GitHubAsset -Url $modulesAsset.Url -Destination $modulesZip
        Expand-Archive -LiteralPath $modulesZip -DestinationPath $modulesExtract -Force
        $modulesTag = $modulesAsset.Tag

        foreach ($moduleName in @('AMDFamily17.bin', 'IntelMSR.bin')) {
            $found = Get-ChildItem -LiteralPath $modulesExtract -Filter $moduleName -Recurse -File | Select-Object -First 1
            if (-not $found) {
                Write-Warning "'$moduleName' was not found in $($modulesAsset.Name); skipping it."
                continue
            }
            Copy-Item -LiteralPath $found.FullName -Destination (Join-Path $pawnioDir $moduleName) -Force
            Write-Detail "$moduleName ($($modulesAsset.Tag))"
        }

        # The daemon loads its module from <exe_dir>\pawnio\. Copy the modules
        # beside the locally-built daemon too, so running the built daemon
        # directly (elevated) reads real telemetry instead of reporting 0.
        $builtDaemonDir = Split-Path -Parent $DaemonBinary
        $builtDaemonPawnio = Join-Path $builtDaemonDir 'pawnio'
        New-Item -ItemType Directory -Path $builtDaemonPawnio -Force | Out-Null
        foreach ($moduleName in @('AMDFamily17.bin', 'IntelMSR.bin')) {
            $src = Join-Path $pawnioDir $moduleName
            if (Test-Path -LiteralPath $src) {
                Copy-Item -LiteralPath $src -Destination (Join-Path $builtDaemonPawnio $moduleName) -Force
            }
        }
    }

    # Write the self-update archive now that the daemon and its PawnIO modules
    # are staged in the app package dir. The daemon loads its module from
    # <exe_dir>\pawnio\, so it must be inside this archive.
    if (-not [string]::IsNullOrEmpty($updateZip)) {
        $packagedAppDir = Join-Path $packageOut 'sudocooler'
        $packagedPawnio = Join-Path $packagedAppDir 'pawnio'
        New-Item -ItemType Directory -Path $packagedPawnio -Force | Out-Null
        foreach ($moduleName in @('AMDFamily17.bin', 'IntelMSR.bin')) {
            $src = Join-Path $pawnioDir $moduleName
            if (Test-Path -LiteralPath $src) {
                Copy-Item -LiteralPath $src -Destination (Join-Path $packagedPawnio $moduleName) -Force
            }
        }
        # Also carry the PawnIO installer so Settings -> Set up PawnIO keeps
        # working after a self-update.
        $packagedInstaller = Join-Path $packagedAppDir 'installer'
        $setupSource = Join-Path $installerAssetsDir 'PawnIO_setup.exe'
        if (Test-Path -LiteralPath $setupSource) {
            New-Item -ItemType Directory -Path $packagedInstaller -Force | Out-Null
            Copy-Item -LiteralPath $setupSource -Destination (Join-Path $packagedInstaller 'PawnIO_setup.exe') -Force
        }
        Compress-Archive -LiteralPath $packagedAppDir -DestinationPath $updateZip -Force
        Write-Detail "Update archive (with daemon + PawnIO modules): $updateZip"
    }

    # -----------------------------------------------------------------------
    # 4. Docs, helper scripts, licenses and the final zip
    # -----------------------------------------------------------------------
    Write-Step '[4/4] Staging docs and creating the portable zip ...'
    foreach ($name in @('install.ps1', 'uninstall.ps1', 'launch-elevated.vbs')) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $stageDir $name) -Force
    }
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'README.md') -Destination (Join-Path $stageDir 'README.md') -Force
    Copy-Item -LiteralPath (Join-Path $repoRoot 'README.md') -Destination (Join-Path $stageDir 'APP_README.md') -Force

    $licenses = @"
sudocooler Windows Portable Release - Third-Party Licenses and Sources
======================================================================

1. sudocooler app (sudocooler.exe and nsh\) - GPL-3.0
   Source: https://github.com/msartore/sudocooler

2. sudokooler daemon (sudokooler.exe) - MIT
   Source: https://github.com/BGW-10/sudokooler

3. PawnIO driver installer (installer\PawnIO_setup.exe, version $setupTag) - GPL-2.0
   Source: https://github.com/namazso/PawnIO

4. PawnIO signed modules (pawnio\*.bin, version $modulesTag) - LGPL-2.1-or-later
   Source: https://github.com/namazso/PawnIO.Modules

The PawnIO binaries are downloaded at build time into build output only and are
never stored in this project's git repository.
"@
    Set-Content -LiteralPath (Join-Path $stageDir 'THIRD_PARTY_LICENSES.txt') -Value $licenses -Encoding UTF8

    Compress-Archive -Path (Join-Path $stageDir '*') -DestinationPath $zipPath -Force

    # -----------------------------------------------------------------------
    # 5. Windows setup.exe (Inno Setup), when the compiler is available
    # -----------------------------------------------------------------------
    $setupPath = Join-Path $OutDir 'sudocooler-setup.exe'
    $isccCandidates = @(
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe')
    )
    $iscc = $isccCandidates | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
    if ($iscc) {
        Write-Step '[5/5] Building the Windows setup executable ...'
        Invoke-NativeChecked -FilePath $iscc -Arguments @(
            ("/D`"SourceDir=$stageDir`""),
            ("/D`"OutDir=$OutDir`""),
            (Join-Path $PSScriptRoot 'sudocooler.iss')
        ) -Description 'Inno Setup compiler'
    }
    else {
        Write-Warning 'Inno Setup was not found; sudocooler-setup.exe was not built. Install it with: choco install innosetup'
    }

    Write-Host ''
    Write-Host 'Build complete.' -ForegroundColor Green
    Write-Host "  Portable folder : $stageDir"
    Write-Host "  Portable zip    : $zipPath"
    if (Test-Path -LiteralPath $setupPath) {
        Write-Host "  Setup executable: $setupPath"
    }
    Write-Host '  Next: run sudocooler-setup.exe, or extract the zip and run install.ps1.'
}
finally {
    if (Test-Path -LiteralPath $tempDir) {
        Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
