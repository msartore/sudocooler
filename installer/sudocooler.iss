; Inno Setup script for the sudocooler Windows installer.
; Build with: ISCC.exe /D"SourceDir=<portable folder>" /D"OutDir=<output>" sudocooler.iss

#ifndef SourceDir
  #define SourceDir "..\dist\sudocooler-portable"
#endif
#ifndef OutDir
  #define OutDir "..\dist"
#endif

#define AppName "sudocooler"
#define AppVersion "0.1.15"
#define AppPublisher "msartore"
#define AppURL "https://msartore.dev"
#define AppExe "sudocooler.exe"

[Setup]
AppId={{4E567778-83AA-4A6F-8D48-EE001D597570}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}
AppUpdatesURL=https://github.com/msartore/sudocooler/releases
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
OutputDir={#OutDir}
OutputBaseFilename=sudocooler-setup
SetupIconFile=..\assets\icon.ico
LicenseFile=..\LICENSE
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=admin
UninstallDisplayIcon={app}\{#AppExe}
DisableProgramGroupPage=yes
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: english; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: desktopicon; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
; PawnIO needs elevation. Launch through a VBScript that ShellExecutes with
; "runas": the UAC prompt appears directly and no console window flashes.
Name: "{group}\{#AppName}"; Filename: "{sys}\wscript.exe"; Parameters: """{app}\launch-elevated.vbs"""; IconFilename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{sys}\wscript.exe"; Parameters: """{app}\launch-elevated.vbs"""; IconFilename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
; Clear a per-user Run entry a 0.1.14 install may have written, so it cannot
; start the app unelevated next to the elevated task.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueName: "{#AppName}"; ValueType: none; Flags: dontcreatekey uninsdeletevalue

[Run]
; Elevated logon task: CPU temperature and package power need PawnIO elevation.
; The task XML is written by the script, because `schtasks /TR` splits a path
; containing spaces (older builds stored Command=C:\Program).
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\register-task.ps1"" -Exe ""{app}\{#AppExe}"""; Flags: runhidden; StatusMsg: "Registering the elevated logon task..."
; Install the signed PawnIO driver only when it is missing.
Filename: "{app}\installer\PawnIO_setup.exe"; Parameters: "/S"; Flags: runhidden skipifdoesntexist; Check: PawnIONotInstalled; StatusMsg: "Installing PawnIO driver..."
Filename: "{sys}\wscript.exe"; Parameters: """{app}\launch-elevated.vbs"""; Description: "Launch sudocooler now"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\schtasks.exe"; Parameters: "/Delete /TN sudocooler /F"; Flags: runhidden; RunOnceId: RemoveSudocoolerTask

[Code]
function PawnIONotInstalled: Boolean;
begin
  Result :=
    not RegKeyExists(HKLM, 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO') and
    not RegKeyExists(HKLM, 'SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO');
end;
