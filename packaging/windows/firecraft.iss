; FireCraft Windows installer recipe (Inno Setup 6). The portable zip
; from build-windows.ps1 is the primary Windows artifact; this script
; is the optional installer. It carries two stand-ins — __APPVERSION__
; and __LICENSEFILE__ — and is compiled through build-windows.ps1,
; which substitutes both and stages the license beside the script.
; Compiling it directly leaves the stand-ins unresolved and fails:
; the repo carries a recipe, never an untested claim of a built
; artifact.

[Setup]
AppId={{7E6B4F4E-8E1F-4B0A-9A7C-4D2C1F0AF1CE}
AppName=FireCraft
AppVersion=__APPVERSION__
AppVerName=FireCraft __APPVERSION__
AppPublisher=FireCraft contributors
AppPublisherURL=https://github.com/aliasfoxkde/FireCraft
DefaultDirName={autopf}\FireCraft
DefaultGroupName=FireCraft
OutputBaseFilename=firecraft-__APPVERSION__-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
LicenseFile=__LICENSEFILE__
UninstallDisplayIcon={app}\bin\firecraft.exe

[Files]
Source: "firecraft-*\bin\firecraft.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "firecraft-*\bin\firecraft-cli.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
; The licenses directory carries both LICENSE and NOTICE: Apache-2.0
; 4(d) requires the attribution notices to travel with the
; distribution, and NOTICE is where the Adobe trademark disclaimer
; lives.
Source: "firecraft-*\share\licenses\*"; DestDir: "{app}\share\licenses"; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{group}\FireCraft"; Filename: "{app}\bin\firecraft.exe"
Name: "{group}\FireCraft CLI help"; Filename: "{cmd}"; Parameters: "/k ""{app}\bin\firecraft-cli.exe"" --help"

[Run]
Filename: "{app}\bin\firecraft.exe"; Description: "Launch FireCraft"; Flags: nowait postinstall skipifsilent
