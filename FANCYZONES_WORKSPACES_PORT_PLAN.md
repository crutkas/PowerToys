# FancyZones + Workspaces Unified Port Plan

*Created: 2026-04-11 | Based on shared code analysis of both modules*

## Why Merge These Ports

FancyZones and Workspaces both manage **windows on monitors** — they share the same
foundational Win32 abstractions but the code is duplicated across two separate libraries:

| Shared Pattern | FancyZones LOC | Workspaces LOC | Unified Rust |
|---------------|----------------|----------------|-------------|
| Window enumeration | Custom EnumWindows | WindowEnumerator.h (29) | ~100 LOC |
| Window filtering | Multiple files | WindowFilter.h (50) | ~80 LOC |
| Virtual desktop COM | VirtualDesktop.cpp (177) | VirtualDesktop.h (40) | ~120 LOC |
| Monitor detection | MonitorUtils.cpp (354) | MonitorUtils.h (42) | ~200 LOC |
| DPI conversion | WindowUtils.cpp (416) | WindowUtils.h (113) | ~150 LOC |
| Window state queries | WindowUtils.cpp | WindowUtils.h | ~100 LOC |
| JSON serialization | JsonHelpers.cpp (662) | JsonUtils.cpp (97) | serde (0 LOC) |
| **Duplicated total** | **~1,600 LOC** | **~370 LOC** | **~750 LOC** |

Building one shared `powertoys-win32` crate eliminates this duplication and makes
both ports faster.

## Current Codebase

### FancyZones (7,213 LOC)
```
FancyZonesLib/          6,780 LOC  — Core snapping engine
├── FancyZones.cpp        1,049    Main orchestrator + event loop
├── JsonHelpers.cpp         662    Layout/config JSON
├── WindowKeyboardSnap.cpp  427    Keyboard zone snapping
├── WindowUtils.cpp         416    Window state, exclusions, DPI
├── LayoutConfigurator.cpp  393    Zone layout creation
├── MonitorUtils.cpp        354    Monitor detection, DPI, WMI
├── trace.cpp               357    ETW telemetry
├── WorkArea.cpp            306    Per-monitor work areas
├── Layout.cpp              302    Zone geometry calculations
├── ZonesOverlay.cpp        291    D2D visual overlay rendering
├── Settings.cpp            244    Settings persistence
├── WindowMouseSnap.cpp     209    Mouse drag zone snapping
├── WindowDrag.cpp          207    Drag interaction state
├── util.cpp                225    Utility functions
├── EditorParameters.cpp    184    Editor communication
├── VirtualDesktop.cpp      177    Virtual desktop tracking
└── (15 more files)         977    Hooks, colors, data types, etc.

FancyZones/               241 LOC  — EXE entry point
FancyZonesModuleInterface/ 192 LOC — Module DLL (already Rust ✅)
```

### Workspaces (4,213 LOC)
```
WorkspacesLib/            2,181 LOC — Shared library
├── WorkspacesData.cpp      469    Data model (projects, apps, monitors)
├── two_way_pipe_ipc.cpp    405    Named pipe IPC protocol
├── AppUtils.cpp            374    App discovery, package detection
├── PwaHelper.cpp           221    Progressive Web App detection
├── SteamGameHelper.cpp     140    Steam game detection
├── LaunchingStatus.cpp     122    Launch state tracking
├── WbemHelper.cpp           99    WMI process queries
├── JsonUtils.cpp            97    JSON file I/O
├── WindowUtils.cpp          88    Window AUMID extraction
└── (8 more files)          166    Trace, CLI, string utils

workspaces-common/         339 LOC — Shared headers
WorkspacesLauncher/        776 LOC — Process orchestrator EXE
WorkspacesSnapshotTool/    298 LOC — Window enumerator EXE
WorkspacesWindowArranger/  609 LOC — Window positioner EXE
WorkspacesModuleInterface/ 308 LOC — Module DLL (already Rust ✅)
```

## Architecture: Hybrid Approach

