# FireCraft Windows recipe (M12): portable zip + optional Inno Setup
# installer.
#
# OFF-FLEET recipe: the release fleet has no Windows toolchain, so the
# repo never builds or ships a Windows artifact — the operator runs
# this on Windows (PowerShell 5.1+ or PowerShell 7, MSVC toolchain via
# rustup, Visual Studio Build Tools for link.exe) and owns the
# artifact. See docs/TOOLCHAINS.md for the no-untested-claims
# rule. Run from the repository root.
#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$Target = "x86_64-pc-windows-msvc"
$Stage = "dist\windows-stage"

function Get-WorkspaceVersion {
    # The workspace version is the single source of truth.
    $lines = Get-Content Cargo.toml
    $inPackage = $false
    foreach ($line in $lines) {
        if ($line -match '^\[workspace\.package\]') { $inPackage = $true; continue }
        if ($inPackage -and $line -match '^\[') { break }
        if ($inPackage -and $line -match '^version = "(.*)"$') { return $Matches[1] }
    }
    throw "no version under [workspace.package] in Cargo.toml"
}

$Version = Get-WorkspaceVersion
$Portable = "firecraft-$Version-windows-x86_64.zip"
$Installer = "firecraft-$Version-windows-x86_64-setup.exe"

Write-Host "==> rustup target $Target"
rustup target add $Target

Write-Host "==> cargo build --release"
cargo build --release --locked -p app -p app-cli --target $Target
if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }

Write-Host "==> staging $Stage"
if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
$Bin = "$Stage\firecraft-$Version\bin"
$Lic = "$Stage\firecraft-$Version\share\licenses"
New-Item -ItemType Directory -Force -Path $Bin, $Lic | Out-Null
Copy-Item "target\$Target\release\firecraft.exe" $Bin
Copy-Item "target\$Target\release\firecraft-cli.exe" $Bin
Copy-Item LICENSE "$Lic\LICENSE"
# NOTICE travels with the binaries. Apache-2.0 4(d) requires a
# distribution to carry the attribution notices when the work has a
# NOTICE file, and this one carries the Adobe trademark disclaimer —
# a Windows install that omits it ships a legally incomplete bundle.
Copy-Item NOTICE "$Lic\NOTICE"

Write-Host "==> portable zip $Portable"
Compress-Archive -Path "$Stage\firecraft-$Version" -DestinationPath $Portable -Force
(Get-FileHash $Portable -Algorithm SHA256).Hash

$Isscc = Get-Command ISCC.exe -ErrorAction SilentlyContinue
if ($Isscc) {
    Write-Host "==> Inno Setup installer $Installer"
    # The checked-in .iss carries two stand-ins: __APPVERSION__ and
    # __LICENSEFILE__. The substituted copy is staged so the tree never
    # carries generated files.
    #
    # The license path is substituted as an ABSOLUTE path. The staged
    # .iss lives in the stage directory, so a relative path would be
    # resolved against the stage, not against the repository root —
    # the committed `..\..\LICENSE` pointed at a file that exists in
    # the repository but not next to the staged script, and the
    # compiler would have failed on a license it could not find.
    #
    # The substitution goes through a MatchEvaluator rather than
    # -replace: a Windows path may contain `$`, which the replacement
    # syntax would read as a capture-group reference.
    Copy-Item LICENSE "$Stage\LICENSE"
    $LicenseFile = (Resolve-Path "$Stage\LICENSE").Path
    $IssText = Get-Content packaging\windows\firecraft.iss -Raw
    $Iss = [regex]::Replace($IssText, '__APPVERSION__|__LICENSEFILE__', {
        param($match)
        if ($match.Value -eq '__APPVERSION__') { $Version } else { '"' + $LicenseFile + '"' }
    })
    $StagedIss = "$Stage\firecraft.iss"
    $Iss | Set-Content -Encoding UTF8 $StagedIss
    & ISCC "/O$Stage" $StagedIss
    if ($LASTEXITCODE -ne 0) { throw "ISCC failed" }
    # The installer's file name comes from the .iss's
    # OutputBaseFilename, which carries the version and the platform.
    # The recipe moves that exact name: a move of any other name finds
    # no file and fails after a successful compile.
    $SetupExe = "$Stage\firecraft-$Version-windows-x86_64-setup.exe"
    if (-not (Test-Path $SetupExe)) { throw "ISCC did not produce $SetupExe" }
    Move-Item $SetupExe $Installer -Force
    (Get-FileHash $Installer -Algorithm SHA256).Hash
} else {
    Write-Host "==> ISCC.exe (Inno Setup) not on PATH: skipping installer."
    Write-Host "    The portable zip is the Windows artifact; the installer"
    Write-Host "    recipe lives in packaging/windows/firecraft.iss."
}
