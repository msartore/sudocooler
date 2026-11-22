<#
.SYNOPSIS
    Removes the per-user sudocooler installation.

.DESCRIPTION
    Run the copy in the install folder (recommended):
        powershell -ExecutionPolicy Bypass -File "$env:LOCALAPPDATA\Programs\sudocooler\uninstall.ps1"

    Or run this script with -InstallDir pointing at the folder to remove.

    Removes:
      * the elevated ONLOGON Scheduled Task "sudocooler",
      * the Start Menu shortcut,
      * the per-user SUDOKOOLER_BIN variable (only if it points into the install folder),
      * the install folder itself (after stopping only the installed processes).

    PawnIO is intentionally left installed: other tools (FanControl, OpenRGB, ...)
    may depend on it. Uninstall it separately from Windows Settings -> Apps.

    Supports -WhatIf and -Confirm; safe to re-run.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\uninstall.ps1

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\uninstall.ps1 -InstallDir 'D:\apps\sudocooler'

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\uninstall.ps1 -WhatIf
#>

[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'High')]
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA 'Programs\sudocooler')
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$TaskName = 'sudocooler'

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

if (-not [System.IO.Path]::IsPathRooted($InstallDir)) {
    $InstallDir = Join-Path $env:LOCALAPPDATA $InstallDir
}
$InstallDir = [System.IO.Path]::GetFullPath($InstallDir)
Write-Host "Install folder: $InstallDir"

# ---------------------------------------------------------------------------
# 1. Scheduled Task (the app's disable() uses the same schtasks command)
# ---------------------------------------------------------------------------
Write-Step '[1/4] Removing the Scheduled Task ...'
if ($PSCmdlet.ShouldProcess("Scheduled Task '$TaskName'", 'Delete logon task')) {
    Invoke-NativeChecked -FilePath 'schtasks.exe' -Arguments @('/Delete', '/TN', $TaskName, '/F') -Description 'schtasks /Delete' -AllowFailure
}
else {
    Write-Host '    Skipped (WhatIf/Confirm).'
}

# ---------------------------------------------------------------------------
# 2. Start Menu shortcut
# ---------------------------------------------------------------------------
Write-Step '[2/4] Removing the Start Menu shortcut ...'
$programs = [Environment]::GetFolderPath('Programs')
if (-not [string]::IsNullOrEmpty($programs)) {
    $shortcutPath = Join-Path $programs 'sudocooler.lnk'
    if (Test-Path -LiteralPath $shortcutPath) {
        if ($PSCmdlet.ShouldProcess($shortcutPath, 'Delete Start Menu shortcut')) {
            Remove-Item -LiteralPath $shortcutPath -Force
            Write-Host "    Removed $shortcutPath"
        }
    }
    else {
        Write-Host '    Shortcut not present.'
    }
}

# ---------------------------------------------------------------------------
# 3. Per-user daemon path (do not clobber a value that belongs to someone else)
# ---------------------------------------------------------------------------
Write-Step '[3/4] Cleaning the SUDOKOOLER_BIN user variable ...'
$current = [Environment]::GetEnvironmentVariable('SUDOKOOLER_BIN', 'User')
if (-not [string]::IsNullOrEmpty($current)) {
    $expected = Join-Path $InstallDir 'sudokooler.exe'
    if ($current -eq $expected -or $current.StartsWith($InstallDir, [System.StringComparison]::OrdinalIgnoreCase)) {
        if ($PSCmdlet.ShouldProcess('SUDOKOOLER_BIN', 'Remove per-user daemon path')) {
            [Environment]::SetEnvironmentVariable('SUDOKOOLER_BIN', $null, 'User')
            if ($env:SUDOKOOLER_BIN -eq $current) { Remove-Item Env:\SUDOKOOLER_BIN -ErrorAction SilentlyContinue }
            Write-Host '    SUDOKOOLER_BIN removed.'
        }
    }
    else {
        Write-Host "    Left unchanged (points elsewhere: $current)."
    }
}
else {
    Write-Host '    Not set.'
}

# ---------------------------------------------------------------------------
# 4. Stop the installed processes and remove the folder
# ---------------------------------------------------------------------------
Write-Step '[4/4] Removing installed files ...'
$running = @(Get-Process -ErrorAction SilentlyContinue |
    Where-Object {
        $_.ProcessName -in @('sudocooler', 'sudokooler') -and
        $_.Path -and
        $_.Path.StartsWith($InstallDir, [System.StringComparison]::OrdinalIgnoreCase)
    })
foreach ($proc in $running) {
    if ($PSCmdlet.ShouldProcess("$($proc.ProcessName) (PID $($proc.Id))", 'Stop installed process')) {
        Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
    }
}

if (Test-Path -LiteralPath $InstallDir) {
    if ($PSCmdlet.ShouldProcess($InstallDir, 'Delete the installation folder')) {
        Remove-Item -LiteralPath $InstallDir -Recurse -Force
        Write-Host "    Removed $InstallDir"
    }
}
else {
    Write-Host '    Install folder not present.'
}

Write-Host ''
Write-Host 'Uninstall complete.' -ForegroundColor Green
Write-Host '  PawnIO was left installed on purpose (other apps may use it).'