```
┌─────────────────────────────────────────────────────────┐
│                  SHARED RUST FOUNDATION                   │
│               powertoys-win32 crate (~900 LOC)           │
│                                                          │
│  window::enumerator  — EnumWindows with filter callback  │
│  window::filter      — IsVisible, IsRoot, IsOnDesktop    │
│  window::state       — IsMaximized, GetStyle, GetRect    │
│  window::dpi         — DPI-aware rect conversion         │
│  monitor::display    — HMONITOR enum, work areas         │
│  monitor::dpi        — Per-monitor DPI detection         │
│  virtual_desktop     — IVirtualDesktopManager COM        │
│  ipc::pipe           — Named pipe two-way messaging      │
└───────────┬─────────────────────────────────┬───────────┘
            │                                 │
    ┌───────▼────────┐              ┌─────────▼──────────┐
    │  FancyZones     │              │  Workspaces         │
    │  Rust Core Lib  │              │  Rust Tools         │
    │  (~5,000 LOC)   │              │  (~2,000 LOC)       │
    │                 │              │                     │
    │  Zone layout    │              │  Snapshot (enum)    │
    │  Snap engine    │              │  Launcher (IPC)     │
    │  Work areas     │              │  Arranger (place)   │
    │  Settings       │              │  App discovery      │
    │  Keyboard snap  │              │  PWA/Steam detect   │
    │  Mouse snap     │              │  JSON data model    │
    └───────┬─────────┘              └─────────┬──────────┘
            │                                  │
    ┌───────▼────────┐              ┌──────────▼─────────┐
    │  FancyZones     │              │  Workspaces         │
    │  C# WinUI3 AOT  │              │  C# WinUI3 AOT     │
    │  (~300 LOC)     │              │  (existing UI)      │
    │                 │              │                     │
    │  Zone overlay   │              │  LauncherUI         │
    │  (Composition)  │              │  EditorUI           │
    └─────────────────┘              └────────────────────┘
```

## Phase Plan

### Phase A: Foundation — `powertoys-win32` crate (2 weeks)

Build the shared Rust crate used by both FancyZones and Workspaces:

```
powertoys-win32/
├── src/
│   ├── lib.rs
│   ├── window.rs         — EnumWindows, GetWindowRect, IsVisible, GetStyle
│   ├── window_filter.rs  — FilterPopup, FilterSystem, IsOnCurrentDesktop
│   ├── monitor.rs        — EnumDisplayMonitors, GetMonitorInfo, DPI
│   ├── virtual_desktop.rs — IVirtualDesktopManager COM wrapper
│   ├── dpi.rs            — DPI-aware RECT conversion
│   ├── ipc.rs            — Named pipe two-way IPC (for Workspaces)
│   └── process.rs        — Process path, AUMID, elevation check
└── tests/
    ├── test_window_enum.rs    — Can enumerate desktop windows
    ├── test_monitor.rs        — Detects monitors correctly
    ├── test_virtual_desktop.rs — COM interface works
    ├── test_dpi.rs            — DPI math roundtrips
    └── test_ipc.rs            — Pipe send/receive roundtrip
```

**Tests: ~20** (window enumeration, monitor detection, DPI math, IPC roundtrip)

### Phase B: Workspaces Tools (2 weeks)

Port 3 headless EXEs using `powertoys-win32`:

| EXE | LOC | What it does |
|-----|-----|-------------|
| WorkspacesSnapshotTool | 298 | Enumerate windows → JSON |
| WorkspacesWindowArranger | 609 | Read JSON → SetWindowPlacement |
| WorkspacesLauncher | 776 | Orchestrate app launches via IPC |

**Plus:** Port WorkspacesLib data types to Rust (WorkspacesData, AppUtils, PwaHelper).

**Tests: ~30** (JSON roundtrip, window placement, app matching, IPC protocol)

**Size target:** 21 MB C++ → ~500 KB Rust (3 EXEs combined)

### Phase C: FancyZones Core Engine (4-5 weeks)

Port FancyZonesLib to Rust using `powertoys-win32`:

```
fancyzones-core/
├── src/
│   ├── lib.rs
│   ├── zone.rs           — Zone geometry, RECT operations
│   ├── layout.rs         — Layout creation (grid, canvas, priority grid)
│   ├── layout_config.rs  — Layout templates from settings
│   ├── work_area.rs      — Per-monitor zone management
│   ├── snap_keyboard.rs  — Win+Arrow zone navigation
│   ├── snap_mouse.rs     — Drag-and-drop zone snapping
│   ├── snap_drag.rs      — Drag state tracking
│   ├── assigned.rs       — Window-to-zone tracking
│   ├── settings.rs       — FancyZones settings (JSON)
│   └── ffi.rs            — C ABI exports for the module DLL
└── tests/
    ├── test_zone_geometry.rs    — Zone RECT calculations
    ├── test_layout_grid.rs      — Grid layout generation
    ├── test_layout_canvas.rs    — Canvas layout zones
    ├── test_snap_keyboard.rs    — Win+Arrow navigation logic
    ├── test_snap_mouse.rs       — Zone detection from point
    ├── test_work_area.rs        — Multi-monitor zone assignment
    └── test_settings.rs         — Settings JSON roundtrip
```

