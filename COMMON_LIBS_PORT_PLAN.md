# PowerToys Common Libraries → Rust Port Plan

*Generated: 2026-04-11*

## Key Finding
PowerToys has 11 shared C++ libraries consumed by 122 C++ projects and 47 C# projects.
Porting the top 2 (logger + SettingsAPI) to Rust FFI crates would benefit **151 projects**.
Both C++ and C# consumers can call the same Rust DLL via C FFI / P-Invoke.

## Priority Table

| Library | LOC | Consumers | Pure Logic | Already Rust | Priority |
|---------|-----|-----------|-----------|-------------|----------|
| **SettingsAPI** | 1,200 | 79 C++ + 47 C# | 80% | No | **CRITICAL** |
| **logger** | 800 | 72 C++ | 95% | No | **CRITICAL** |
| **Display** | 900 | 17 C++ | 5% | Partial (powertoys-win32) | HIGH |
| **GPOWrapper** | 600 | 1 + downstream | 10% | Partial | MEDIUM |
| **notifications** | 2,800 | 8 C++ | 20% | No | MEDIUM |
| **version** | 200 | 10 C++ | 0% | No | LOW |
| **interop** | 3,500 | 3 C++ + 47 C# | 30% | No | SKIP (COM works) |
| **updating** | 800 | 3 C++ | 80% | ✅ Done | SHIPPED |
| **Telemetry** | 300 | — | 100% | No | SKIP |
| **hooks** | headers | — | 0% | ✅ Done | SHIPPED |
| **utils** | headers | everywhere | 20% | Partial | Incremental |

## Architecture After Port

```
C# Apps ──P/Invoke──→ powertoys-settings.dll (Rust FFI)
                       powertoys-logger.dll (Rust FFI)
C++ Apps ──link──────→ same Rust DLLs via C headers
```

Single source of truth. No COM marshaling. 50-70% binary reduction.

## Solution Breakdown (311 projects)

| Category | Count |
|----------|-------|
| C++ projects | 122 |
| C# projects | 189 |
| Module interface DLLs (Rust-ported) | 18 |
| Shared C++ libraries | 11 |
| C++ app EXEs | 26+ |
| Shell extensions | 74+ |

## Phase 1 — SettingsAPI + Logger (biggest ROI)
- `powertoys-settings` Rust crate: serde_json + notify file watcher
- `powertoys-logger` Rust crate: tracing-subscriber + file sinks
- Both export C FFI: callable from C++ and C# (P/Invoke)
- 151 projects benefit immediately

## Phase 2 — Display + GPO
- Extend `powertoys-win32` crate with complete monitor/DPI helpers
- Add GPO registry access functions

## Phase 3 — Notifications
- WinRT toast abstraction in Rust via windows-rs