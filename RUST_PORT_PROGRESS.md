# PowerToys Rust Port — Progress Tracker

*Last updated: 2026-04-10*

## Summary

| Metric | C++ | Rust | Change |
|--------|-----|------|--------|
| **All module DLLs (15)** | 74.4 MB | 1.7 MB | **42x smaller** |
| **AlwaysOnTop EXE** | 5.8 MB | 178 KB | **33x smaller** |
| **Total ported** | **80.2 MB** | **1.9 MB** | **97.6% reduction** |
| **AOT EXE RAM (Working Set)** | 57.9 MB | 7.0 MB | **88% less** |
| **AOT EXE RAM (Private)** | 39.8 MB | 1.2 MB | **97% less** |

## Module Interface DLLs — All 15 Ported ✅

| Module DLL | C++ | Rust | Ratio | Status |
|------------|-----|------|-------|--------|
| FancyZones | 6,930 KB | 98 KB | 70x | ✅ |
| AdvancedPaste | 5,730 KB | 98 KB | 58x | ✅ |
| CmdPal | 5,488 KB | 98 KB | 56x | ✅ |
| PowerDisplay | 5,470 KB | 98 KB | 56x | ✅ |
| MouseWithoutBorders | 5,034 KB | 98 KB | 51x | ✅ |
| Workspaces | 4,954 KB | 98 KB | 50x | ✅ |
| CropAndLock | 4,944 KB | 98 KB | 50x | ✅ |
| LightSwitch | 4,941 KB | 98 KB | 50x | ✅ |
| ShortcutGuide | 4,918 KB | 98 KB | 50x | ✅ |
| PowerAccent | 4,864 KB | 98 KB | 49x | ✅ |
| PowerOCR | 4,862 KB | 98 KB | 49x | ✅ |
| CmdNotFound | 4,787 KB | 98 KB | 49x | ✅ |
| ZoomIt | 4,832 KB | 98 KB | 49x | ✅ |
| Awake | 4,820 KB | 152 KB | 32x | ✅ |
| AlwaysOnTop | 369 KB | 142 KB | 3x | ✅ |
| **DLL Total** | **74,408 KB** | **1,573 KB** | **47x** | |

## Application EXEs Ported

| EXE | Original | Rust | Ratio | RAM (WS) | Status |
|-----|----------|------|-------|----------|--------|
| AlwaysOnTop.exe | 5,800 KB (C++) | 178 KB | 33x | 57.9→7.0 MB | ✅ |
| Awake.exe | 254 KB (C#) +.NET | 401 KB | standalone | 50→5.0 MB | ✅ via crutkas/awake |

*Awake.exe is larger on disk (401 KB vs 254 KB) but eliminates the ~100 MB .NET runtime dependency and uses 90% less RAM.*

## RAM Usage

| Process | C++/C# WS | Rust WS | C++/C# Private | Rust Private |
|---------|-----------|---------|----------------|--------------|
| **AlwaysOnTop.exe** | 57.9 MB | 7.0 MB | 39.8 MB | 1.2 MB |
| **Awake.exe** | ~50 MB | 5.0 MB | — | — |
| **Runner (all modules)** | 139.5 MB | 137.9 MB | 87.5 MB | 85.6 MB |

## Test Results

| Suite | Tests | Status |
|-------|-------|--------|
| Rust: FFI bridge + modules + integration | 113 | ✅ |
| Rust: awake binary (crutkas/awake) | 84 | ✅ |
| C++: CommonLib + CommonUtils | 524 | ✅ |
| .NET: ColorPicker | 378 | ✅ |
| .NET: Hosts | 151 | ✅ |
| .NET: ImageResizer | 134 | ✅ |
| .NET: PowerToys Run (Wox) | 130 | ✅ |
| .NET: MouseJump | 52 | ✅ |
| .NET: AdvancedPaste | 42 | ✅ |
| **Total** | **1,531** | **0 failures** |

## Commits (branch: `rust-awake-module`)

| Hash | Description |
|------|-------------|
| `183ae8a` | feat: batch port all 15 module interface DLLs to Rust |
| `61a0851` | feat: integrate crutkas/awake as Rust Awake binary |
| `945ef0c` | test: add unit tests for all 13 batch-generated modules |
| `183ae8a` | feat: batch port all 15 module interface DLLs to Rust |
| `e32ae07` | feat: add rounded corner borders + progress tracker |
| `9a7b1ee` | fix: rewrite border rendering with proper UpdateLayeredWindow |
| `7e9fbf9` | fix: dangling AOT_INSTANCE pointer + stale terminate event |
| `ceda9e0` | feat: port AlwaysOnTop EXE to Rust |
| `cd67ca5` | feat: port AlwaysOnTop module interface to Rust |
| `eed6b7c` | perf: add optimized release profile (LTO, strip, opt-z, panic=abort) |
| `a0e1497` | fix: add missing GetHotkeyEx/OnHotkeyEx to vtable layout |
| `8d645a8` | feat: integrate Rust Awake module into PowerToys build tree |

## Architecture

```
PowerToys.exe (C++ runner — unchanged)
  ├─ loads PowerToys.AwakeModuleInterface.dll               ← RUST ✅
  │    └─ launches PowerToys.Awake.exe                       ← RUST ✅ (crutkas/awake)
  ├─ loads PowerToys.AlwaysOnTopModuleInterface.dll          ← RUST ✅
  │    └─ launches PowerToys.AlwaysOnTop.exe                 ← RUST ✅
  ├─ loads PowerToys.FancyZonesModuleInterface.dll           ← RUST ✅
  │    └─ launches PowerToys.FancyZones.exe                  (C++)
  ├─ loads PowerToys.AdvancedPasteModuleInterface.dll        ← RUST ✅
  ├─ loads PowerToys.CmdPalModuleInterface.dll               ← RUST ✅
  ├─ loads PowerToys.CmdNotFoundModuleInterface.dll          ← RUST ✅
  ├─ loads PowerToys.CropAndLockModuleInterface.dll          ← RUST ✅
  ├─ loads PowerToys.LightSwitchModuleInterface.dll          ← RUST ✅
  ├─ loads PowerToys.MouseWithoutBordersModuleInterface.dll  ← RUST ✅
  ├─ loads PowerToys.PowerAccentModuleInterface.dll          ← RUST ✅
  ├─ loads PowerToys.PowerDisplayModuleInterface.dll         ← RUST ✅
  ├─ loads PowerToys.PowerOCRModuleInterface.dll             ← RUST ✅
  ├─ loads PowerToys.ShortcutGuideModuleInterface.dll        ← RUST ✅
  ├─ loads PowerToys.WorkspacesModuleInterface.dll           ← RUST ✅
  └─ loads PowerToys.ZoomItModuleInterface.dll               ← RUST ✅
```
