# PowerToys Rust Port — Progress Tracker

*Last updated: 2026-04-11*

## Summary

| Metric | C++ | Rust | Change |
|--------|-----|------|--------|
| **All module DLLs (15)** | 74.4 MB | 1.6 MB | **47x smaller** |
| **Ported EXEs (4)** | 16.1 MB | 2.1 MB | **8x smaller** |
| **Total ported** | **86.9 MB** | **3.6 MB** | **24x smaller** |
| **Runner RAM (private)** | 87.5 MB | **55.6 MB** | **36% less** |
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
| ActionRunner.exe | 4,628 KB (C++) | 110 KB | 42x | — | ✅ |
| Update.exe | 5,377 KB (C++) | 1,396 KB | 3.9x | — | ✅ |
| Awake.exe | 254 KB (C#) +.NET | 401 KB | standalone | 50→5.0 MB | ✅ via crutkas/awake |

## RAM Usage

| Process | C++/C# WS | Rust WS | C++/C# Private | Rust Private |
|---------|-----------|---------|----------------|--------------|
| **Runner (all 15 Rust DLLs)** | 139.5 MB | **109.8 MB** | 87.5 MB | **55.6 MB** |
| **AlwaysOnTop.exe** | 57.9 MB | 7.0 MB | 39.8 MB | 1.2 MB |
| **Awake.exe** | ~50 MB | 5.0 MB | — | — |

## Test Results

| Suite | Tests | Status |
|-------|-------|--------|
| Rust: FFI bridge + modules + integration | 113 | ✅ |
| Rust: awake binary (crutkas/awake) | 84 | ✅ |
| Rust: Update.exe | 19 | ✅ |
| C++: CommonLib + CommonUtils | 524 | ✅ |
| .NET: ColorPicker | 378 | ✅ |
| .NET: Hosts | 151 | ✅ |
| .NET: ImageResizer | 134 | ✅ |
| .NET: PowerToys Run (Wox) | 130 | ✅ |
| .NET: MouseJump | 52 | ✅ |
| .NET: AdvancedPaste | 42 | ✅ |
| **Total** | **1,627+** | **0 failures** |

## Roadmap

### Phase 1 — ✅ Complete
- FFI bridge + vtable adapter
- All 15 module DLLs ported
- AlwaysOnTop, Awake, ActionRunner, Update EXEs ported
- CI pipeline (GitHub Actions)
- 200+ Rust tests

### Phase 2 — In Progress
- [ ] Installer integration (WiX .wxs files)
- [ ] MSBuild solution wiring (Rust projects in PowerToys.slnx)
- [ ] Remove/disable old C++ module DLL projects
- [ ] Awake settings restart on config change ✅
- [ ] Workspaces tools (3 EXEs — deferred, large shared lib)

### Phase 3 — Future
- [ ] FancyZones EXE (D2D overlay, complex)
- [ ] CropAndLock EXE (WinRT Composition overlay — needs matching UX)
- [ ] Port the runner (5.8K LOC, eliminates last C++ dependency)

### Not porting (keep C++)
- ZoomIt (11 MB, complex D2D drawing/recording — better as-is)
- ShortcutGuide (D2D overlay — low value vs effort)

## Branches

| Branch | Purpose |
|--------|---------|
| **[`rust-awake-module`](https://github.com/crutkas/PowerToys/tree/rust-awake-module)** | Main integration branch |
| [`rust-poweraccent-exe`](https://github.com/crutkas/PowerToys/tree/rust-poweraccent-exe) | PowerAccent EXE (parked — UX review needed) |
| [`rust-cropandlock-exe`](https://github.com/crutkas/PowerToys/tree/rust-cropandlock-exe) | CropAndLock EXE (parked — UX doesn't match) |
