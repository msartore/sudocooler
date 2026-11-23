<#
.SYNOPSIS
    Installs the staged portable sudocooler release for the current user.

.DESCRIPTION
    Run this script from an extracted sudocooler-portable folder (it expects
    sudocooler.exe, sudokooler.exe, nsh\ and pawnio\ next to it).

    * Copies the app + daemon into $env:LOCALAPPDATA\Programs\sudocooler (no admin).
    * Creates a Start Menu shortcut.
    * Removes a Scheduled Task an older build registered at logon (start at
      login is now a per-user entry the app manages, unelevated).
    * Sets the per-user SUDOKOOLER_BIN variable so the app can always find the
      daemon when it is started from another working directory.
    * Installs PawnIO ONLY if it is missing AND -InstallPawnIO is supplied.
      The bundled installer (installer\PawnIO_setup.exe) is used when present;
      otherwise it is downloaded to a temporary folder. The driver installer may
      show a UAC/elevation prompt.

    Supports -WhatIf and -Confirm (all changes go through ShouldProcess) and is
    safe to re-run.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\install.ps1

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\install.ps1 -InstallPawnIO

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\install.ps1 -WhatIf
#>

[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'Medium')]
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\sudocooler'),
    [switch]$InstallPawnIO,
    [switch]$SkipTask
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$TaskName = 'sudocooler'
$AppExeName = 'sudocooler.exe'
$DaemonExeName = 'sudokooler.exe'

function Write-Step {
    param([Parameter(Mandatory)][string]$Message)
    Write-Host ''
    Write-Host "==> $Message" -ForegroundColor Cyan
}

function Invoke-NativeChecked {
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @(),
        [Parameter(Mandatory)][string]$Description,
        [switch]$AllowFailure
    )
    Write-Host "    $FilePath $($Arguments -join ' ')"
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
        if ($AllowFailure) {
            Write-Warning "$Description exited with code $code (continuing)."
        }
        else {
            throw "$Description failed with exit code $code."
        }
    }
}

function Test-PawnIoInstalled {
    $keys = @(
        'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO',
        'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO'
    )
    foreach ($key in $keys) {
        if (Test-Path -LiteralPath $key) { return $true }
    }
    return (Test-Path -LiteralPath 'C:\Program Files\PawnIO')
}

function Get-GitHubReleaseAsset {
    param(
        [Parameter(Mandatory)][string]$Repo,
        [Parameter(Mandatory)][string]$Pattern
    )
    $headers = @{ 'User-Agent' = 'sudocooler-installer' }
    if ($env:GITHUB_TOKEN) {
        $headers['Authorization'] = "Bearer $($env:GITHUB_TOKEN)"
    }
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers $headers -TimeoutSec 60
    $asset = $release.assets | Where-Object { $_.name -like $Pattern } | Select-Object -First 1
    if (-not $asset) {
        throw "Could not find an asset matching '$Pattern' in the latest release of $Repo."
    }
    return [string]$asset.browser_download_url
}

# ---------------------------------------------------------------------------
# Validate the staged sources
# ---------------------------------------------------------------------------
$sourceDir = $PSScriptRoot
if (-not [System.IO.Path]::IsPathRooted($InstallDir)) {
    $InstallDir = Join-Path $env:LOCALAPPDATA $InstallDir
}
$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)

$appExeSource = Join-Path $sourceDir $AppExeName
$daemonSource = Join-Path $sourceDir $DaemonExeName
if (-not (Test-Path -LiteralPath $appExeSource)) {
    throw "'$appExeSource' is missing. Extract sudocooler-portable.zip and run install.ps1 from the extracted folder."
}
if (-not (Test-Path -LiteralPath $daemonSource)) {
    throw "'$daemonSource' is missing. Extract the complete portable package and try again."
}
if (-not (Test-Path -LiteralPath (Join-Path $sourceDir 'nsh'))) {
    Write-Warning "The 'nsh' bundle folder is missing from '$sourceDir'; the app may not start."
}

Write-Host "Source    : $sourceDir"
Write-Host "Install to: $InstallDir"

$appExe = Join-Path $InstallDir $AppExeName
$daemonExe = Join-Path $InstallDir $DaemonExeName

