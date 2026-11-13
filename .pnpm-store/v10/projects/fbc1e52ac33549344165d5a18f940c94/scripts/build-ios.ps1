<#
.SYNOPSIS
    Build Liner Music for iOS on a Windows host and produce an unsigned .ipa.

.DESCRIPTION
    Thin wrapper around the WSL build pipeline:
      1. pnpm install (only when node_modules is missing)
      2. pnpm build        -> dist/ (the WKWebView bundle)
      3. bash ios-build.sh -> cargo + clang + lld + zip, inside WSL
      4. copy the .ipa into ./release

    No macOS, no Xcode, no Apple developer certificate is involved.

.PARAMETER SkipFrontend
    Reuse an existing dist/ (useful while iterating on Rust only).

.PARAMETER Distro
    WSL distribution to build in. Defaults to Ubuntu-24.04.

.EXAMPLE
    pwsh -File scripts/build-ios.ps1
#>
[CmdletBinding()]
param(
    [switch]$SkipFrontend,
    [string]$Distro = "Ubuntu-24.04"
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $projectRoot

function Invoke-Step {
    param([string]$Name, [scriptblock]$Action)
    Write-Host ""
    Write-Host "==> $Name" -ForegroundColor Cyan
    & $Action
    if ($LASTEXITCODE -ne 0) { throw "step failed: $Name (exit $LASTEXITCODE)" }
}

# ---------------------------------------------------------------- frontend --
if (-not $SkipFrontend) {
    if (-not (Test-Path "$projectRoot\node_modules")) {
        Invoke-Step "pnpm install" { pnpm install }
    }
    Invoke-Step "pnpm build" { pnpm build }
}

# ------------------------------------------------------------------- build --
# Translate the Windows checkout path into its WSL mount so the script is
# self-contained regardless of where the repository lives.
$wslProject = (wsl -d $Distro -- wslpath -a "$projectRoot").Trim()
if (-not $wslProject) { throw "could not resolve the WSL path for $projectRoot" }

Invoke-Step "cross-compile + package (WSL: $Distro)" {
    wsl -d $Distro -- bash "$wslProject/scripts/ios-build.sh"
}

# ------------------------------------------------------------------ collect --
$ipa = "\\wsl.localhost\$Distro\opt\ios\src\build-ios\LinerMusic-unsigned-ios.ipa"
$outDir = Join-Path $projectRoot "release"
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

if (Test-Path $ipa) {
    $target = Join-Path $outDir "LinerMusic-unsigned-ios.ipa"
    Copy-Item $ipa $target -Force
    $size = [math]::Round((Get-Item $target).Length / 1MB, 2)
    Write-Host ""
    Write-Host "ipa: $target ($size MB)" -ForegroundColor Green
}
else {
    Write-Warning "build finished but the .ipa was not found at $ipa"
}
