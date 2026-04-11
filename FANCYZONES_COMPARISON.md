# FancyZones C++ vs Rust — Gap Analysis
*Updated: 2026-04-11*

## What's WORKING in Rust
| Feature | Status | Notes |
|---------|--------|-------|
| Drag-to-snap | ✅ | WinEvent hooks detect drag, snap on drop |
| Zone overlay (D2D) | ✅ | D2D + DirectWrite, zone numbers, anti-aliased |
| Zone layout calculation | ✅ | Columns, Rows, Grid, PriorityGrid, Focus |
| Per-monitor work areas | ✅ | Enumerates monitors, creates zones per work area |
| Settings loading | ✅ | shift_drag, colors, opacity, excluded_apps |
| Singleton mutex | ✅ | Prevents duplicate instances |
| Parent PID monitoring | ✅ | Exits when runner exits |
| COM initialization | ✅ | Required for WinEvent hooks |
| Window filtering | ✅ | Skip minimized/tool/invisible/child/excluded/elevated |
| App zone history | ✅ | save/load/restore across sessions |
| Keyboard hook (WH_KEYBOARD_LL) | ✅ | Win+Arrow override, eats key before Windows snap |
| Keyboard zone cycling | ✅ | Win+Arrow snaps to adjacent zones |
| Shift-during-drag | ✅ | Checked continuously via timer, not just at start |
| Virtual desktop tracking | ✅ | Detects switches via EVENT_OBJECT_NAMECHANGE |
| Editor integration | ✅ | Launches editor, reloads layouts on save |
| 130 core logic tests | ✅ | Zone math, layout, data, keyboard snap, util |
| 63 engine tests | ✅ | Work area, drag handler, engine, overlay, window filter |
| 85 C++ MSTest parity tests | ✅ | In FancyZonesTests/UnitTests (wired into solution) |

## Remaining Gaps

### MEDIUM (nice to have, not blocking)
| Feature | C++ | Rust | Notes |
|---------|-----|------|-------|
| Display change handling | ✅ | ❌ | Zones not recalculated on monitor connect/disconnect |
| Multi-zone selection (Ctrl+drag) | ✅ | ❌ | |
| Quick layout switch (Ctrl+Alt+Win+digit) | ✅ | ❌ | |
| Window transparency during drag | ✅ | ❌ | |
| Flash zones on layout switch | ✅ | ❌ | |
| Full settings (25+) | ✅ | ~10 | Missing: move_window_across_monitors, show_zones_on_all_monitors, etc. |

### LOW (cosmetic/edge cases)
| Feature | C++ | Rust |
|---------|-----|------|
| ETW telemetry (trace.cpp) | ✅ | ❌ |
| Move windows based on position | ✅ | ❌ |

## CRITICAL and HIGH gaps: ZERO
All critical and high-priority gaps have been fixed.