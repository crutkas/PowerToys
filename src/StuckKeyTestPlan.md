# Stuck Key Bug — Comprehensive Test Plan

**Author:** Test Engineering  
**Scope:** `centralized_kb_hook.cpp`, `KeyboardEventHandlers.cpp`, `KeyboardListener.cpp` (PowerAccent)  
**Objective:** Verify that no keyboard key remains logically "pressed" after any user interaction sequence across all three keyboard subsystems.

---

## Table of Contents

1. [Module A: Centralized Keyboard Hook](#module-a-centralized-keyboard-hook)
2. [Module B: Keyboard Manager Event Handlers](#module-b-keyboard-manager-event-handlers)
3. [Module C: PowerAccent Keyboard Listener](#module-c-poweraccent-keyboard-listener)
4. [Cross-Module Interference Tests](#cross-module-interference-tests)
5. [Automated Repro Scenarios](#automated-repro-scenarios)
6. [Stuck Key Recovery Mechanism Design](#stuck-key-recovery-mechanism-design)

---

## Verification Method (All Tests)

After every test case, verify keys are not stuck:

```
1. Open Notepad. Type "hello" — confirm no unexpected modifiers applied (e.g., "HELLO" means Shift is stuck).
2. Press and release Win key alone — Start menu should open and close. If it doesn't toggle, Win is stuck.
3. Press and release Alt alone — focus should briefly shift to menu bar. If not, Alt is stuck.
4. Run programmatic check:
     for each vk in {VK_LWIN, VK_RWIN, VK_LCONTROL, VK_RCONTROL, VK_LSHIFT, VK_RSHIFT, VK_LMENU, VK_RMENU}:
         assert (GetAsyncKeyState(vk) & 0x8000) == 0
```

---

## Module A: Centralized Keyboard Hook

### Architecture Summary

- Single `WH_KEYBOARD_LL` hook dispatches to registered modules via `hotkeyDescriptors` multiset.
- Tracks last pressed key in `vkCodePressed` (non-atomic DWORD, reset to `0x100` on KEYUP).
- Uses `SetTimer`/`KillTimer` for pressed-key-hold actions (e.g., hold Win to activate PowerToys Run).
- Modifier state sampled via `GetAsyncKeyState()` on each KEYDOWN.

### A-1: Hotkey Fires During Rapid Modifier Tap

| Field | Value |
|-------|-------|
| **ID** | `HOOK-STUCK-01` |
| **Priority** | P0 |
| **Preconditions** | PowerToys Run enabled (Win hotkey registered). No other remappings. |
| **Sequence** | 1. Tap Win key rapidly 5 times in < 500ms. |
| **Expected** | Start menu toggles on odd taps. No key stuck. |
| **Failure** | Win key stuck down — Start menu won't open, all subsequent keypresses act as Win+key combos. |
| **Root Cause** | Hotkey match on Win KEYDOWN injects dummy `0xFF KEYUP` (line 176-180) to suppress Start menu, but if the real Win KEYUP arrives before the dummy is processed, `GetAsyncKeyState(VK_LWIN)` returns "pressed" on the next cycle. |
| **Verify** | `GetAsyncKeyState(VK_LWIN) & 0x8000 == 0` after sequence completes. Type in Notepad. |

### A-2: Timer Hash Collision Between Modules

| Field | Value |
|-------|-------|
| **ID** | `HOOK-STUCK-02` |
| **Priority** | P1 |
| **Preconditions** | Two modules registered with `AddPressedKeyAction` for the same virtual key (e.g., VK_LWIN). Module names hash to same upper 16-bit value. |
| **Sequence** | 1. Press and hold VK_LWIN for 500ms. 2. Release VK_LWIN. |
| **Expected** | Both modules' actions fire (or the correct one fires). Timer cleaned up on release. |
| **Failure** | `KillTimer` kills wrong module's timer. One module's pressed-key action never fires or fires for the wrong module. Timer leaks and fires repeatedly. |
| **Root Cause** | Timer ID = `(hash(moduleName) & 0xFFFF) << 16 | (vk & 0xFFFF)`. Two modules with same hash + same VK = identical timer ID. `SetTimer` replaces the first timer silently. |
| **Verify** | Confirm both module callbacks executed. No orphaned timers (check with Spy++ or timer enumeration). |

### A-3: vkCodePressed Race — Simultaneous KEYDOWN Events

| Field | Value |
|-------|-------|
| **ID** | `HOOK-STUCK-03` |
| **Priority** | P1 |
| **Preconditions** | Two pressed-key actions registered (e.g., VK_LWIN and VK_APPS). |
| **Sequence** | 1. Press VK_LWIN. 2. Before the timer fires (~300ms), press VK_APPS. 3. Release VK_APPS. 4. Release VK_LWIN. |
| **Expected** | VK_LWIN timer killed when VK_APPS pressed (line 104-105). VK_APPS timer started. Both release cleanly. |
| **Failure** | `vkCodePressed` overwritten to VK_APPS. On VK_LWIN release, `vkCodePressed != VK_LWIN` so timer for VK_LWIN is not killed. Timer fires after key is physically released → phantom action. |
| **Root Cause** | `vkCodePressed` is a single DWORD tracking only the LAST pressed key. Multi-key press overwrites it. Release of earlier key fails the `vkCodePressed == key` check. |
| **Verify** | No phantom PowerToys Run activation after releasing all keys. |

### A-4: Hook Callback Exception Crashes Hook Chain

| Field | Value |
|-------|-------|
| **ID** | `HOOK-STUCK-04` |
| **Priority** | P2 |
| **Preconditions** | A module registers a hotkey action that throws an exception (simulate via fault injection). |
| **Sequence** | 1. Press the registered hotkey. |
| **Expected** | Exception caught gracefully; hook continues operating. |
| **Failure** | Uncaught exception propagates through `KeyboardHookProc`. Windows unhooks the callback. ALL keyboard processing ceases — every key that was down at crash time remains stuck. |
| **Root Cause** | Line 173: `action()` called with no try-catch. |
| **Verify** | Hook is still installed (`hHook != NULL`). Type in Notepad to confirm keyboard works. |

### A-5: KEYUP Arrives for Key Not in pressedKeyDescriptors

| Field | Value |
|-------|-------|
| **ID** | `HOOK-STUCK-05` |
| **Priority** | P2 |
| **Preconditions** | Module cleared via `ClearModuleHotkeys` while a key is physically pressed. |
| **Sequence** | 1. Hold VK_LWIN (timer starts). 2. Module unloads → `ClearModuleHotkeys` removes all descriptors. 3. Release VK_LWIN. |
| **Expected** | Release processed gracefully, no crash, timer killed. |
| **Failure** | Orphaned timer fires after descriptors removed. `PressedKeyTimerProc` iterates empty set or finds stale data. |
| **Root Cause** | `KillTimer` at line 134 iterates `pressedKeyDescriptors` — if cleared between KEYDOWN and KEYUP, the timer ID is unknown and cannot be killed. |
| **Verify** | No timer-related crash. `vkCodePressed` reset to `0x100`. |

---

## Module B: Keyboard Manager Event Handlers

### Architecture Summary

- `HandleSingleKeyRemapEvent`: Remaps individual keys (A→B, A→Ctrl+C, A→disabled).
- `HandleShortcutRemapEvent`: 6-case state machine for shortcut remapping.
- State tracked in `RemapShortcut` struct: `isShortcutInvoked`, `modifierKeysInvoked`.
- Uses `KEYBOARDMANAGER_INJECTED_FLAG` to prevent recursive self-processing.
- AltGr special case: RightAlt sends phantom LCtrl, requires careful modifier handling.

### B-1: Single Key Remap — KEYUP Lost During Key-to-Shortcut

| Field | Value |
|-------|-------|
| **ID** | `KBM-STUCK-01` |
| **Priority** | P0 |
| **Preconditions** | KBM enabled. Single key remap: `A → Ctrl+C`. |
| **Sequence** | 1. Press A (sends Ctrl DOWN, C DOWN). 2. While A held, press and release B. 3. Release A (should send C UP, Ctrl UP). |
| **Expected** | Ctrl+C fires on A press. B types normally. On A release, Ctrl UP and C UP sent. No keys stuck. |
| **Failure** | Ctrl remains stuck down. All subsequent typing produces Ctrl+key combos (Ctrl+H opens Find/Replace instead of typing 'h'). |
| **Root Cause** | If `SendVirtualInput` for the KEYUP sequence fails silently (returns fewer events than expected), Ctrl never gets a KEYUP. The `Input::SendVirtualInput` logs an error but doesn't retry. |
| **Verify** | `GetAsyncKeyState(VK_CONTROL) & 0x8000 == 0`. Type "hello" in Notepad — should appear as "hello", not trigger shortcuts. |

### B-2: Shortcut Remap — Case 1: Modifier Released Before Action Key

| Field | Value |
|-------|-------|
| **ID** | `KBM-STUCK-02` |
| **Priority** | P0 |
| **Preconditions** | KBM enabled. Shortcut remap: `Ctrl+A → Win+E`. |
| **Sequence** | 1. Press Ctrl. 2. Press A (triggers remap: Win DOWN, E DOWN). 3. Release Ctrl (Case 1 triggers). 4. Release A. |
| **Expected** | Case 1: E UP sent, Win UP sent, Ctrl set back to physical state. Case 3: cleanup. |
| **Failure** | Win key stuck down. After releasing everything, pressing any key acts like Win+key. Desktop shortcuts activate unexpectedly. |
| **Root Cause** | Case 1 (line 642) sends action key UP then modifier UPs. If Case 1 processing is interrupted by Case 3 arriving very quickly (fast typist), the state machine may skip the modifier cleanup. `isShortcutInvoked` reset to false before all UPs sent. |
| **Verify** | `GetAsyncKeyState(VK_LWIN) & 0x8000 == 0`. Press Win alone — Start menu should toggle. |

### B-3: Shortcut Remap — Case 5: Unexpected Key During Active Remap

| Field | Value |
|-------|-------|
| **ID** | `KBM-STUCK-03` |
| **Priority** | P0 |
| **Preconditions** | KBM enabled. Shortcut remap: `Ctrl+A → Ctrl+B`. |
| **Sequence** | 1. Press Ctrl. 2. Press A (triggers remap: Ctrl stays, B DOWN). 3. While holding Ctrl+A, press C. 4. Release C. 5. Release A. 6. Release Ctrl. |
| **Expected** | Case 5: keyboard reverted to physical state (Ctrl down, C down). C types with Ctrl (Ctrl+C = copy). Then normal release. |
| **Failure** | B key stuck down. Remap state (`isShortcutInvoked`) not properly reset. Every subsequent activation of Ctrl+A sends B DOWN again without a matching B UP. |
| **Root Cause** | Case 5 (lines 899-927) sends B UP and reverts modifiers, but if the target shortcut's action key matches Ctrl (modifier overlap), the revert logic may not send B UP because it considers Ctrl already "correct". |
| **Verify** | Hold Ctrl+A again after test — should cleanly trigger Ctrl+B without double-B. |

### B-4: AltGr Remap — Phantom LCtrl Stuck

| Field | Value |
|-------|-------|
| **ID** | `KBM-STUCK-04` |
| **Priority** | P0 |
| **Preconditions** | KBM enabled. Shortcut using RAlt (AltGr) as modifier. Non-US keyboard layout with AltGr. Remap: `RAlt+E → Win+D`. |
| **Sequence** | 1. Press RAlt (OS also sends phantom LCtrl DOWN). 2. Press E (triggers remap). 3. Release E. 4. Release RAlt. |
| **Expected** | AltGr handling detects `isAltRightKeyInvoked` flag. LCtrl UP properly sent on cleanup. |
| **Failure** | LCtrl stuck down. The `isAltRightKeyInvoked` flag prevents normal Ctrl release path (lines 647-655, 671-678). If the flag isn't cleared on RAlt release, LCtrl never gets KEYUP. |
| **Root Cause** | Static `isAltRightKeyInvoked` flag (line 621) persists across invocations. If RAlt KEYUP arrives but Case 1 code skips the Ctrl UP due to the flag, Ctrl remains logically pressed. |
| **Verify** | `GetAsyncKeyState(VK_LCONTROL) & 0x8000 == 0`. Type in Notepad — letters should appear normally, not trigger Ctrl+key shortcuts. |

### B-5: Chord Shortcut — Second Key Never Pressed

| Field | Value |
|-------|-------|
| **ID** | `KBM-STUCK-05` |
| **Priority** | P1 |
| **Preconditions** | KBM enabled. Chord remap: `Ctrl+A, B → Ctrl+X`. |
| **Sequence** | 1. Press Ctrl+A (chord started, `chordStarted = true`). 2. Release A. 3. Press C instead of B. 4. Release Ctrl. |
| **Expected** | Chord detection resets. C typed normally with Ctrl (= Ctrl+C). State fully cleaned. |
| **Failure** | `chordStarted` remains true indefinitely. Next time Ctrl+A is pressed, the chord continues from stale state and may fire unexpectedly or skip the first key. |
| **Root Cause** | `ResetChordsIfNeeded` (line 272) is called at top of `HandleShortcutRemapEvent`, but only resets if certain conditions met (lines 1108-1111). If the wrong key is pressed and the chord state isn't in the right condition, it persists. |
| **Verify** | Press Ctrl+A, B — should trigger Ctrl+X cleanly. No stale chord state. |

### B-6: Shortcut Remap — Case 4: Duplicate Modifier Suppression

| Field | Value |
|-------|-------|
| **ID** | `KBM-STUCK-06` |
| **Priority** | P1 |
| **Preconditions** | KBM enabled. Remap: `LCtrl+A → RCtrl+B`. User has only one Ctrl key used. |
| **Sequence** | 1. Press LCtrl. 2. Press A (remap fires: RCtrl DOWN, B DOWN). 3. OS generates key-repeat for LCtrl (physical repeat). 4. Release A. 5. Release LCtrl. |
| **Expected** | Case 4 suppresses the repeated LCtrl (line 841). Full cleanup on release. |
| **Failure** | Case 4 returns 1 (suppress), but the suppression means the OS never sees the "re-press" — if RCtrl was meant to replace LCtrl, the suppression may cause the target Ctrl state to be inconsistent. |
| **Verify** | `GetAsyncKeyState(VK_LCONTROL) & 0x8000 == 0` AND `GetAsyncKeyState(VK_RCONTROL) & 0x8000 == 0`. |

### B-7: Single Key Remap to Disabled — Rapid Toggle

| Field | Value |
|-------|-------|
| **ID** | `KBM-STUCK-07` |
| **Priority** | P1 |
| **Preconditions** | KBM enabled. Remap: `CapsLock → Disabled`. |
| **Sequence** | 1. Press CapsLock rapidly 10 times. 2. Check CapsLock LED and state. |
| **Expected** | All CapsLock events suppressed. LED doesn't toggle. |
| **Failure** | One KEYDOWN gets through while KEYUP is suppressed (or vice versa). CapsLock toggles to ON and subsequent KEYDOWN is suppressed, so it can never be turned OFF. |
| **Root Cause** | Race between the hook returning 1 (suppress) and the OS processing the key toggle. If hook response is delayed, the toggle may process before suppression. |
| **Verify** | CapsLock LED is OFF. Type "hello" — appears lowercase. |

### B-8: Shortcut-to-Text Remap — Modifier Not Released

| Field | Value |
|-------|-------|
| **ID** | `KBM-STUCK-08` |
| **Priority** | P0 |
| **Preconditions** | KBM enabled. Remap: `Ctrl+Shift+T → "hello"` (text output). |
| **Sequence** | 1. Press Ctrl+Shift+T (remap fires, types "hello"). 2. Release T. 3. Release Shift. 4. Release Ctrl. |
| **Expected** | "hello" typed. Ctrl and Shift released cleanly. |
| **Failure** | The text injection sends key events for 'h','e','l','l','o' but Ctrl and Shift are still logically pressed (the remap sends the text but doesn't release source modifiers first). Result: Ctrl+Shift+H, Ctrl+Shift+E... — gibberish or shortcut activations instead of "hello". |
| **Root Cause** | Text remap path (lines 532-546) calls `Helpers::SetTextKeyEvents` but may not first send modifier UP events for source shortcut modifiers. |
| **Verify** | Open Notepad. Press Ctrl+Shift+T. Verify "hello" appears. No modifier stuck after. |

### B-9: App-Specific Remap — Foreground App Switch Mid-Shortcut

| Field | Value |
|-------|-------|
| **ID** | `KBM-STUCK-09` |
| **Priority** | P1 |
| **Preconditions** | KBM enabled. App-specific remap for "notepad.exe": `Ctrl+S → Ctrl+Shift+S`. |
| **Sequence** | 1. Notepad in foreground. Press Ctrl+S (remap fires: Ctrl+Shift+S). 2. While still holding Ctrl+S, Alt+Tab to switch to another app. 3. Release all keys. |
| **Expected** | Remap state for the notepad-specific shortcut cleaned up. No stuck Shift in the new app. |
| **Failure** | Shift stuck down in the new foreground app. The app-specific remap sent Shift DOWN, but the KEYUP handling checks the foreground app and skips cleanup because the new app doesn't have a matching remap. |
| **Root Cause** | App-specific shortcut handler uses `GetForegroundProcess()` to scope remaps. If app changes between KEYDOWN and KEYUP, the KEYUP may not find the active remap and skip the modifier release path. |
| **Verify** | In the new app, type "hello" — should be lowercase. `GetAsyncKeyState(VK_SHIFT) & 0x8000 == 0`. |

---

## Module C: PowerAccent Keyboard Listener

### Architecture Summary

- Separate `WH_KEYBOARD_LL` hook (NOT using centralized hook).
- State machine: idle → letter held → trigger pressed (toolbar visible) → accent selected → reset.
- Tracks `letterPressed`, `m_toolbarVisible`, trigger booleans.
- Uses `GetAsyncKeyState` to verify letter is still physically held before showing toolbar.
- Time-based "false start" detection: if letter held < `inputTime`, outputs trigger key instead.

### C-1: Space Stuck After Accent Selection

| Field | Value |
|-------|-------|
| **ID** | `PA-STUCK-01` |
| **Priority** | P0 |
| **Preconditions** | PowerAccent enabled. Activation key = Space. |
| **Sequence** | 1. Hold 'a'. 2. Press Space (toolbar appears). 3. Press Space again (cycles to next accent). 4. Release 'a' (accent selected and typed). 5. Type normally. |
| **Expected** | Accent character ('à' or 'á') typed. Space key not stuck. Subsequent typing works normally. |
| **Failure** | Space was suppressed (return true) during accent selection but never given a KEYUP. OS thinks Space is still held. Next keypress in some apps may behave as if Space is held (repeated spaces or space+key combos). |
| **Root Cause** | `OnKeyDown` returns true for Space (line 232), suppressing it. The KEYUP for Space goes through `OnKeyUp` which only handles letter keys (lines 234-286). Space KEYUP is not intercepted, so `CallNextHookEx` passes it through — but some apps may have already recorded Space as "down" from the hook suppression mismatch. |
| **Verify** | After accent selection, type "test" in Notepad. Should appear as "test" with no extra spaces. `GetAsyncKeyState(VK_SPACE) & 0x8000 == 0`. |

### C-2: Letter Key Stuck After Rapid False Start

| Field | Value |
|-------|-------|
| **ID** | `PA-STUCK-02` |
| **Priority** | P0 |
| **Preconditions** | PowerAccent enabled. Activation key = Space. `inputTime` = 200ms (default). |
| **Sequence** | 1. Press 'e'. 2. Immediately press Space (< 200ms after 'e'). 3. Immediately release 'e' (< 200ms elapsed). |
| **Expected** | False start detected. Toolbar hidden. Space character typed (trigger key output). 'e' typed before Space. |
| **Failure** | The 'e' KEYDOWN was suppressed (toolbar was briefly shown), but the false-start handler outputs the trigger key (Space) without re-injecting the original 'e' KEYDOWN. User loses the 'e' character. Worse: if the handler doesn't clean up `letterPressed`, subsequent Space presses are suppressed. |
| **Root Cause** | Lines 261-278: false start path outputs trigger key but `letterPressed` was already consumed. The original 'e' KEYDOWN was passed through (not suppressed), but the timing race means the toolbar briefly activated and suppressed intermediate keys. |
| **Verify** | Type "test" rapidly with Space after each letter. All characters should appear. No stuck suppression. |

### C-3: Arrow Key Stuck During Accent Cycling

| Field | Value |
|-------|-------|
| **ID** | `PA-STUCK-03` |
| **Priority** | P0 |
| **Preconditions** | PowerAccent enabled. Activation key = Left/Right Arrow. |
| **Sequence** | 1. Hold 'o'. 2. Press Right Arrow (toolbar appears, cycles right). 3. Press Right Arrow again (cycles again). 4. Release 'o' while Right Arrow is still held. 5. Release Right Arrow. |
| **Expected** | Accent selected on 'o' release. Right Arrow released cleanly. |
| **Failure** | Right Arrow stuck in "pressed" state. Cursor moves right continuously in text fields. |
| **Root Cause** | Right Arrow KEYDOWN suppressed during toolbar visibility (lines 214-232). On 'o' release, toolbar closes and state resets. But the Right Arrow is still physically held, and the KEYUP for Right Arrow isn't handled specially — it passes through `CallNextHookEx`, but the matching KEYDOWN was never delivered to the app. App receives an orphan KEYUP. Some apps may ignore orphan KEYUPs and never clear the "arrow pressed" state. |
| **Verify** | Place cursor in a text field. It should not move. `GetAsyncKeyState(VK_RIGHT) & 0x8000 == 0`. |

### C-4: Shift State Lost During Accent Selection

| Field | Value |
|-------|-------|
| **ID** | `PA-STUCK-04` |
| **Priority** | P1 |
| **Preconditions** | PowerAccent enabled. Activation key = Space. |
| **Sequence** | 1. Hold Shift. 2. Hold 'a' (for uppercase accented A). 3. Press Space (toolbar appears with uppercase accents: À, Á, Â...). 4. Release Shift while toolbar is still visible. 5. Press Space to cycle. 6. Release 'a' to select. |
| **Expected** | Uppercase accent selected (e.g., 'Á'). Shift released cleanly. |
| **Failure** | When Shift is released (step 4), `m_leftShiftPressed`/`m_rightShiftPressed` flags update (lines 240-248), but the toolbar was populated with uppercase variants. The selected character may be uppercase (from initial Shift state) but Shift is now released. Or: the character output injects Shift+key but Shift was already released, causing a phantom Shift press that isn't cleaned up. |
| **Verify** | Type lowercase "hello" after the test. Should appear as "hello". `GetAsyncKeyState(VK_SHIFT) & 0x8000 == 0`. |

### C-5: On-Screen Keyboard — Repeated KEYDOWN Without KEYUP

| Field | Value |
|-------|-------|
| **ID** | `PA-STUCK-05` |
| **Priority** | P1 |
| **Preconditions** | PowerAccent enabled. Windows On-Screen Keyboard (osk.exe) open. |
| **Sequence** | 1. Click and hold 'e' on on-screen keyboard (generates repeated KEYDOWN events without KEYUP until release). 2. While holding, click Space on on-screen keyboard. |
| **Expected** | PowerAccent toolbar appears. Accent selectable. |
| **Failure** | Lines 172-178 suppress repeated 'e' KEYDOWN when toolbar visible, but on-screen keyboard may send events differently (injected flag set). If the injected 'e' events bypass the "same key pressed again" check, each repeat re-triggers the toolbar or resets `m_stopwatch`, making accent selection impossible. |
| **Verify** | Toolbar appears and stays stable. Can cycle with Space. Accent selected on 'e' release. |

### C-6: Game Mode Toggle During Active Accent Selection

| Field | Value |
|-------|-------|
| **ID** | `PA-STUCK-06` |
| **Priority** | P2 |
| **Preconditions** | PowerAccent enabled. Game Mode detectable by `m_isGameMode` callback. |
| **Sequence** | 1. Hold 'u'. 2. Press Space (toolbar appears). 3. Another process activates Game Mode. 4. Release 'u'. |
| **Expected** | Toolbar dismissed. Selected accent (or original key) typed. Game Mode fully active. |
| **Failure** | Game Mode check (line 202) only runs on toolbar activation, not during active selection. Toolbar remains visible but non-functional. Key events suppressed indefinitely until `letterPressed` somehow resets. Space stays suppressed → stuck. |
| **Verify** | Type normally after. No keys suppressed. `m_toolbarVisible == false`. |

---

## Cross-Module Interference Tests

### X-1: KBM Remap + PowerAccent Both Active — Remapped Key Triggers Accent

| Field | Value |
|-------|-------|
| **ID** | `CROSS-STUCK-01` |
| **Priority** | P0 |
| **Preconditions** | KBM enabled with remap: `X → A`. PowerAccent enabled with Space activation. |
| **Sequence** | 1. Press X (KBM remaps to A, sends A KEYDOWN with `KEYBOARDMANAGER_INJECTED_FLAG`). 2. Hold X. Press Space. 3. Release X. |
| **Expected** | One of: (a) PowerAccent shows accents for 'A', or (b) PowerAccent ignores injected keys. Either is valid as long as no key is stuck. |
| **Failure** | PowerAccent's hook sees A KEYDOWN, sets `letterPressed = A`. But it also checks `GetAsyncKeyState('A')` (line 190) — physical 'A' is NOT pressed (X is pressed). `isLetterReleased` returns true. Toolbar doesn't show. Space is NOT suppressed and types a space character. Now: KBM sends A for X, but the A KEYDOWN was already consumed by PowerAccent's hook check, potentially causing A to be stuck or doubled. |
| **Root Cause** | Two independent hooks processing the same logical key event. KBM injects A, PowerAccent's hook sees A but `GetAsyncKeyState` reports physical state (X pressed, not A). State divergence. |
| **Verify** | Release all keys. Type in Notepad. No stuck keys. `GetAsyncKeyState` clear for all modifiers AND for 'A' and 'X'. |

### X-2: Centralized Hook Hotkey During Active KBM Remap

| Field | Value |
|-------|-------|
| **ID** | `CROSS-STUCK-02` |
| **Priority** | P0 |
| **Preconditions** | KBM remap: `Ctrl+A → Ctrl+B`. PowerToys Run hotkey: `Alt+Space`. |
| **Sequence** | 1. Press Ctrl+A (KBM fires: sends Ctrl+B). 2. While still holding Ctrl+A, press Alt+Space (centralized hook fires PowerToys Run). 3. Release Space. 4. Release Alt. 5. Release A. 6. Release Ctrl. |
| **Expected** | Ctrl+B active. PowerToys Run opens. On full release, all keys clean. |
| **Failure** | Centralized hook's hotkey match (line 163-169) fires and returns 1 (swallow Alt+Space). KBM's Case 5 (unexpected key) triggers for Alt, attempting to revert keyboard state. But the centralized hook already swallowed the event. KBM sends modifier UPs/DOWNs that don't match what the centralized hook expects. Ctrl or Alt stuck. |
| **Root Cause** | Two hooks processing the same event stream with different suppression decisions. KBM's handler runs in the centralized hook's callback chain but the centralized hook also processes hotkeys independently. |
| **Verify** | Close PowerToys Run. Type in Notepad. No modifiers stuck. |

### X-3: PowerAccent + KBM Shortcut-to-Text — Race on Space Key

| Field | Value |
|-------|-------|
| **ID** | `CROSS-STUCK-03` |
| **Priority** | P1 |
| **Preconditions** | KBM remap: `Ctrl+Space → "hello"` (text). PowerAccent enabled with Space activation. |
| **Sequence** | 1. Hold 'e'. 2. Press Ctrl+Space. |
| **Expected** | Either KBM fires (Ctrl+Space → "hello") OR PowerAccent activates (Space triggers accent for 'e'). Not both. No stuck keys. |
| **Failure** | Both modules partially process the event. PowerAccent sees Space KEYDOWN, starts accent flow, suppresses Space. KBM sees Ctrl+Space KEYDOWN but Space was suppressed. KBM's shortcut never fully triggers. Ctrl stays logically pressed because KBM expected to handle the full sequence. |
| **Root Cause** | Hook ordering determines which module sees the event first. If PowerAccent's hook is installed before KBM's, it suppresses Space before KBM can match Ctrl+Space. |
| **Verify** | Release all keys. `GetAsyncKeyState(VK_CONTROL) & 0x8000 == 0`. No repeated "hello" in Notepad. |

### X-4: Fast Typing With All Three Systems Active

| Field | Value |
|-------|-------|
| **ID** | `CROSS-STUCK-04` |
| **Priority** | P0 |
| **Preconditions** | All three active: centralized hook (FancyZones hotkey Win+Shift+Arrow), KBM remap (CapsLock → Ctrl), PowerAccent (Space activation). |
| **Sequence** | 1. Type "café résumé naïve" at ~80 WPM, using PowerAccent for accented characters. 2. Mid-sentence, accidentally hit CapsLock (remapped to Ctrl). 3. Continue typing. |
| **Expected** | Accented characters produced correctly. CapsLock acts as Ctrl (no Caps toggle). Typing resumes normally. |
| **Failure** | CapsLock→Ctrl remap sends Ctrl DOWN. While PowerAccent is in accent-selection state (toolbar visible, Space suppressed), the Ctrl DOWN changes the modifier context. When accent is selected and toolbar closes, Ctrl is still logically pressed (CapsLock physically down → Ctrl injected, but release of CapsLock must send Ctrl UP). If accent selection consumes the CapsLock KEYUP, Ctrl stays stuck. |
| **Verify** | Complete the sentence. Type "test" — appears as "test". No Ctrl stuck. No Caps Lock toggled. |

### X-5: Centralized Hook Pressed-Key Action During PowerAccent Toolbar

| Field | Value |
|-------|-------|
| **ID** | `CROSS-STUCK-05` |
| **Priority** | P1 |
| **Preconditions** | PowerAccent enabled (Space activation). PowerToys Run enabled (activated by pressing and holding Win for 300ms). |
| **Sequence** | 1. Hold 'e'. 2. Press Space (PowerAccent toolbar appears). 3. While toolbar is visible and 'e' still held, press and hold Win for 300ms. 4. Release Win. 5. Release 'e'. |
| **Expected** | PowerAccent handles 'e' + Space. Win press activates PowerToys Run timer. On Win release, either PowerToys Run opens or Win key cleaned up. On 'e' release, accent selected. |
| **Failure** | Win key timer in centralized hook fires (line 75: `PressedKeyTimerProc`). The action callback activates PowerToys Run which steals focus. PowerAccent's toolbar loses context. 'e' KEYUP arrives but toolbar is already dismissed by focus change. `letterPressed` may not be reset, causing subsequent 'e' presses to be mishandled. |
| **Verify** | Close PowerToys Run. Type "hello" — no stuck keys, no suppressed letters. |

---

## Automated Repro Scenarios

### Approach 1: Unit Tests Using MockedInput (Recommended — Existing Infrastructure)

The `MockedInput` class in `KeyboardManagerEngineTest` already provides the perfect abstraction. Each test:

1. Sets up `MockedInput` with `SetHookProc` pointing to the handler under test.
2. Calls `SendVirtualInput` to simulate key sequences.
3. Inspects `MockedInput::keyboardState` to verify no keys are stuck.
4. Uses `GetSendVirtualInputCallCount()` to verify expected KEYUP events were sent.

```cpp
// Example: Verify Ctrl not stuck after Ctrl+A → Ctrl+B remap
TEST_METHOD(CtrlNotStuckAfterShortcutRemap_Case1_ModifierReleasedFirst)
{
    // Setup: Ctrl+A → Ctrl+B
    Shortcut src;
    src.ctrlKey = ModifierKey::Left;
    src.actionKey = 'A';
    Shortcut target;
    target.ctrlKey = ModifierKey::Left;
    target.actionKey = 'B';
    state.AddOSLevelShortcut(src, target);
    mockedInput.SetHookProc(
        [&](LowlevelKeyboardEvent* data) {
            return HandleShortcutRemapEvent(
                data, state, mockedInput, ...);
        });

    // Simulate: Ctrl DOWN, A DOWN, Ctrl UP, A UP
    mockedInput.SendVirtualInput(
        KeyDown(VK_LCONTROL), KeyDown('A'),
        KeyUp(VK_LCONTROL), KeyUp('A'));

    // Verify: No keys stuck
    Assert::IsFalse(mockedInput.GetVirtualKeyState(VK_LCONTROL));
    Assert::IsFalse(mockedInput.GetVirtualKeyState(VK_CONTROL));
    Assert::IsFalse(mockedInput.GetVirtualKeyState('A'));
    Assert::IsFalse(mockedInput.GetVirtualKeyState('B'));
    Assert::IsFalse(mockedInput.GetVirtualKeyState(VK_LWIN));
}
```

#### Proposed New Test File: `StuckKeyTests.cpp`

Location: `src/modules/keyboardmanager/KeyboardManagerEngineTest/StuckKeyTests.cpp`

```
StuckKeyTests test class:
├── SingleKey_KeyToShortcut_NoModifierStuck()         // B-1
├── SingleKey_DisabledKey_RapidToggle()               // B-7
├── Shortcut_Case1_ModifierReleasedFirst()            // B-2
├── Shortcut_Case3_ActionKeyReleasedFirst()           // verify no action key stuck
├── Shortcut_Case5_UnexpectedKeyDuringRemap()         // B-3
├── Shortcut_AltGr_NoPhantomCtrl()                    // B-4
├── Shortcut_Chord_SecondKeyNeverPressed()             // B-5
├── Shortcut_Case4_DuplicateModifier()                // B-6
├── Shortcut_ToText_ModifiersReleasedBeforeText()     // B-8
├── Shortcut_AppSpecific_ForegroundSwitch()            // B-9
├── AllModifiersClean_AfterEveryTestSequence()         // Meta-validator
└── StressTest_100RapidRemaps_NoStuckKeys()           // Stress
```

### Approach 2: Key State Validator (Integration Test Harness)

A reusable validator function that can be called after any test sequence:

```cpp
struct StuckKeyReport {
    DWORD vk;
    const char* name;
    bool isStuck;
};

std::vector<StuckKeyReport> ValidateNoKeysStuck(MockedInput& input)
{
    static const std::pair<DWORD, const char*> modifiers[] = {
        {VK_LWIN, "Left Win"},     {VK_RWIN, "Right Win"},
        {VK_LCONTROL, "Left Ctrl"},{VK_RCONTROL, "Right Ctrl"},
        {VK_LSHIFT, "Left Shift"}, {VK_RSHIFT, "Right Shift"},
        {VK_LMENU, "Left Alt"},    {VK_RMENU, "Right Alt"},
        {VK_CAPITAL, "CapsLock"},  {VK_SPACE, "Space"},
    };

    std::vector<StuckKeyReport> stuck;
    for (auto& [vk, name] : modifiers) {
        if (input.GetVirtualKeyState(vk)) {
            stuck.push_back({vk, name, true});
        }
    }
    return stuck;
}
```

### Approach 3: SendInput-Based System Integration Test

For real end-to-end testing (runs on actual Windows with PowerToys installed):

```cpp
// Integration test using real SendInput
void SystemTest_NoStuckKeysAfterRemapSequence()
{
    // Precondition: PowerToys running, KBM remap Ctrl+A → Ctrl+B configured

    INPUT inputs[4] = {};
    // Ctrl DOWN
    inputs[0].type = INPUT_KEYBOARD;
    inputs[0].ki.wVk = VK_LCONTROL;
    // A DOWN
    inputs[1].type = INPUT_KEYBOARD;
    inputs[1].ki.wVk = 'A';
    // Ctrl UP
    inputs[2].type = INPUT_KEYBOARD;
    inputs[2].ki.wVk = VK_LCONTROL;
    inputs[2].ki.dwFlags = KEYEVENTF_KEYUP;
    // A UP
    inputs[3].type = INPUT_KEYBOARD;
    inputs[3].ki.wVk = 'A';
    inputs[3].ki.dwFlags = KEYEVENTF_KEYUP;

    SendInput(4, inputs, sizeof(INPUT));
    Sleep(100); // Allow hooks to process

    // Validate
    for (DWORD vk : {VK_LCONTROL, VK_RCONTROL, VK_LWIN, VK_RWIN,
                      VK_LSHIFT, VK_RSHIFT, VK_LMENU, VK_RMENU}) {
        ASSERT(!(GetAsyncKeyState(vk) & 0x8000),
               "Key 0x%x stuck after remap sequence", vk);
    }
}
```

### Approach 4: State Machine Transition Auditor

Hook into KBM's state machine to log and verify transitions:

```cpp
class RemapStateAuditor {
    struct Transition {
        int caseNumber;           // 1-6
        bool isShortcutInvoked;   // before
        bool afterInvoked;        // after
        DWORD actionKey;
        std::vector<DWORD> modifiersPressed;
        std::vector<DWORD> modifiersReleased;
    };
    std::vector<Transition> log;

    void Verify() {
        for (auto& t : log) {
            // Rule 1: If isShortcutInvoked transitions false→true,
            //         there MUST be a later true→false transition
            // Rule 2: Every modifier DOWN must have matching UP
            // Rule 3: Every action key DOWN must have matching UP
            // Rule 4: No transition should leave more keys pressed
            //         than physically held
        }
    }
};
```

### Approach 5: Continuous Key State Monitor (Background Thread)

A background monitor that detects stuck keys during automated test runs:

```cpp
class KeyStateMonitor {
    std::atomic<bool> running{true};
    std::thread monitorThread;
    static constexpr DWORD MODIFIERS[] = {
        VK_LWIN, VK_RWIN, VK_LCONTROL, VK_RCONTROL,
        VK_LSHIFT, VK_RSHIFT, VK_LMENU, VK_RMENU
    };
    static constexpr int STUCK_THRESHOLD_MS = 5000;

    void MonitorLoop() {
        std::map<DWORD, std::chrono::steady_clock::time_point> pressedSince;
        while (running) {
            for (DWORD vk : MODIFIERS) {
                bool pressed = GetAsyncKeyState(vk) & 0x8000;
                if (pressed && pressedSince.find(vk) == pressedSince.end()) {
                    pressedSince[vk] = std::chrono::steady_clock::now();
                } else if (!pressed) {
                    pressedSince.erase(vk);
                } else {
                    auto elapsed = std::chrono::steady_clock::now() - pressedSince[vk];
                    if (elapsed > std::chrono::milliseconds(STUCK_THRESHOLD_MS)) {
                        LogStuckKey(vk);
                        // Optionally: inject KEYUP to recover
                    }
                }
            }
            Sleep(100);
        }
    }
};
```

---

## Stuck Key Recovery Mechanism Design

### Proposed: `StuckKeyRecovery` Class in Runner

**Location:** `src/runner/stuck_key_recovery.h` / `.cpp`

#### Design

```
┌──────────────────────────────────────────────┐
│           StuckKeyRecovery                    │
│                                               │
│  Monitor Thread (100ms poll interval)         │
│  ┌─────────────────────────────────────────┐  │
│  │ For each modifier (Win/Ctrl/Alt/Shift): │  │
│  │  1. Read GetAsyncKeyState(vk)           │  │
│  │  2. If pressed:                         │  │
│  │     a. Record first-seen timestamp      │  │
│  │     b. If held > threshold (10s):       │  │
│  │        - Check GetKeyState for toggle   │  │
│  │        - Check physical input (RAWINPUT)│  │
│  │        - If no physical key → STUCK     │  │
│  │        - Inject KEYUP via SendInput     │  │
│  │        - Log telemetry event            │  │
│  │  3. If not pressed:                     │  │
│  │     - Clear first-seen timestamp        │  │
│  └─────────────────────────────────────────┘  │
│                                               │
│  Grace Period: Suppress recovery during       │
│  active remap / accent selection              │
│  (query KBM state + PowerAccent state)        │
└──────────────────────────────────────────────┘
```

#### Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `pollIntervalMs` | 100 | How often to check key states |
| `stuckThresholdMs` | 10000 | How long a key must be "pressed" with no physical input before considered stuck |
| `graceAfterRemapMs` | 2000 | Don't trigger recovery within 2s of a KBM remap completing |
| `maxRecoveriesPerMinute` | 3 | Rate limit to prevent recovery storms |
| `enableTelemetry` | true | Log stuck key events for analysis |

#### Risk Analysis

| Risk | Severity | Mitigation |
|------|----------|------------|
| **False positive: legitimate long-press** (e.g., Win held for window snapping, Ctrl held for multi-select) | HIGH | Use RAWINPUT to verify physical key state. Only recover if `GetAsyncKeyState` says pressed but no corresponding RAWINPUT message in the last `stuckThresholdMs`. |
| **False positive: accessibility software** (StickyKeys, on-screen keyboard, voice control holding keys) | MEDIUM | Check if accessibility features are active (`SystemParametersInfo(SPI_GETSTICKYKEYS)`). Increase threshold or disable recovery when detected. |
| **Recovery during game/fullscreen** | LOW | Check foreground app. Don't inject KEYUP into games (check game mode flag). |
| **Recovery KEYUP itself triggers a shortcut** | MEDIUM | Mark recovery KEYUPs with `KEYBOARDMANAGER_INJECTED_FLAG` so KBM ignores them. Use `CENTRALIZED_KEYBOARD_HOOK_DONT_TRIGGER_FLAG` in extraInfo. |
| **Race: physical key pressed between check and recovery** | LOW | Double-check `GetAsyncKeyState` immediately before `SendInput`. If still pressed, inject KEYUP. If not, skip (already resolved). |
| **Threading: recovery interferes with active hook processing** | MEDIUM | Use a critical section shared with the hook callback. Recovery only injects when hook is not actively processing. |

#### Implementation Sketch

```cpp
class StuckKeyRecovery {
    static constexpr DWORD WATCHED_KEYS[] = {
        VK_LWIN, VK_RWIN, VK_LCONTROL, VK_RCONTROL,
        VK_LSHIFT, VK_RSHIFT, VK_LMENU, VK_RMENU
    };

    struct KeyWatch {
        std::chrono::steady_clock::time_point firstPressedAt;
        bool tracking = false;
        int recoveryCount = 0;
    };

    std::map<DWORD, KeyWatch> watches;
    std::chrono::steady_clock::time_point lastRemapActivity;
    std::atomic<bool> running{true};

    void InjectKeyUp(DWORD vk) {
        INPUT input{};
        input.type = INPUT_KEYBOARD;
        input.ki.wVk = static_cast<WORD>(vk);
        input.ki.dwFlags = KEYEVENTF_KEYUP;
        input.ki.dwExtraInfo =
            CENTRALIZED_KEYBOARD_HOOK_DONT_TRIGGER_FLAG;
        SendInput(1, &input, sizeof(INPUT));
    }

    bool IsPhysicallyPressed(DWORD vk) {
        // Use RAWINPUT or GetAsyncKeyState heuristics
        // to determine if a physical key is actually down.
        // RAWINPUT provides ground truth for HID devices.
        return GetAsyncKeyState(vk) & 0x8000;
    }

    void CheckAndRecover() {
        auto now = std::chrono::steady_clock::now();

        // Grace period after remap activity
        if (now - lastRemapActivity <
            std::chrono::milliseconds(GRACE_AFTER_REMAP_MS))
            return;

        for (DWORD vk : WATCHED_KEYS) {
            bool logicallyPressed = GetAsyncKeyState(vk) & 0x8000;

            if (logicallyPressed) {
                if (!watches[vk].tracking) {
                    watches[vk].tracking = true;
                    watches[vk].firstPressedAt = now;
                } else {
                    auto elapsed = now - watches[vk].firstPressedAt;
                    if (elapsed >
                        std::chrono::milliseconds(STUCK_THRESHOLD_MS))
                    {
                        // Double-check before recovery
                        if (GetAsyncKeyState(vk) & 0x8000) {
                            InjectKeyUp(vk);
                            watches[vk].tracking = false;
                            watches[vk].recoveryCount++;
                            LogTelemetry(vk, elapsed);
                        }
                    }
                }
            } else {
                watches[vk].tracking = false;
            }
        }
    }
};
```

#### Integration Points

1. **Runner startup**: Create `StuckKeyRecovery` instance, start monitor thread.
2. **KBM remap events**: Call `recovery.NotifyRemapActivity()` on every remap invocation/reset to extend grace period.
3. **PowerAccent toolbar**: Call `recovery.NotifyAccentActive(true/false)` when toolbar shows/hides.
4. **Centralized hook hotkeys**: Call `recovery.NotifyHotkeyFired()` on hotkey match.
5. **Module unload**: Stop recovery for that module's keys during unload grace period.

---

## Test Execution Priority Matrix

| Priority | Count | Test IDs | Execution |
|----------|-------|----------|-----------|
| **P0 — Always Run** | 9 | HOOK-STUCK-01, KBM-STUCK-01/02/03/04/08, PA-STUCK-01/02/03, CROSS-STUCK-01/02/04 | Every PR, every CI build |
| **P1 — Daily** | 8 | HOOK-STUCK-02/03, KBM-STUCK-05/06/07/09, PA-STUCK-04/05, CROSS-STUCK-03/05 | Nightly CI |
| **P2 — Weekly** | 3 | HOOK-STUCK-04/05, PA-STUCK-06 | Weekly regression |

---

## Summary of Root Causes Found in Code Review

| # | Module | Root Cause | Risk |
|---|--------|-----------|------|
| 1 | Centralized Hook | `vkCodePressed` is non-atomic, single-key tracking | Timers not killed on multi-key |
| 2 | Centralized Hook | `pressedKeyDescriptors.empty()` checked without lock | Data race on module unload |
| 3 | Centralized Hook | Timer ID 16-bit hash collision | Wrong module's timer killed |
| 4 | Centralized Hook | No try-catch around action callbacks | Hook crash = all keys stuck |
| 5 | KBM | AltGr static flag `isAltRightKeyInvoked` persists | Phantom LCtrl stuck |
| 6 | KBM | Case 5 revert may miss action key UP for modifier-overlapping targets | Target key stuck |
| 7 | KBM | App-specific remap — foreground change between DOWN/UP | Modifier stuck in new app |
| 8 | KBM | Text remap may not release source modifiers first | Modifiers active during text |
| 9 | KBM | Chord `chordStarted` not reset on wrong second key | Stale chord state |
| 10 | PowerAccent | Suppressed Space KEYDOWN but KEYUP not tracked | Space stuck |
| 11 | PowerAccent | Arrow KEYDOWN suppressed, orphan KEYUP on toolbar close | Arrow stuck in app |
| 12 | PowerAccent | No cleanup on Game Mode / exclusion change mid-selection | Toolbar stuck open, keys suppressed |
| 13 | Cross-module | KBM-injected key fails `GetAsyncKeyState` physical check in PA | PA ignores key, state divergence |
| 14 | Cross-module | Two hooks with independent suppression decisions | Modifier stuck from conflicting swallow |
