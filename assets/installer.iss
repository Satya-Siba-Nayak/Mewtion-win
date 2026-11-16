#ifndef MyAppVersion
  #define MyAppVersion "0.1.0"
#endif
#ifndef MyAppArch
  #define MyAppArch "x86_64"
#endif
#ifndef SourceDir
  #define SourceDir "..\dist\Mewtion-v0.1.0-windows-x86_64"
#endif

[Setup]
AppId={{D9A6F2E0-5E81-4A1A-93D4-8C83E94F7120}
AppName=Mewtion
AppVersion={#MyAppVersion}
AppPublisher=Mewtion Open Source
AppPublisherURL=https://github.com/Satya-Siba-Nayak/Mewtion-win
DefaultDirName={autopf}\Mewtion
DefaultGroupName=Mewtion
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..
OutputBaseFilename=Mewtion-Setup-v{#MyAppVersion}-windows-{#MyAppArch}
SetupIconFile=mewtion.ico
UninstallDisplayIcon={app}\control_panel.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible arm64
ArchitecturesInstallIn64BitMode=x64compatible arm64

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startup"; Description: "Start Mewtion automatically when Windows starts"; GroupDescription: "Startup:"

[Files]
Source: "{#SourceDir}\Mewtion.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\control_panel.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\start.bat"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\mewtion_config.txt"; DestDir: "{app}"; Flags: onlyifdoesntexist
Source: "{#SourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\assets\*"; DestDir: "{app}\assets"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Mewtion Settings"; Filename: "{app}\control_panel.exe"; WorkingDir: "{app}"
Name: "{autoprograms}\Mewtion Overlay"; Filename: "{app}\Mewtion.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\Mewtion"; Filename: "{app}\control_panel.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Mewtion"; ValueData: """{app}\Mewtion.exe"""; Flags: uninsdeletevalue; Tasks: startup

[Run]
Filename: "{app}\control_panel.exe"; WorkingDir: "{app}"; Description: "Launch Mewtion"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "taskkill.exe"; Parameters: "/F /IM Mewtion.exe"; Flags: runhidden; RunOnceId: "StopMewtionOverlay"
Filename: "taskkill.exe"; Parameters: "/F /IM control_panel.exe"; Flags: runhidden; RunOnceId: "StopMewtionControlPanel"
