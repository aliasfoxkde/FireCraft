; FireCraft Windows installer recipe (Inno Setup 6). The portable zip
; from build-windows.ps1 is the primary Windows artifact; this script
; is the optional installer. It is substituted (with __APPVERSION__)
; and run by build-windows.ps1 when ISCC.exe is on PATH — the repo
; carries the recipe, never an untested claim of a built artifact.

[Setup]
AppId={{7E6B4F4E-8E1F-4B0A-9A7C-4D2C1F0AF1CE}
AppName=FireCraft
AppVersion=__APPVERSION__
AppVerName=FireCraft __APPVERSION__
DefaultDirName={autopf}\FireCraft
DefaultGroupName=FireCraft
OutputBaseFilename=firecraft-__APPVERSION__-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=..\..\LICENSE-MIT
UninstallDisplayIcon={app}\bin\firecraft.exe

[Files]
Source: "firecraft-*\bin\firecraft.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "firecraft-*\bin\firecraft-cli.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "firecraft-*\share\licenses\*"; DestDir: "{app}\share\licenses"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{group}\FireCraft"; Filename: "{app}\bin\firecraft.exe"
Name: "{group}\FireCraft CLI help"; Filename: "{cmd}"; Parameters: "/k ""{app}\bin\firecraft-cli.exe"" --help"

[Run]
Filename: "{app}\bin\firecraft.exe"; Description: "Launch FireCraft"; Flags: nowait postinstall skipifsilent
