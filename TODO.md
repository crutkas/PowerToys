# PowerToys Rust Port — TODO

*Last updated: 2026-04-12 06:00 UTC*

## 🔴 Bug Tracker (test-first methodology)

Every fix follows: read C++ → write failing test → implement fix → test passes → live verify.

### CRITICAL
| ID | Component | Bug | C++ Behavior | Rust Behavior | Status |
|----|-----------|-----|-------------|---------------|--------|
| `update-version` | Update EXE | Version hardcoded | Reads from `version_gen.h` compile-time constants | Returns "0.0.1" always | ✅ fixed |

### HIGH
| ID | Component | Bug | C++ Behavior | Rust Behavior | Status |
|----|-----------|-----|-------------|---------------|--------|
| `fz-overlay-d2d` | FancyZones | GDI overlay, no zone numbers | D2D + DirectWrite (GPU, anti-aliased, zone numbers) | ✅ D2D + DirectWrite | ✅ fixed |
| `fz-window-filter` | FancyZones | No window filtering | Skips minimized/tool/invisible/child/excluded/elevated | ✅ Filters matching C++ | ✅ fixed |
| `aot-virtual-desktop` | AlwaysOnTop | No virtual desktop tracking | Hides borders on other desktops | Registry query + stub | 🟡 partial |
| `ws-snapshot-fields` | Workspaces | Missing fields in snapshot | Captures packageFullName, appUserModelId, pwaAppId, isElevated | ✅ Captures elevation + package | ✅ fixed |

### MEDIUM
| ID | Component | Bug | Status |
|----|-----------|-----|--------|
| `aot-sound` | AlwaysOnTop | No sound on pin/unpin (PlaySoundW) | ✅ fixed |
| `aot-file-watcher` | AlwaysOnTop | Settings don't auto-reload on file change | ✅ fixed |
| `aot-system-menu` | AlwaysOnTop | No "Pin/Unpin" in title bar context menu | ✅ fixed |
| `aot-game-mode` | AlwaysOnTop | Pins windows in full-screen games | ✅ fixed |
| `aot-excluded-apps` | AlwaysOnTop | No exclusion list for apps | ✅ fixed |
| `fz-app-history` | FancyZones | Windows forget zones across sessions | ✅ fixed |
| `fz-display-change` | FancyZones | Zones not recalculated on monitor change | open |
| `fz-editor` | FancyZones | Can't launch editor or reload layouts | ✅ fixed |
| `fz-virtual-desktop` | FancyZones | No virtual desktop switch detection | ✅ fixed |
| `update-progress` | Update EXE | No download progress (UI frozen) | ✅ fixed |
| `ws-launcher-elevation` | Workspaces | Elevated apps fail without UAC retry | ✅ fixed |

## 🟡 Infrastructure
- [ ] **Installer build** — blocked by pre-existing DSC COM error
- [ ] **ARM64 CI** — needs rustup on ARM64 agent

## 📋 Unported C++ Components

### Module Interface DLLs — ALL 22 PORTED ✅
All module interface DLLs are Rust. Original 15 + EnvironmentVariables, Hosts,
MeasureTool, CursorWrap, MouseHighlighter, FindMyMouse, Crosshairs.

### Mouse Utilities — ALL PORTED ✅
| Component | Status | Tests |
|-----------|--------|-------|
| **FindMyMouse** DLL + D2D overlay | ✅ ported | 47 Rust + D2D spotlight overlay |
| **MouseHighlighter** DLL + D2D overlay | ✅ ported | 46 Rust + DC render target |
| **MousePointerCrosshairs** DLL + D2D | ✅ ported | 57 Rust |
| **CursorWrap** DLL | ✅ ported | 60 Rust + 16 C++ MSTest |
| **MeasureTool** DLL + EXE | ✅ ported | 53 Rust |
| **MouseJump** Module Interface | ✅ ported | — |
| **MouseJumpUI** EXE | C# (keep) | — |

### Service EXEs — ALL PORTED ✅
| Component | Status | Tests |
|-----------|--------|-------|
| **LightSwitchService** | ✅ ported | 48 Rust + 46 C++ MSTest |
| **FileLocksmithCLI** | ✅ ported | 24 Rust + 33 C++ MSTest |
| **PowerAccentKeyboardService** | ✅ ported | 40 Rust + 32 C++ MSTest |

### Remaining (not porting)
| Component | LOC | Reason |
|-----------|-----|--------|
| **CmdPalKeyboardService** | 500 | Small keyboard hook — low priority |
| **CropAndLock** EXE | 1,899 | D2D + DWM Thumbnail — plan written |
| **ShortcutGuide** EXE | 2,950 | D2D overlay — keep C++ |
| **KeyboardManagerEngine** | 3,000 | Complex hooks — defer |

## 🟢 Shipped & Working

### Phase 1-2 — Module DLLs + Core EXEs
- [x] All 22 module interface DLLs (vtable mismatch fixed across all adapters)
- [x] Awake: DLL + EXE
- [x] AlwaysOnTop: DLL + EXE (D2D borders, GPU-accelerated)
- [x] ActionRunner EXE
- [x] Update EXE (version from registry/PE, download progress)
- [x] ShortcutGuide DLL: settings-aware (legacy Win press vs custom hotkey)
- [x] CI pipeline + MSBuild integration + Build-RustModules.ps1