**Tests: ~40** (zone geometry, layout generation, snap logic, multi-monitor)

This is where the real testing value comes in — FancyZones currently has C++ unit tests
for zone calculation. We port those AND add coverage for snap logic which is untested today.

### Phase D: FancyZones Overlay → C# WinUI3 AOT (1-2 days)

The EXE itself is 241 LOC — just a message loop + Composition overlay.
Port to C# WinUI3 for native Fluent Design zone rendering.

The Rust `fancyzones-core` lib exports C ABI functions that the C# UI calls
for zone calculations and snap decisions.

## Performance Benchmarks

### FancyZones

| Metric | Current C++ | Rust Target | How to measure |
|--------|------------|-------------|---------------|
| Zone snap latency | ~30ms | ~15ms | ETW: drag event → SetWindowPos |
| Layout calculation | ~5ms | ~2ms | Timer around LayoutConfigurator |
| Settings load | ~20ms | ~5ms | serde vs custom JSON parser |
| Memory (idle) | 57.5 MB priv | ~5 MB | No D2D/GDI+ runtime loaded |
| Memory (active) | 77 MB WS | ~20 MB | Rust core + C# overlay |

### Workspaces

| Metric | Current C++ | Rust Target | How to measure |
|--------|------------|-------------|---------------|
| Snapshot time | ~200ms | ~100ms | Timer around full enum |
| Arrange time | ~500ms | ~200ms | Timer: JSON read → all windows placed |
| Launcher IPC | ~50ms/msg | ~20ms/msg | Pipe roundtrip latency |
| Memory (3 tools) | ~60 MB total | ~15 MB | 3 Rust EXEs combined |

## Existing Test Coverage to Port

### FancyZones C++ Unit Tests (existing)
```
FancyZones.UnitTests.dll — zone calculation tests
  - Zone snapping geometry
  - Layout generation (grid, canvas)
  - Multi-monitor zone assignment
  - DPI conversion accuracy
```

### Workspaces C++ Unit Tests (existing)
```
Workspaces.Lib.UnitTests.dll — 11 tests
  - WorkspacesFile path validation
  - Position.ToRect conversion
  - Application comparison
  - Position comparison
```

All existing tests get ported to Rust `#[test]` and expanded with new coverage.

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|-----------|
| Virtual Desktop COM instability | High | Test on Win10 + Win11 22H2/23H2/24H2 |
| Snap performance regression | High | Benchmark before/after; the math is identical |
| IPC protocol mismatch | Medium | Byte-for-byte protocol compatibility tests |
| FancyZones Editor communication | Medium | Keep existing C# Editor unchanged |
| Multi-monitor DPI edge cases | Medium | Port DPI logic exactly; test on 100%/125%/150%/200% |

## Dependency Graph

```
powertoys-win32 (shared foundation)
    ↑               ↑
    │               │
fancyzones-core  workspaces-lib
    ↑               ↑        ↑         ↑
    │               │        │         │
    │        ws-snapshot  ws-launcher  ws-arranger
    │
fancyzones-module-interface (already Rust ✅)
    +
FancyZones.exe (C# WinUI3 AOT overlay)
```

## Timeline

| Week | Deliverable | Tests |
|------|------------|-------|
| 1-2 | `powertoys-win32` crate | 20 |
| 3-4 | Workspaces 3 EXEs | 30 |
| 5-6 | FancyZones zone/layout engine | 25 |
| 7-8 | FancyZones snap engine | 15 |
| 9 | FancyZones C# overlay + integration | 10 |
| 10 | Performance benchmarks + polish | — |
| **Total** | **10 weeks** | **100 tests** |

## Success Criteria

- [ ] `powertoys-win32` crate with 20+ tests, used by both modules
- [ ] Workspaces: 3 Rust EXEs, 21 MB → 500 KB, all features working
- [ ] FancyZones: Rust core + C# overlay, zone snap works identically
- [ ] FancyZones: port all existing C++ unit tests + add snap coverage
- [ ] Performance: snap latency ≤ C++, memory < 50% of C++
- [ ] No regressions in FancyZones Editor (C#, unchanged)
