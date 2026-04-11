# PowerToys Rust Port — Progress Tracker

*Last updated: 2026-04-11 07:00 UTC*

## Summary

| Metric | C++ | Rust | Change |
|--------|-----|------|--------|
| **All module DLLs (15)** | 74.4 MB | 1.6 MB | **47x smaller** |
| **Ported EXEs (4+3)** | 16.1 MB+ | 2.1 MB+ | **8x smaller** |
| **Runner RAM (private)** | 87.5 MB | **55.6 MB** | **36% less** |
| **AlwaysOnTop RAM** | 39.8 MB | 1.2 MB | **97% less** |
| **Rust tests** | 0 | **515** | — |

## Architecture

```
Rust crates (8 libraries, 7 apps):
├── libs/
│   ├── powertoys-module-ffi     — FFI bridge (trait + vtable)
│   ├── powertoys-win32          — Shared Win32 helpers (37 tests)
│   ├── fancyzones-core          — Zone math, layout, data (130 tests)
│   ├── fancyzones-engine        — Runtime engine: drag, snap, overlay (46 tests)
│   └── workspaces-core          — App detection, JSON, launch status (83 tests)
├── modules/ (15 DLLs)          — Module interface DLLs
├── apps/
│   ├── alwaysontop              — Pin windows with borders
│   ├── awake                    — Keep PC awake (submodule)
│   ├── actionrunner             — Non-elevated process launcher
│   ├── update                   — GitHub update checker
│   ├── workspaces-snapshot      — Capture window layout
│   ├── workspaces-launcher      — Launch workspace apps
│   └── workspaces-arranger      — Arrange windows to positions
└── tests/integration            — LoadLibrary + vtable tests
```

## Module Interface DLLs — All 15 Ported ✅

| Module DLL | C++ | Rust | Ratio |
|------------|-----|------|-------|
| FancyZones | 6,930 KB | 98 KB | 70x |
| AdvancedPaste | 5,730 KB | 98 KB | 58x |
| CmdPal | 5,488 KB | 98 KB | 56x |
| PowerDisplay | 5,470 KB | 98 KB | 56x |
| MouseWithoutBorders | 5,034 KB | 98 KB | 51x |
| Workspaces | 4,954 KB | 98 KB | 50x |
| CropAndLock | 4,944 KB | 98 KB | 50x |
| LightSwitch | 4,941 KB | 98 KB | 50x |
| ShortcutGuide | 4,918 KB | 98 KB | 50x |
| PowerAccent | 4,864 KB | 98 KB | 49x |
| PowerOCR | 4,862 KB | 98 KB | 49x |
| CmdNotFound | 4,787 KB | 98 KB | 49x |
| ZoomIt | 4,832 KB | 98 KB | 49x |
| Awake | 4,820 KB | 152 KB | 32x |
| AlwaysOnTop | 369 KB | 142 KB | 3x |
| **DLL Total** | **74,408 KB** | **1,573 KB** | **47x** |

## Application EXEs

| EXE | Original | Rust | Ratio | Status |
|-----|----------|------|-------|--------|
| AlwaysOnTop | 5,800 KB | 178 KB | 33x | ✅ Shipped |
| ActionRunner | 4,628 KB | 110 KB | 42x | ✅ Shipped |
| Update | 5,377 KB | 1,396 KB | 3.9x | ✅ Shipped |
| Awake | 254 KB+.NET | 401 KB | standalone | ✅ Shipped |
| WorkspacesSnapshotTool | C++ | Rust | — | ✅ Built |
| WorkspacesLauncher | C++ | Rust | — | ✅ Built |
| WorkspacesWindowArranger | C++ | Rust | — | ✅ Built |

## Test Results — 515 passing

| Crate | Tests |
|-------|-------|
| 15 module DLLs | 113 |
| powertoys-win32 | 37 |
| fancyzones-core | 130 |
| fancyzones-engine | 46 |
| workspaces-core | 83 |
| Apps + integration | 106 |
| **Total** | **515** |

## Roadmap

### Phase 1+2 — ✅ Complete
- FFI bridge + vtable adapter
- All 15 module DLLs, 4 EXEs ported
- CI pipeline, MSBuild integration
- ShortcutGuide Win key tracking

### Phase 3 — ✅ Complete
- `powertoys-win32` shared crate
- `fancyzones-core` + `fancyzones-engine` crates
- `workspaces-core` crate + 3 Workspaces EXEs
- Shared crate wired into existing apps (removed duplication)

### Future
- ZoomIt (all tiers — after PT core is done)
- Installer end-to-end verification
- ARM64 cross-compilation
