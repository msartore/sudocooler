<#
.SYNOPSIS
    Registers the elevated logon task for sudocooler.

.DESCRIPTION
    `schtasks /Create /TR` cannot express an executable path with spaces: it
    stores `C:\Program` and `Files\sudocooler\sudocooler.exe` and every run
    fails with 0x80070002. This script writes a task definition (XML) and
    creates the task from it, so the command is a single proper element.

    The task runs at sign-in, at the highest available run level, with
    `--minimized` (tray only). It is what makes PawnIO telemetry work without
    a UAC prompt at logon. Call it from an elevated installer/script.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\register-task.ps1 -Exe "C:\Program Files\sudocooler\sudocooler.exe"
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Exe,
    [string]$TaskName = 'sudocooler'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (-not (Test-Path -LiteralPath $Exe)) {
    throw "The executable '$Exe' does not exist."
}

$userId = if ([string]::IsNullOrEmpty($env:USERDOMAIN)) {
    $env:USERNAME
}
else {
    "$env:USERDOMAIN\$env:USERNAME"
}
$escape = { param($text) [System.Security.SecurityElement]::Escape($text) }
$exeXml = & $escape $Exe
$userXml = & $escape $userId

$xml = @"
<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>sudocooler at sign-in</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>$userXml</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>HighestAvailable</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <AllowHardTerminate>true</AllowHardTerminate>
    <StartWhenAvailable>true</StartWhenAvailable>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>$exeXml</Command>
      <Arguments>--minimized</Arguments>
    </Exec>
  </Actions>
</Task>
"@

$temp = Join-Path ([System.IO.Path]::GetTempPath()) ("sudocooler-task-" + [guid]::NewGuid().ToString('N') + ".xml")
try {
    # Task Scheduler reads the XML as UTF-16 with a BOM.
    [System.IO.File]::WriteAllText($temp, $xml, [System.Text.Encoding]::Unicode)
    & schtasks.exe /Create /TN $TaskName /XML $temp /F | ForEach-Object { Write-Host "    $_" }
    if ($LASTEXITCODE -ne 0) {
        throw "schtasks /Create /XML failed with exit code $LASTEXITCODE."
    }
}
finally {
    Remove-Item -LiteralPath $temp -Force -ErrorAction SilentlyContinue
}

Write-Host "Registered the elevated logon task '$TaskName'."
