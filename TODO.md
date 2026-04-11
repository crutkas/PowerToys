# PowerToys Rust Port — TODO

*Last updated: 2026-04-11 07:00 UTC*

## 🔴 Blocking / In Flight

- [ ] **Installer build** — MSBuild succeeds (296 projects + 22 Rust outputs), blocked by pre-existing DSC Schema Generator COM error (`0x80040154`). Not related to Rust changes.
- [ ] **ARM64 CI failure** — Rust build hook triggers on ARM64 agent without `rustup`. Need to:
  - Add `rustup` install step to CI pipeline, OR
  - Skip Rust build on ARM64 until cross-compile is set up (`/p:SkipRustBuild=true`)
- [ ] **ShortcutGuide fix** — was launching EXE on `enable()` instead of on long Win press. Fixed: added `on_hotkey_ex` to trait/vtable/adapter so runner's `AddPressedKeyAction` can toggle the EXE. Needs re-verify.
- [ ] **3 unported C++ module DLLs** — EnvironmentVariables, Hosts, MeasureTool (Screen Ruler) still use C++ interface DLLs. Should port for completeness.

## 🟡 Needs Verification

- [ ] **Installer end-to-end** — install Rust-built MSI → launch PowerToys → all modules load
- [ ] **ShortcutGuide behavior** — re-verify after `on_hotkey_ex` fix: long Win = guide, quick tap = nothing
- [ ] **Workspaces EXEs live test** — deploy snapshot/launcher/arranger, verify capture → launch → arrange cycle
- [ ] **FancyZones engine integration** — wire engine crate into module DLL, verify drag → snap flow

## 🟢 Ready to Ship (Phase 1+2 Complete)

- [x] FFI bridge (`PowerToyModule` trait + C++ vtable adapter)
- [x] All 15 module interface DLLs ported to Rust
- [x] Awake: DLL + EXE (crutkas/awake submodule, `CREATE_NO_WINDOW`, settings restart)
- [x] AlwaysOnTop: DLL + EXE (rounded corner borders, pin/unpin/opacity)
- [x] ActionRunner EXE (non-elevated process launcher)
- [x] Update EXE (GitHub API, version comparison, MSI/bootstrapper install)
- [x] CI pipeline (GitHub Actions, `workflow_dispatch`, ARM64 target)
- [x] MSBuild integration (`Directory.Build.targets` auto-triggers Rust build)
- [x] `Build-RustModules.ps1` build script
- [x] Spelling allowlist updated
- [x] Progress tracker + plan docs

## 🟢 Phase 3 — Complete

- [x] `powertoys-win32` shared crate (37 tests — string, event, process, monitor, window, mutex, settings, rect)
- [x] `fancyzones-core` crate (130 tests — zone math, layout, data, keyboard snap, settings, util)
- [x] `workspaces-core` crate (83 tests — string, app detection, data, JSON, PWA, launch status, window arrange)
- [x] `fancyzones-engine` crate (46 tests — work area, drag handler, engine coordinator, snap, overlay)
- [x] Workspaces 3 EXEs: snapshot, launcher, arranger (using shared crates)
- [x] Shared crate wired into AlwaysOnTop, ActionRunner, Update (removed duplicated code)

### Future
- [ ] ZoomIt (all tiers — zoom, draw, break, recording, OCR, panorama)
- [ ] Runner modernization (thin C++ shell + Rust core lib)

## 🧪 Test Coverage

| Crate | Tests | Coverage |
|-------|-------|----------|
| 15 module DLLs | 113 | Module interface, settings, GPO |
| powertoys-win32 | 37 | String, event, process, monitor, window, mutex, settings, rect |
| fancyzones-core | 130 | Zone math, layout, data/JSON, keyboard snap, device ID, monitor ordering |
| fancyzones-engine | 46 | Work area creation, drag detection, engine coordination |
| workspaces-core | 83 | App detection, data structs, JSON, PWA, launch status, window arrange |
| 4 original apps | 89 | AlwaysOnTop, Awake, Update, integration |
| **Total** | **515** | **0 failures** |

### Still untested
- Window drag detection end-to-end (requires live Win32 environment)
- DPI scaling, monitor hotplug
- IPC between Workspaces launcher ↔ arranger
- Registry queries for packaged apps

## 🚫 Not Porting (keep C++)

- **AlwaysOnTop EXE** — D2D border rendering + DWM blur matches OS perfectly. Rust DLL stays.
- ShortcutGuide EXE (D2D overlay, working fine as C++)
- ZoomIt recording subsystem (keep C++ for Media Foundation)
- Settings UI (already C# WinUI3)
- Preview handlers (already C#)
- PowerToys Run (already C# WPF)
- Runner (thin orchestrator, 70% Win32-entangled, 0% tested — not worth it)

## 📊 Metrics

| Metric | Before | After | 
|--------|--------|-------|
| 15 module DLLs | 74.4 MB | 1.6 MB |
| 4 ported EXEs | 16.1 MB | 2.1 MB |
| Runner RAM (private) | 87.5 MB | 55.6 MB |
| AlwaysOnTop RAM | 39.8 MB | 1.2 MB |
| Awake RAM | ~50 MB | 5.0 MB |
| Rust tests | 0 | 515 |

## 📁 Plan Documents

| Document | What it covers |
|----------|---------------|
| `RUST_PORT_PROGRESS.md` | Shipped metrics, architecture diagram |
| `PHASE3_PLAN.md` | High-level: Rust vs C# AOT vs keep C++ |
| `FANCYZONES_WORKSPACES_PORT_PLAN.md` | 10-week plan, shared crate design |
| `ZOOMIT_PORT_PLAN.md` | Tiered approach, perf benchmarks |

## 🌿 Branches

| Branch | Status |
|--------|--------|
| `rust-awake-module` | **Main working branch** — all shipped work |
| `rust-poweraccent-exe` | Parked — UX needs review |
| `rust-cropandlock-exe` | Parked — UX doesn't match OS styling |
