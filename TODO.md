# PowerToys Rust Port — TODO

*Last updated: 2026-04-11 16:06 UTC*

## 🔴 Blocking / In Flight

- [ ] **Installer build** — MSBuild succeeds (296 projects + 23 Rust outputs), blocked by pre-existing DSC Schema Generator COM error (`0x80040154`). Not related to Rust changes.
- [ ] **ARM64 CI** — Rust build hook triggers on ARM64 agent without `rustup`
- [ ] **3 unported C++ module DLLs** — EnvironmentVariables, Hosts, MeasureTool (Screen Ruler)

## 🟡 Known Issues

- [ ] **FancyZones: app zone history** — windows don't remember their zones across sessions
- [ ] **FancyZones: window filtering** — no filtering for system windows, tool windows, excluded apps
- [ ] **FancyZones: display change** — zones not recalculated on monitor connect/disconnect
- [ ] **FancyZones: editor integration** — can't launch editor or reload custom layouts
- [ ] **FancyZones: virtual desktop** — no tracking of virtual desktop switches
- [ ] **ShortcutGuide: custom shortcut** — Win+Shift+/ registered but needs live verification
- [ ] **Workspaces EXEs** — not yet live-tested (capture → launch → arrange cycle)
- [ ] **Installer end-to-end** — not yet tested

## 🟢 Shipped & Working

### Phase 1+2 — Module DLLs + Core EXEs
- [x] FFI bridge (`PowerToyModule` trait + C++ vtable adapter + `on_hotkey_ex`/`get_hotkey_ex`)
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
- [ ] Runner modernization (thin C++ shell + Rust core lib)

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
