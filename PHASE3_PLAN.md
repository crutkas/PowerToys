# PowerToys Modernization — Phase 3 Upgrade Plan

*Created: 2026-04-11 | Based on codebase analysis*

## Strategy: Right Tool for Each Job

| Layer | Technology | Why |
|-------|-----------|-----|
| **Module DLLs (15)** | Rust | ✅ Done. 47x smaller, 36% less RAM |
| **Headless EXEs** | Rust | ✅ Done. ActionRunner, Update, Awake, AlwaysOnTop |
| **UI-heavy EXEs** | C# WinUI3 + AOT | Native Fluent Design, Composition API, matches Settings UI |
| **D2D overlay EXEs** | Keep C++ | ShortcutGuide — D2D perf-critical, not worth porting |
| **Runner** | C++ (Rust core lib later) | Risky to port — every module depends on it |

## Immediate Wins (< 1 week)

### 1. Enable CmdPal AOT (4 hours)
CmdPal.UI.csproj already has AOT config commented out — uncomment 3 lines.
```xml
<PublishAot>true</PublishAot>
```
PowerDisplay.csproj is the working template. RAM drops ~50%.

### 2. FancyZones EXE → C# AOT (1-2 days)
The FancyZones EXE is only **9 KB of source** — it's just a message loop host.
The heavy lifting is in FancyZonesLib (already loaded as a DLL).
Port to C# WinUI3 for native Composition overlay, AOT for fast startup.

### 3. CropAndLock → C# AOT (3-4 days)
The C++ version uses `Windows.UI.Composition` which has perfect C# bindings.
`DwmRegisterThumbnail` and `SetParent` are simple P/Invoke calls.
Result: native Fluent overlay that matches the OS perfectly.

## Medium-term (2-4 weeks)

### 4. Workspaces Tools (3 EXEs)
~4,200 LOC total across WorkspacesLib + 3 EXEs. All headless.
Key challenge: 470 LOC IPC pipe protocol + 370 LOC app discovery.

Recommended approach:
1. Create `powertoys-win32` shared Rust crate (~900 LOC)
   - Window enumeration, virtual desktop COM, monitor detection, DPI
2. Port WorkspacesLib data types (WorkspacesData, JsonUtils)
3. Port the 3 EXEs using the shared crate

### 5. Shared `powertoys-win32` Crate
Both Workspaces and FancyZones share Win32 patterns:

| Abstraction | Used by | Rust LOC |
|-------------|---------|----------|
| Window enumeration | Both | ~100 |
| Virtual desktop COM | Both | ~120 |
| Window state queries | Both | ~150 |
| Monitor/DPI detection | Both | ~350 |
| Window filtering | Both | ~80 |
| **Total** | | **~900** |

## Long-term (2-6 months)

### 6. FancyZones Core → Rust
6,780 LOC in FancyZonesLib. The snapping engine, layout system,
and window management are all pure Win32 — good Rust candidates.
The overlay rendering would stay C# (from step 2 above).

Estimated: 12-16 weeks for full port with tests.

### 7. Runner Modernization
5,144 LOC. Key subsystems:
- Module loading (LoadLibrary + vtable) — 578 LOC
- Tray icon + Quick Access — 470 LOC
- Centralized keyboard hook — 543 LOC  
- Settings IPC — 874 LOC
- General settings — 500 LOC

Options:
a) **Thin C++ shell + Rust core lib** — lowest risk, highest reuse
b) **Full Rust port** — clean but risky (every module depends on it)
c) **Keep C++** — just optimize and clean up

Recommendation: Option (a) — extract core logic to Rust, keep the Win32
message loop and tray icon in C++ as a thin host.

## What NOT to Port

| Component | Reason |
|-----------|--------|
| ShortcutGuide EXE | D2D overlay, perf-critical, working fine |
| ZoomIt EXE | 11 MB, complex D2D drawing/recording, external code |
| Settings UI | Already C# WinUI3, already good |
| Preview handlers | Already C#, small and working |
| PowerToys Run | Already C# WPF, massive plugin ecosystem |

## Size Impact Projection

| Component | Current | After Phase 3 | Savings |
|-----------|---------|---------------|---------|
| 15 module DLLs | 74.4 MB | 1.6 MB (Rust) | 97% |
| Headless EXEs | 16 MB | 2.1 MB (Rust) | 87% |
| UI EXEs (AOT) | 18 MB | ~2 MB (C# AOT) | 89% |
| Workspaces (3 EXEs) | 21 MB | ~500 KB (Rust) | 98% |
| **Total reduction** | **~130 MB** | **~6 MB** | **95%** |

## RAM Impact Projection

| Process | Current | After | Method |
|---------|---------|-------|--------|
| Runner (all modules) | 87.5 MB priv | 55.6 MB | Rust DLLs ✅ |
| AlwaysOnTop | 39.8 MB priv | 1.2 MB | Rust EXE ✅ |
| Awake | ~50 MB WS | 5.0 MB | Rust EXE ✅ |
| FancyZones | 57.5 MB priv | ~12 MB | C# AOT |
| CropAndLock | 44.9 MB priv | ~12 MB | C# AOT |
| Workspaces tools | ~60 MB total | ~15 MB | Rust EXEs |
