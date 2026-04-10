# PowerToys Rust Port — Progress Tracker

## File Size Reduction (Release builds)

| Component | C++ | Rust | Reduction | Status |
|-----------|-----|------|-----------|--------|
| **Awake Module DLL** | 4,820 KB | 152 KB | **32x smaller** | ✅ Ported |
| **AlwaysOnTop Module DLL** | 364 KB | 142 KB | **2.6x smaller** | ✅ Ported |
| **AlwaysOnTop EXE** | 5,800 KB | 178 KB | **33x smaller** | ✅ Ported |
| **Ported total** | **10,984 KB** | **472 KB** | **96% reduction** | |

### Remaining Module Interface DLLs (not yet ported)

| Module DLL | C++ Size | Est. Rust | Priority |
|------------|----------|-----------|----------|
| AdvancedPaste | 5,730 KB | ~140 KB | |
| CmdPal | 5,488 KB | ~140 KB | |
| PowerDisplay | 5,470 KB | ~140 KB | |
| MouseWithoutBorders | 5,034 KB | ~140 KB | |
| Workspaces | 4,954 KB | ~140 KB | |
| CropAndLock | 4,944 KB | ~140 KB | |
| LightSwitch | 4,941 KB | ~140 KB | |
| ShortcutGuide | 4,918 KB | ~140 KB | |
| PowerAccent | 4,864 KB | ~140 KB | |
| PowerOCR | 4,862 KB | ~140 KB | |
| ZoomIt | 4,832 KB | ~140 KB | |
| CmdNotFound | 4,787 KB | ~140 KB | |
| FancyZones | 6,930 KB | ~140 KB | |
| **All 15 DLLs total** | **67 MB** | **~2.1 MB** | **97% reduction** |

## RAM Usage (Working Set)

| Process | C++ | Rust | Reduction |
|---------|-----|------|-----------|
| **AlwaysOnTop.exe** | 57.9 MB | 7.0 MB | **88% less** |
| **Runner (all modules loaded)** | 139.5 MB | 137.9 MB | -1.6 MB |

*Note: Awake.exe RAM is unchanged — the Rust module DLL launches the same C# Awake.exe.*

## RAM Detail (Private Bytes)

| Process | C++ | Rust |
|---------|-----|------|
| **AlwaysOnTop.exe** | 39.8 MB | 1.2 MB |

## Test Results

| Suite | Tests | Status |
|-------|-------|--------|
| Rust: FFI bridge + modules + integration | 36 | ✅ |
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
  ├─ loads PowerToys.AwakeModuleInterface.dll          ← RUST ✅
  │    └─ launches PowerToys.Awake.exe (C#)
  ├─ loads PowerToys.AlwaysOnTopModuleInterface.dll     ← RUST ✅
  │    └─ launches PowerToys.AlwaysOnTop.exe            ← RUST ✅
  ├─ loads PowerToys.FancyZonesModuleInterface.dll      (C++)
  │    └─ launches PowerToys.FancyZones.exe             (C++)
  └─ ... (13 more C++ module DLLs)
```
