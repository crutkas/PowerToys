<#
.SYNOPSIS
    Builds all Rust modules and copies outputs to the PowerToys bin directory.
    Run this before or after the main MSBuild solution build.

.PARAMETER Platform
    Target platform: x64 (default) or ARM64

.PARAMETER Configuration
    Build configuration: Debug (default) or Release

.PARAMETER SkipBuild
    Skip cargo build, only copy existing outputs

.EXAMPLE
    .\Build-RustModules.ps1
    .\Build-RustModules.ps1 -Configuration Release
    .\Build-RustModules.ps1 -Platform ARM64 -Configuration Release
#>
param(
    [ValidateSet("x64", "ARM64")]
    [string]$Platform = "x64",

    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Debug",

    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$RustWorkspace = $ScriptDir
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir "..\..")).Path
$OutDir = Join-Path $RepoRoot "$Platform\$Configuration"

# Map platform to Rust target
$RustTarget = switch ($Platform) {
    "x64"   { "x86_64-pc-windows-msvc" }
    "ARM64" { "aarch64-pc-windows-msvc" }
}

# Map configuration to Cargo profile
$CargoProfile = if ($Configuration -eq "Release") { "release" } else { "debug" }
$CargoFlag = if ($Configuration -eq "Release") { "--release" } else { "" }
$CargoOutputDir = Join-Path $RustWorkspace "target\$RustTarget\$CargoProfile"

# DLL and EXE name mappings (Rust output → PowerToys expected name)
$Dlls = @{
    "awake_module_interface.dll"           = "PowerToys.AwakeModuleInterface.dll"
    "alwaysontop_module_interface.dll"     = "PowerToys.AlwaysOnTopModuleInterface.dll"
    "advancedpaste_module_interface.dll"   = "PowerToys.AdvancedPasteModuleInterface.dll"
    "cmdnotfound_module_interface.dll"     = "PowerToys.CmdNotFoundModuleInterface.dll"
    "cmdpal_module_interface.dll"          = "PowerToys.CmdPalModuleInterface.dll"
    "cropandlock_module_interface.dll"     = "PowerToys.CropAndLockModuleInterface.dll"
    "fancyzones_module_interface.dll"      = "PowerToys.FancyZonesModuleInterface.dll"
    "lightswitch_module_interface.dll"     = "PowerToys.LightSwitchModuleInterface.dll"
    "mousewithoutborders_module_interface.dll" = "PowerToys.MouseWithoutBordersModuleInterface.dll"
    "poweraccent_module_interface.dll"     = "PowerToys.PowerAccentModuleInterface.dll"
    "powerdisplay_module_interface.dll"    = "PowerToys.PowerDisplayModuleInterface.dll"
    "powerocr_module_interface.dll"        = "PowerToys.PowerOCRModuleInterface.dll"
    "shortcutguide_module_interface.dll"   = "PowerToys.ShortcutGuideModuleInterface.dll"
    "workspaces_module_interface.dll"      = "PowerToys.WorkspacesModuleInterface.dll"
    "zoomit_module_interface.dll"          = "PowerToys.ZoomItModuleInterface.dll"
}

$Exes = @{
    "PowerToys-AlwaysOnTop.exe"              = "PowerToys.AlwaysOnTop.exe"
    "PowerToys-ActionRunner.exe"             = "PowerToys.ActionRunner.exe"
    "PowerToys-Update.exe"                   = "PowerToys.Update.exe"
    "awake.exe"                              = "PowerToys.Awake.exe"
    "PowerToys_WorkspacesSnapshotTool.exe"   = "PowerToys.WorkspacesSnapshotTool.exe"
    "PowerToys_WorkspacesLauncher.exe"       = "PowerToys.WorkspacesLauncher.exe"
    "PowerToys_WorkspacesWindowArranger.exe" = "PowerToys.WorkspacesWindowArranger.exe"
}

# Step 1: Build
if (-not $SkipBuild) {
    Write-Host "Building Rust workspace ($RustTarget, $CargoProfile)..." -ForegroundColor Cyan
    Push-Location $RustWorkspace
    $env:CARGO_TARGET_DIR = Join-Path $RustWorkspace "target"

    $buildArgs = @("build", "--workspace", "--target", $RustTarget, "--exclude", "powertoys-integration-tests")
    if ($CargoFlag) { $buildArgs += $CargoFlag }

    & cargo @buildArgs
    if ($LASTEXITCODE -ne 0) {
        Pop-Location
        throw "Cargo build failed with exit code $LASTEXITCODE"
    }
    Pop-Location
    Write-Host "Rust build complete" -ForegroundColor Green
}

# Step 2: Copy outputs
Write-Host "Copying Rust outputs to $OutDir..." -ForegroundColor Cyan
New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

$copied = 0
foreach ($src in $Dlls.Keys) {
    $srcPath = Join-Path $CargoOutputDir $src
    $dstPath = Join-Path $OutDir $Dlls[$src]
    if (Test-Path $srcPath) {
        Copy-Item $srcPath $dstPath -Force
        $copied++
    } else {
        Write-Warning "Missing: $src"
    }
}

foreach ($src in $Exes.Keys) {
    $srcPath = Join-Path $CargoOutputDir $src
    $dstPath = Join-Path $OutDir $Exes[$src]
    if (Test-Path $srcPath) {
        Copy-Item $srcPath $dstPath -Force
        $copied++
    } else {
        Write-Warning "Missing: $src"
    }
}

Write-Host "Copied $copied files to $OutDir" -ForegroundColor Green
