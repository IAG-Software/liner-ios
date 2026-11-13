<#
.SYNOPSIS
    One-time provisioning of the iOS cross-compilation toolchain inside WSL.

.DESCRIPTION
    Installs, inside the WSL distribution:
      * clang + lld (C/ObjC driver and Mach-O linker)
      * a dedicated Rust toolchain at /opt/rust with the aarch64-apple-ios target
      * the iPhoneOS SDK (theos/sdks stubs) at /opt/ios/sdk-src

    Everything is isolated under /opt so it cannot disturb the host toolchains.

.PARAMETER Distro
    WSL distribution to provision. Defaults to Ubuntu-24.04.

.PARAMETER SdkVersion
    iPhoneOS SDK version to fetch from theos/sdks. Defaults to 16.5 (the newest
    public stub set; stubs are forward compatible, so apps built against it run
    on iOS 27).

.EXAMPLE
    pwsh -File scripts/setup-toolchain.ps1
#>
[CmdletBinding()]
param(
    [string]$Distro = "Ubuntu-24.04",
    [string]$SdkVersion = "16.5"
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $projectRoot

function Invoke-Wsl {
    param([string]$Command)
    wsl -d $Distro -- bash -lc $Command
    if ($LASTEXITCODE -ne 0) { throw "command failed: $Command" }
}

Write-Host "==> installing apt packages (clang, lld, zip, rsync)" -ForegroundColor Cyan
Invoke-Wsl "export DEBIAN_FRONTEND=noninteractive; apt-get update -qq && apt-get install -y -qq clang lld llvm zip unzip rsync xz-utils git curl pkg-config libssl-dev"

Write-Host "==> Rust toolchain at /opt/rust + aarch64-apple-ios" -ForegroundColor Cyan
Invoke-Wsl "bash /mnt/d/ios2/scripts/wsl-setup-rust.sh"

Write-Host "==> iPhoneOS $SdkVersion SDK stubs" -ForegroundColor Cyan
Invoke-Wsl "if [ ! -d /opt/ios/sdk-src/iPhoneOS$SdkVersion.sdk ]; then mkdir -p /opt/ios && cd /opt/ios && git clone --filter=blob:none --sparse --depth 1 https://github.com/theos/sdks.git sdk-src && cd sdk-src && git sparse-checkout set iPhoneOS$SdkVersion.sdk; fi; du -sh /opt/ios/sdk-src/iPhoneOS$SdkVersion.sdk"

Write-Host ""
Write-Host "toolchain ready. run: pwsh -File scripts/build-ios.ps1" -ForegroundColor Green
