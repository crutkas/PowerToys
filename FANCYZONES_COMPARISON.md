# FancyZones C++ vs Rust — Side-by-Side Analysis
*Generated: 2026-04-11*

## What's WORKING in Rust today
| Feature | Status | Notes |
|---------|--------|-------|
| Drag-to-snap | ✅ | WinEvent hooks detect drag, snap on drop |
| Zone overlay | ✅ | Layered window with ARGB, zone highlight on hover |
| Zone layout calculation | ✅ | Columns, Rows, Grid, PriorityGrid, Focus |
| Per-monitor work areas | ✅ | Enumerates monitors, creates zones per work area |
| Settings loading | ✅ | Reads shift_drag, colors, opacity from JSON |
| Singleton mutex | ✅ | Prevents duplicate instances |
| Parent PID monitoring | ✅ | Exits when runner exits |
| COM initialization | ✅ | Required for WinEvent hooks |
| 130 core logic tests | ✅ | Zone math, layout, data, keyboard snap, util |
| 46 engine tests | ✅ | Work area, drag handler, engine coordinator |

## CRITICAL gaps (breaks expected behavior)

### 1. Low-Level Keyboard Hook (WH_KEYBOARD_LL)
**C++**: FancyZones EXE installs its OWN WH_KEYBOARD_LL hook (separate from runner).
Intercepts Win+Arrow BEFORE Windows processes them. Returns 1 to eat the key.

**Rust**: ❌ No keyboard hook at all. Relies only on WinEvent for drag detection.

**Impact**: Win+Arrow snapping doesn't work. "Override Windows snapping" setting has no effect.
Win+Left/Right still triggers Windows' built-in snap instead of FancyZones.

**Fix**: Install SetWindowsHookExW(WH_KEYBOARD_LL) in the Rust FancyZones EXE.
Post WM_APP messages for Win+Arrow, eat the key to prevent Windows snap.

### 2. Missing WinEvent types
**C++** hooks 6 event types + 1 dynamic:
- EVENT_SYSTEM_MOVESIZESTART ✅ (Rust has this)
- EVENT_SYSTEM_MOVESIZEEND ✅ (Rust has this)
- EVENT_OBJECT_NAMECHANGE ❌ (virtual desktop switch detection)
- EVENT_OBJECT_UNCLOAKED ❌ (window restore/show)
- EVENT_OBJECT_SHOW ❌ (new window visible)
- EVENT_OBJECT_CREATE ❌ (new window — triggers app-zone-history restore)
- EVENT_OBJECT_LOCATIONCHANGE ❌ (dynamic, during drag for cursor tracking)

**Impact**: New windows don't auto-snap to remembered zones. Virtual desktop switching not detected.

### 3. Window Filtering
**C++** has extensive filtering in FancyZonesWindowProcessing.cpp:
- Skip minimized, invisible, tool windows, child windows
- Skip non-processable popups (no caption/minimize box)
- Skip excluded apps list
- Skip windows on different virtual desktop
- Skip elevated windows when FZ not elevated

**Rust**: ❌ No filtering. Tries to snap system windows, Start menu, notifications etc.

## HIGH gaps (missing expected features)

### 4. App Zone History
**C++**: Persists app→zone mapping in app-zone-history.json. When app launches, auto-snaps to last zone.
**Rust**: ❌ Not implemented. Windows don't remember their zones.

### 5. Settings — only partial
**C++ loads 25+ settings**. Rust loads ~5 (shift_drag, colors, opacity).
Missing: override_snap_hotkeys, move_window_across_monitors, move_windows_based_on_position,
app_last_zone_move_windows, excluded_apps, show_zones_on_all_monitors, quick_layout_switch, etc.

### 6. Display Change Handling
**C++**: Recalculates zones on WM_DISPLAYCHANGE and WM_SETTINGCHANGE.
**Rust**: ❌ Zones calculated once at startup. Monitor connect/disconnect not handled.

### 7. Editor Integration
**C++**: Launches FancyZonesEditor.exe, reloads layouts when editor saves.
**Rust**: ❌ No editor launch or layout reload.

## MEDIUM gaps (nice to have)

| Feature | C++ | Rust |
|---------|-----|------|
| Virtual desktop tracking | ✅ | ❌ |
| Multi-zone selection (Ctrl+drag) | ✅ | ❌ |
| Quick layout switch (Ctrl+Alt+Win+digit) | ✅ | ❌ |
| Window transparency during drag | ✅ | ❌ |
| Zone number display in overlay | ✅ | ❌ |
| Flash zones on layout switch | ✅ | ❌ |
| Elevated window handling | ✅ | ❌ |

## KEYBOARD HOOK ARCHITECTURE (affects ALL Rust modules)

The runner has a centralized WH_KEYBOARD_LL hook. Modules interact via:
1. **get_hotkeys() + on_hotkey()** — standard hotkeys (Rust: ✅ works)
2. **GetHotkeyEx() + OnHotkeyEx()** — extended hotkeys (Rust: ✅ via on_hotkey_ex, GetHotkeyEx has ABI issue)
3. **keep_track_of_pressed_win_key()** — long press tracking (Rust: ✅ works for ShortcutGuide)
4. **FancyZones/PowerAccent own WH_KEYBOARD_LL** — separate hooks in EXE process (Rust: ❌ MISSING)

### Modules needing their own keyboard hooks:
| Module | C++ has own hook? | Rust status |
|--------|-------------------|-------------|
| FancyZones | ✅ WH_KEYBOARD_LL in EXE | ❌ Missing — causes override snap failure |
| PowerAccent | ✅ WH_KEYBOARD_LL in KeyboardService EXE | ❌ N/A (using C++ EXE) |
| ShortcutGuide | ❌ Uses runner's centralized hook | ✅ Works via keep_track + on_hotkey_ex |

## RECOMMENDED TEST ADDITIONS

### Keyboard hook tests (new test crate)
1. Hotkey struct layout matches C++ (16 bytes, correct field order)
2. HotkeyEx struct layout matches C++ (modifiersMask, vkCode, id)
3. get_hotkeys returns correct count and fills buffer
4. on_hotkey dispatches to correct handler
5. on_hotkey_ex fires for extended hotkey
6. keep_track_of_pressed_win_key returns correct bool per module
7. milliseconds_win_key_must_be_pressed returns correct value per module

### FancyZones engine tests
8. Window filtering: skip minimized, tool, invisible, excluded
9. App zone history: save/load/restore cycle
10. Display change: recalculate zones after monitor change
11. Override snap: keyboard hook eats Win+Arrow when enabled
12. Multi-monitor: zones correct on second monitor

### Integration tests
13. Module DLL load → get_hotkeys → on_hotkey roundtrip
14. ShortcutGuide: keep_track=true when legacy, get_hotkeys when custom
15. All 15 module DLLs: vtable field order matches C++ exactly

## PRIORITY FIX ORDER
1. **Add WH_KEYBOARD_LL to Rust FancyZones EXE** (fixes override snap)
2. **Add window filtering** (prevents snapping system windows)
3. **Add missing WinEvent types** (CREATE, UNCLOAKED for auto-restore)
4. **Add app zone history** (windows remember zones)
5. **Load remaining settings** (excluded_apps, override_snap_hotkeys, etc.)