# ---------------------------------------------------------------------------
# 1. PawnIO: check first; install only when missing AND explicitly allowed
# ---------------------------------------------------------------------------
Write-Step '[1/5] Checking PawnIO driver ...'
if (Test-PawnIoInstalled) {
    Write-Host '    PawnIO is already installed - skipping driver setup.'
}
elseif ($InstallPawnIO) {
    $driver = Join-Path $sourceDir 'installer\PawnIO_setup.exe'
    $tempDir = ''
    if ($PSCmdlet.ShouldProcess('PawnIO', 'Install the signed PawnIO kernel driver')) {
        if (-not (Test-Path -LiteralPath $driver)) {
            Write-Host '    Bundled PawnIO installer not found; downloading to a temporary folder ...'
            $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ('sudocooler-pawnio-' + [guid]::NewGuid().ToString('N'))
            New-Item -ItemType Directory -Path $tempDir -Force | Out-Null
            $url = Get-GitHubReleaseAsset -Repo 'namazso/PawnIO.Setup' -Pattern 'PawnIO_setup*.exe'
            $driver = Join-Path $tempDir 'PawnIO_setup.exe'
            $previous = $ProgressPreference
            $ProgressPreference = 'SilentlyContinue'
            try {
                Invoke-WebRequest -Uri $url -OutFile $driver -UseBasicParsing -TimeoutSec 300
            }
            finally {
                $ProgressPreference = $previous
            }
        }
        Write-Host '    Launching the PawnIO installer (a UAC prompt may appear) ...'
        $proc = Start-Process -FilePath $driver -Wait -PassThru
        if ($proc.ExitCode -ne 0) {
            throw "PawnIO installer exited with code $($proc.ExitCode). CPU temperature/power may not work."
        }
        Write-Host '    PawnIO driver installed.'
    }
    else {
        Write-Host '    Skipped (WhatIf/Confirm); no download or driver install performed.'
    }
    if (-not [string]::IsNullOrEmpty($tempDir) -and (Test-Path -LiteralPath $tempDir)) {
        Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
else {
    Write-Warning 'PawnIO is NOT installed, so CPU temperature and package power will read 0 (HWiNFO64 shared memory can also supply them). Re-run with -InstallPawnIO to install the signed driver.'
}

# ---------------------------------------------------------------------------
# 2. Stop only the copies that live in the target install folder
# ---------------------------------------------------------------------------
Write-Step '[2/5] Stopping any running installed instance ...'
$running = @(Get-Process -ErrorAction SilentlyContinue |
    Where-Object {
        $_.ProcessName -in @('sudocooler', 'sudokooler') -and
        $_.Path -and
        $_.Path.StartsWith($InstallDir, [System.StringComparison]::OrdinalIgnoreCase)
    })
foreach ($proc in $running) {
    if ($PSCmdlet.ShouldProcess("$($proc.ProcessName) (PID $($proc.Id))", 'Stop process before replacing files')) {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
}
if ($running.Count -eq 0) { Write-Host '    No running installed instance.' }

# ---------------------------------------------------------------------------
# 3. Copy the portable package into the install folder
# ---------------------------------------------------------------------------
Write-Step '[3/5] Installing files ...'
if ($PSCmdlet.ShouldProcess($InstallDir, 'Copy the portable app, daemon, modules and docs')) {
    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Get-ChildItem -LiteralPath $sourceDir -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $InstallDir -Recurse -Force
    }
    Write-Host "    Installed to $InstallDir"
}
else {
    Write-Host '    Skipped (WhatIf/Confirm).'
}

# The app looks here first for the daemon. A launcher may start it with
# System32 as its working directory, so a relative path is not reliable.
if ($PSCmdlet.ShouldProcess('SUDOKOOLER_BIN', "Set the per-user daemon path to '$daemonExe'")) {
    [Environment]::SetEnvironmentVariable('SUDOKOOLER_BIN', $daemonExe, 'User')
    $env:SUDOKOOLER_BIN = $daemonExe
    Write-Host "    SUDOKOOLER_BIN = $daemonExe"
}

# ---------------------------------------------------------------------------
# 4. Start Menu shortcut
# ---------------------------------------------------------------------------
Write-Step '[4/5] Creating the Start Menu shortcut ...'
$programs = [Environment]::GetFolderPath('Programs')
if ([string]::IsNullOrEmpty($programs)) {
    Write-Warning 'Could not resolve the Start Menu Programs folder; shortcut skipped.'
}
else {
    $shortcutPath = Join-Path $programs 'sudocooler.lnk'
    if ($PSCmdlet.ShouldProcess($shortcutPath, 'Create/refresh the Start Menu shortcut')) {
        $shell = New-Object -ComObject WScript.Shell
        $shortcut = $shell.CreateShortcut($shortcutPath)
        # PawnIO needs elevation; the VBScript ShellExecutes with "runas" so
        # the UAC prompt appears directly and no console flashes.
        $shortcut.TargetPath = Join-Path $env:WINDIR 'System32\wscript.exe'
        $shortcut.Arguments = """" + (Join-Path $InstallDir 'launch-elevated.vbs') + """"
        $shortcut.WorkingDirectory = $InstallDir
        $shortcut.IconLocation = "$appExe,0"
        $shortcut.Description = 'sudocooler - Sudokoo CPU cooler dashboard'
        $shortcut.Save()
        Write-Host "    Shortcut: $shortcutPath"
    }
}

# ---------------------------------------------------------------------------
# 5. Elevated ONLOGON Scheduled Task (same task the app's toggle manages)
# ---------------------------------------------------------------------------
if ($SkipTask) {
    Write-Step '[5/5] Skipping Scheduled Task registration (-SkipTask).'
}
else {
    Write-Step "[5/5] Registering the elevated ONLOGON Scheduled Task '$TaskName' ..."
    if ($PSCmdlet.ShouldProcess("Scheduled Task '$TaskName'", 'Register elevated logon task')) {
        $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
            [Security.Principal.WindowsBuiltInRole]::Administrator)
        if (-not $isAdmin) {
            Write-Warning 'Registering the elevated logon task needs administrator rights; run this script from an elevated prompt, or enable Start at login from the app (launched through the shortcut).'
        }
        else {
            $register = Join-Path $sourceDir 'register-task.ps1'
            if (-not (Test-Path -LiteralPath $register)) {
                throw "'$register' is missing; re-extract the portable package."
            }
            & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $register -Exe $appExe -TaskName $TaskName
            if ($LASTEXITCODE -ne 0) {
                throw "Registering the logon task failed with exit code $LASTEXITCODE."
            }
        }
    }
}

Write-Host ''
Write-Host 'Installation complete.' -ForegroundColor Green
Write-Host "  App    : $appExe"
Write-Host "  Daemon : $daemonExe"
Write-Host "  Uninstall: powershell -ExecutionPolicy Bypass -File `"$InstallDir\uninstall.ps1`""
