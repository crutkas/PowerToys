# PowerToys Rust Port — TODO

*Last updated: 2026-04-11 05:17 UTC*

## 🔴 Blocking / In Flight

- [ ] **Installer build** — MSBuild succeeds (296 projects + 19 Rust outputs), blocked by pre-existing DSC Schema Generator COM error (`0x80040154`). Not related to Rust changes.
- [ ] **ARM64 CI failure** — Rust build hook triggers on ARM64 agent without `rustup`. Need to:
  - Add `rustup` install step to CI pipeline, OR
  - Skip Rust build on ARM64 until cross-compile is set up (`/p:SkipRustBuild=true`)

## 🟡 Needs Verification

- [ ] **Installer end-to-end** — install Rust-built MSI → launch PowerToys → all modules load
- [ ] **ShortcutGuide behavior** — deployed Rust DLL with Win key tracking. Verify: hold Win 900ms = guide, quick tap = no trigger

## 🟢 Ready to Ship (Phase 1+2 Complete)

- [x] FFI bridge (`PowerToyModule` trait + C++ vtable adapter)
- [x] All 15 module interface DLLs ported to Rust (219 tests)
- [x] Awake: DLL + EXE (crutkas/awake submodule, `CREATE_NO_WINDOW`, settings restart)
- [x] AlwaysOnTop: DLL + EXE (rounded corner borders, pin/unpin/opacity)
- [x] ActionRunner EXE (non-elevated process launcher)
- [x] Update EXE (GitHub API, version comparison, MSI/bootstrapper install, 19 tests)
- [x] CI pipeline (GitHub Actions, `workflow_dispatch`, ARM64 target)
- [x] MSBuild integration (`Directory.Build.targets` auto-triggers Rust build)
- [x] `Build-RustModules.ps1` build script
- [x] Spelling allowlist updated
- [x] Progress tracker + plan docs

## 📋 Phase 3 Backlog

### Next up
- [ ] `powertoys-win32` shared Rust crate (window enum, virtual desktop COM, monitor, DPI)
- [ ] Workspaces 3 EXEs (using shared crate, 21 MB → 500 KB)
- [ ] FancyZones core engine → Rust (6,780 LOC, snap logic, layout system)
- [ ] ZoomIt Tier 1 (zoom + draw + break, GDI+ → D2D upgrade)

### Future
- [ ] ZoomIt Tier 2-3 (recording, OCR, panorama)
- [ ] Runner modernization (thin C++ shell + Rust core lib)

## 🧪 Test Coverage Analysis (for porting safety)

### FancyZones — ~42+ tests, ~55% coverage
- ✅ **Well tested:** Zone math, layout calculations, JSON persistence, keyboard snapping, multi-monitor basics
- ⚠️ **Partial:** Window-zone assignment, virtual desktop tracking
- ❌ **No tests:** Window drag detection, DPI scaling, monitor hotplug, resize behavior
- **Verdict:** Core engine (zone math, layout init, snapping) has enough tests to port confidently. Win32 interaction layer needs test investment first.

### Workspaces — 61 tests, ~15-20% coverage
- ✅ **Well tested:** Utility functions (AppUtils 17, JsonUtils 8, StringUtils 8, PwaHelper 7, WorkspacesData 11)
- ❌ **Zero tests:** Window arrangement, app launching, IPC, launch status tracking, command line args
- ⚠️ **19+ tests disabled/commented out** (editor UI, launcher stability issues)
- **Verdict:** Helpers are safe to port. Core logic (arranging windows, launching apps) has no safety net — needs test investment before porting.

## 🚫 Not Porting

- ShortcutGuide EXE (D2D overlay, working fine as C++)
- ZoomIt recording subsystem (keep C++ for Media Foundation)
- Settings UI (already C# WinUI3)
- Preview handlers (already C#)
- PowerToys Run (already C# WPF)

## 📊 Metrics

| Metric | Before | After | 
|--------|--------|-------|
| 15 module DLLs | 74.4 MB | 1.6 MB |
| 4 ported EXEs | 16.1 MB | 2.1 MB |
| Runner RAM (private) | 87.5 MB | 55.6 MB |
| AlwaysOnTop RAM | 39.8 MB | 1.2 MB |
| Awake RAM | ~50 MB | 5.0 MB |
| Rust tests | 0 | 221 |

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
