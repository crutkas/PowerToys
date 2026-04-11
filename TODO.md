# PowerToys Rust Port — TODO

*Last updated: 2026-04-11 17:40 UTC*

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

### Module Interface DLLs (not yet Rust)
| Module | LOC | Status |
|--------|-----|--------|
| EnvironmentVariables | 293 | ✅ ported |
| Hosts | 300 | ✅ ported |
| MeasureTool (Screen Ruler) | 300 | ✅ ported |

### Mouse Utilities (all C++ → Rust with D2D)
| Component | LOC | Rendering | Recommendation |
|-----------|-----|-----------|---------------|
| **FindMyMouse** DLL | 1,726 | C++: WinRT Composition | ✅ Port fully — D2D radial gradient spotlight (same pattern as AOT) |
| **MouseHighlighter** DLL | 1,134 | C++: Composition | ✅ Port fully — D2D FillEllipse for click circles |
| **MousePointerCrosshairs** DLL | 1,613 | C++: Composition | ✅ Port fully — D2D DrawLine for crosshairs |
| **CursorWrap** DLL | 1,179 | Headless | ✅ Port fully — pure logic, WH_MOUSE_LL hook, no UI |
| **MouseJump** Module Interface | 747 | Headless | ✅ Port — same pattern as other 15 DLLs |
| **MouseJumpUI** EXE | C# | GDI/WinForms | Keep existing UX |

### Other C++ EXEs
| Component | LOC | Rendering | Recommendation |
|-----------|-----|-----------|---------------|
| **LightSwitchService** | 2,179 | Headless | ✅ Port — headless scheduler, registry ops |
| **FileLocksmithCLI** | 2,437 | Headless | ✅ Port — CLI tool, process/handle enumeration |
| **PowerAccentKeyboardService** | 1,500 | Headless + WH_KEYBOARD_LL | ✅ Port — keyboard hook service |
| **CmdPalKeyboardService** | 500 | Headless | ✅ Port — small keyboard hook |
| **CropAndLock** EXE | 1,899 | D2D + DWM Thumbnail | Keep C++ — D2D rendering |
| **ShortcutGuide** EXE | 2,950 | D2D overlay | Keep C++ — D2D overlay |
| **MeasureToolCore** | 5,000 | D2D + DirectX | ✅ Port — D2D DrawLine + DirectWrite (same as FZ overlay) |
| **KeyboardManagerEngine** | 3,000 | Headless + hooks | Possible but complex — defer |

## 🟢 Shipped & Working
- [x] All 15 module interface DLLs (vtable mismatch fixed across all 14 adapters)
- [x] Awake: DLL + EXE (crutkas/awake submodule)
- [x] AlwaysOnTop: DLL (Rust) + EXE (Rust with D2D borders, GPU-accelerated)
- [x] ActionRunner EXE
- [x] Update EXE
- [x] ShortcutGuide DLL: settings-aware (legacy Win press vs custom hotkey), no startup popup
- [x] CI pipeline + MSBuild integration + Build-RustModules.ps1
- [x] Spelling allowlist (2,317 words)

### Phase 3 — Shared Crates + FancyZones + Workspaces
- [x] `powertoys-win32` shared crate (37 tests) — wired into AOT, ActionRunner, Update
- [x] `fancyzones-core` (130 tests) — zone math, layout, data, keyboard snap
- [x] `fancyzones-engine` (46 tests) — work area, drag handler, engine, overlay
- [x] `workspaces-core` (83 tests) — app detection, JSON, launch status, window arrange
- [x] FancyZones EXE — drag-to-snap with overlay, Win+Arrow override, keyboard zone cycling
- [x] Workspaces 3 EXEs — snapshot, launcher, arranger

### Bugs Fixed Today
- [x] Vtable mismatch in all 14 module adapters (on_hotkey_ex/get_hotkey_ex fields)
- [x] FancyZones: overlay not rendering (missing UpdateLayeredWindow)
- [x] FancyZones: Shift-during-drag (was only checked at start, now continuous)
- [x] FancyZones: keyboard snap direction wrong (zone assignment not tracked on drag-snap)
- [x] FancyZones: Win+Arrow override (added WH_KEYBOARD_LL hook)
- [x] ShortcutGuide: launching on startup (on_hotkey missing enabled check)
- [x] ShortcutGuide: on_hotkey_ex added to vtable for long Win press
- [x] AlwaysOnTop: D2D borders replacing GDI/SDF

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
| 15 module DLLs | 74.4 MB | 1.6 MB (47x) |
| AlwaysOnTop EXE | 5,805 KB | 522 KB (11x) |
| FancyZones EXE | ~50 MB WS | 1.9 MB WS |
| AlwaysOnTop RAM | 54 MB WS / 40 MB priv | 7.9 MB / 1.3 MB |
| Awake RAM | ~50 MB | 6.1 MB WS / 0.9 MB priv |
| Runner RAM (private) | 87.5 MB | 47 MB |
| Rust tests | 0 | 515 |
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