### Phase 3 — Shared Crates + FancyZones + Workspaces
- [x] `powertoys-win32` shared crate (37 tests) — wired into AOT, ActionRunner, Update
- [x] `fancyzones-core` (130 tests) — zone math, layout, data, keyboard snap
- [x] `fancyzones-engine` (63 tests) — work area, drag handler, engine, overlay, window filter
- [x] `workspaces-core` (83 tests) — app detection, JSON, launch status, window arrange
- [x] FancyZones EXE — drag-to-snap, Win+Arrow override, keyboard zone cycling, D2D overlay
- [x] Workspaces 3 EXEs — snapshot, launcher, arranger

### Phase 4 — Mouse Utilities + Services
- [x] `findmymouse-core` (47 tests) + D2D spotlight overlay
- [x] `highlighter-core` (46 tests) + D2D overlay (DC render target + UpdateLayeredWindow)
- [x] `crosshairs-core` (57 tests) + D2D crosshair overlay
- [x] `cursorwrap-core` (60 tests) + module DLL
- [x] `measuretool-core` (53 tests) + EXE
- [x] `poweraccent-core` (40 tests) + keyboard service EXE
- [x] `filelocksmith-core` (24 tests) + CLI EXE
- [x] `lightswitch-core` (48 tests) + service EXE

### Phase 5 — Shared Libraries + Runner Core
- [x] `powertoys-settings-ffi` (43 tests) — SettingsAPI replacement with C FFI
- [x] `powertoys-logger-ffi` (18 tests) — Logger replacement with C FFI
- [x] `runner-core` (41 tests) — hotkey conflict detection, settings parsing, shortcuts

### C++ MSTest Parity Tests (all in solution, all pass)
- [x] FancyZones.Tests.cpp (85) in FancyZonesTests/UnitTests
- [x] HotkeyConflictTests.cpp (15) in runner/UnitTests
- [x] TopologyTests.cpp (16) in CursorWrap/UnitTests
- [x] HighlighterTests.cpp (14) in MouseHighlighter/UnitTests
- [x] CrosshairsTests.cpp (25) in MousePointerCrosshairs/UnitTests
- [x] FindMyMouseTests.cpp (22) in FindMyMouse/UnitTests — shake detection + activation guard
- [x] MeasureToolTests.cpp (39) in MeasureTool/UnitTests — edge detection, unit conversion, BGRA
- [x] LightSwitchTests.cpp (48) in LightSwitch/UnitTests
- [x] PowerAccentTests.cpp (40) in poweraccent/UnitTests — includes OSK repeat issue #36853
- [x] FileLocksmithTests.cpp (38) in FileLocksmith/UnitTests

### Future
- [ ] ZoomIt (all tiers — after PT core is done)

### Runner Modernization (queued after bug fixes)
- [x] Analyze runner ~30% pure logic (hotkey conflict 471 LOC, settings 588 LOC)
- [x] Extract hotkey conflict detection to `runner-core` Rust crate (18 tests)
- [x] Extract settings parsing to `runner-core` Rust crate (13 tests)
- [ ] Wire `runner-core` into C++ runner via FFI (runner stays C++, calls Rust for logic) — crate ready, FFI bridge TODO

## 🚫 Not Porting (keep C++)

- ShortcutGuide EXE (D2D overlay, working fine)
- ZoomIt recording subsystem (Media Foundation)
- Settings UI (C# WinUI3)
- Preview handlers (C#)
- PowerToys Run (C# WPF)
- Runner (thin orchestrator, 70% Win32-entangled)

## 📊 Metrics

| Metric | Before | After |
|--------|--------|-------|
| Module DLLs | 22 × C++ (74+ MB) | 22 × Rust (1.6 MB, 47x) |
| AlwaysOnTop EXE | 5,805 KB | 522 KB (11x) |
| FancyZones EXE | ~50 MB WS | 1.9 MB WS |
| AlwaysOnTop RAM | 54 MB WS / 40 MB priv | 7.9 MB / 1.3 MB |
| Awake RAM | ~50 MB | 6.1 MB WS / 0.9 MB priv |
| Runner RAM (private) | 87.5 MB | 47 MB |
| Rust core logic crates | 0 | 15 |
| Rust tests | 0 | 858 |
| C++ MSTest parity tests (new) | 0 | 340 |
| App EXEs (Rust) | 0 | 10+ |
| Release opt level | size (z) | **performance (3)** |

## 📁 Documents

| Document | What it covers |
|----------|---------------|
| `RUST_PORT_PROGRESS.md` | Shipped metrics, architecture |
| `FANCYZONES_COMPARISON.md` | C++ vs Rust gap analysis |
| `PHASE3_PLAN.md` | Rust vs C# AOT vs keep C++ |
| `FANCYZONES_WORKSPACES_PORT_PLAN.md` | Shared crate design |
| `ZOOMIT_PORT_PLAN.md` | Tiered approach |

## 🌿 Branches

| Branch | Status |
|--------|--------|
| `rust-awake-module` | **Main working branch** — all shipped work |
| `rust-poweraccent-exe` | Parked — UX needs review |
| `rust-cropandlock-exe` | Parked — UX doesn't match OS styling |
