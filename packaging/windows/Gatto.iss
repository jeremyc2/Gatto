; Per-user Windows x64 installer. The app owns its saved projects and settings
; in Local AppData; Inno manages only binaries, shortcuts and registration.
#ifndef AppVersion
  #error AppVersion is required
#endif
#ifndef GattoExe
  #error GattoExe is required
#endif
#ifndef SetupName
  #error SetupName is required
#endif
#ifndef ReleaseDir
  #error ReleaseDir is required
#endif

#define GattoId "com.jeremy-chandler.gatto"
#define AppClass "Software\Classes\Applications\Gatto.exe"
#define SchemeClass "Software\Classes\gatto"

[Setup]
AppId={#GattoId}
AppName=Gatto
AppPublisher=Jeremy Chandler
AppPublisherURL=https://github.com/jeremyc2/Gatto
AppSupportURL=https://github.com/jeremyc2/Gatto/blob/main/SUPPORT.md
AppUpdatesURL=https://github.com/jeremyc2/Gatto/releases?q=windows-v
AppVersion={#AppVersion}
DefaultDirName={localappdata}\Programs\Gatto
DisableProgramGroupPage=yes
DisableDirPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
MinVersion=10.0.22000
AppMutex=Local\Gatto.Desktop.78623da0
CloseApplications=no
ChangesAssociations=yes
SetupIconFile=Gatto.ico
UninstallDisplayIcon={app}\Gatto.exe
OutputDir={#ReleaseDir}
OutputBaseFilename={#SetupName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Files]
Source: "{#GattoExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\Gatto"; Filename: "{app}\Gatto.exe"; WorkingDir: "{app}"; AppUserModelID: "{#GattoId}"
Name: "{usersendto}\Gatto"; Filename: "{app}\Gatto.exe"; Parameters: "--files"; WorkingDir: "{app}"

[Registry]
; Offer Gatto in Open with without taking over anybody's defaults.
Root: HKCU; Subkey: "{#AppClass}"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "Gatto"; Flags: uninsdeletekey
Root: HKCU; Subkey: "{#AppClass}\DefaultIcon"; ValueType: string; ValueData: "{app}\Gatto.exe,0"
Root: HKCU; Subkey: "{#AppClass}\shell\open\command"; ValueType: string; ValueData: """{app}\Gatto.exe"" --files ""%1"""
Root: HKCU; Subkey: "{#AppClass}\SupportedTypes"; ValueType: string; ValueName: ".png"
Root: HKCU; Subkey: "{#AppClass}\SupportedTypes"; ValueType: string; ValueName: ".jpg"
Root: HKCU; Subkey: "{#AppClass}\SupportedTypes"; ValueType: string; ValueName: ".jpeg"
Root: HKCU; Subkey: "{#AppClass}\SupportedTypes"; ValueType: string; ValueName: ".gif"
Root: HKCU; Subkey: "{#AppClass}\SupportedTypes"; ValueType: string; ValueName: ".webp"
Root: HKCU; Subkey: "{#AppClass}\SupportedTypes"; ValueType: string; ValueName: ".mp4"
Root: HKCU; Subkey: "{#AppClass}\SupportedTypes"; ValueType: string; ValueName: ".mov"
Root: HKCU; Subkey: "{#AppClass}\SupportedTypes"; ValueType: string; ValueName: ".webm"

Root: HKCU; Subkey: "{#SchemeClass}"; ValueType: string; ValueData: "URL:Gatto"; Flags: uninsdeletekey
Root: HKCU; Subkey: "{#SchemeClass}"; ValueType: string; ValueName: "URL Protocol"
Root: HKCU; Subkey: "{#SchemeClass}\DefaultIcon"; ValueType: string; ValueData: "{app}\Gatto.exe,0"
Root: HKCU; Subkey: "{#SchemeClass}\shell\open\command"; ValueType: string; ValueData: """{app}\Gatto.exe"" --url ""%1"""

Root: HKCU; Subkey: "Software\Classes\AppUserModelId\{#GattoId}"; ValueType: string; ValueName: "DisplayName"; ValueData: "Gatto"; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "Gatto"; Flags: dontcreatekey uninsdeletevalue

[Run]
Filename: "{app}\Gatto.exe"; Description: "Open Gatto"; Flags: postinstall nowait skipifsilent
