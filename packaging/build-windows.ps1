# FireCraft Windows recipe (M12): portable zip + optional Inno Setup
# installer.
#
# OFF-FLEET recipe: the release fleet has no Windows toolchain, so the
# repo never builds or ships a Windows artifact — the operator runs
# this on Windows (PowerShell 5.1+ or PowerShell 7, MSVC toolchain via
# rustup, Visual Studio Build Tools for link.exe) and owns the
# artifact. See packaging/TOOLCHAINS.md for the no-untested-claims
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
Copy-Item LICENSE-MIT "$Lic\LICENSE-MIT"
Copy-Item LICENSE-APACHE "$Lic\LICENSE-APACHE"

Write-Host "==> portable zip $Portable"
Compress-Archive -Path "$Stage\firecraft-$Version" -DestinationPath $Portable -Force
(Get-FileHash $Portable -Algorithm SHA256).Hash

$Isscc = Get-Command ISCC.exe -ErrorAction SilentlyContinue
if ($Isscc) {
    Write-Host "==> Inno Setup installer $Installer"
    # The checked-in .iss carries a __APPVERSION__ stand-in; the
    # substituted copy is staged so the tree never carries generated
    # files.
    $Iss = (Get-Content packaging\windows\firecraft.iss -Raw) `
        -replace '__APPVERSION__', $Version
    $StagedIss = "$Stage\firecraft.iss"
    $Iss | Set-Content -Encoding UTF8 $StagedIss
    & ISCC "/O$Stage" $StagedIss
    if ($LASTEXITCODE -ne 0) { throw "ISCC failed" }
    Move-Item "$Stage\firecraft-$Version-setup.exe" $Installer -Force
    (Get-FileHash $Installer -Algorithm SHA256).Hash
} else {
    Write-Host "==> ISCC.exe (Inno Setup) not on PATH: skipping installer."
    Write-Host "    The portable zip is the Windows artifact; the installer"
    Write-Host "    recipe lives in packaging/windows/firecraft.iss."
}
