; Inno Setup script for the sudocooler Windows installer.
; Build with: ISCC.exe /D"SourceDir=<portable folder>" /D"OutDir=<output>" sudocooler.iss

#ifndef SourceDir
  #define SourceDir "..\dist\sudocooler-portable"
#endif
#ifndef OutDir
  #define OutDir "..\dist"
#endif

#define AppName "sudocooler"
#define AppVersion "0.1.7"
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
; PawnIO needs elevation. The shortcuts launch through a hidden PowerShell
; using -Verb RunAs so CPU temperature and package power stay available.
Name: "{group}\{#AppName}"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -WindowStyle Hidden -Command ""Start-Process -FilePath '{app}\sudocooler.exe' -Verb RunAs"""; IconFilename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -WindowStyle Hidden -Command ""Start-Process -FilePath '{app}\sudocooler.exe' -Verb RunAs"""; IconFilename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
; Elevated logon task: CPU temperature and package power need PawnIO elevation.
Filename: "{sys}\schtasks.exe"; Parameters: "/Create /TN sudocooler /TR ""{app}\{#AppExe}"" /SC ONLOGON /RL HIGHEST /F"; Flags: runhidden; StatusMsg: "Registering elevated autostart task..."
; Install the signed PawnIO driver only when it is missing.
Filename: "{app}\installer\PawnIO_setup.exe"; Parameters: "/S"; Flags: runhidden skipifdoesntexist; Check: PawnIONotInstalled; StatusMsg: "Installing PawnIO driver..."
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoProfile -WindowStyle Hidden -Command ""Start-Process -FilePath '{app}\sudocooler.exe' -Verb RunAs"""; Description: "Launch sudocooler now"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\schtasks.exe"; Parameters: "/Delete /TN sudocooler /F"; Flags: runhidden; RunOnceId: RemoveSudocoolerTask

[Code]
function PawnIONotInstalled: Boolean;
begin
  Result :=
    not RegKeyExists(HKLM, 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO') and
    not RegKeyExists(HKLM, 'SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\PawnIO');
end;
