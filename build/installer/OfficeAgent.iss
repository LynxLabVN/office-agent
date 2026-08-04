; ============================================================================
; office-agent Windows installer (Inno Setup 6)
; Build:  powershell -ExecutionPolicy Bypass -File build\installer\build.ps1
; ============================================================================
#define MyAppName      "Office Agent"
#define MyAppVersion   "0.1.0"
#define MyAppPublisher "LynxLabVN"
#define MyAppExeName   "launch.bat"
#define MyAppIco       "icon.ico"

[Setup]
AppId={{8F4A2E1C-3B7D-4E2A-9C1F-OfficeAgent0001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\Office Agent
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
Compression=lzma2/ultra64
SolidCompression=yes
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
WizardStyle=modern
PrivilegesRequired=admin
OutputDir=dist
OutputBaseFilename=OfficeAgent-Setup
LicenseFile=
SetupLogging=yes
SetupIconFile={#MyAppIco}
; Allow install to per-machine Program Files; venv + user data go to %LOCALAPPDATA%.

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
; Payload assembled by build.ps1 into build\installer\payload\
Source: "payload\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{group}\Office Agent"; Filename: "{app}\launch.bat"; IconFilename: "{app}\{#MyAppIco}"
Name: "{group}\Uninstall Office Agent"; Filename: "{uninstallexe}"
Name: "{commondesktop}\Office Agent"; Filename: "{app}\launch.bat"; IconFilename: "{app}\{#MyAppIco}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop icon"; GroupDescription: "Additional icons:"

[Run]
; Post-install: build venv, offline pip install, overlay delta, write config, copy skills.
Filename: "powershell.exe"; Parameters: "-ExecutionPolicy Bypass -NoProfile -File ""{app}\post-install.ps1"" -InstallDir ""{app}"""; StatusMsg: "Setting up Office Agent (venv + MCPs + dashboard)..."; Flags: runhidden waituntilterminated

[UninstallRun]
; Remove the venv only; keep HERMES_HOME user data (sessions, DBs, skills) by default.
Filename: "powershell.exe"; Parameters: "-NoProfile -Command ""if (Test-Path ""$env:LOCALAPPDATA\hermes\venv"") {{ Remove-Item -Recurse -Force ""$env:LOCALAPPDATA\hermes\venv"" }}"""; Flags: runhidden

[Code]
function InitializeSetup(): Boolean;
begin
  Result := True;
end;