# Stuck Key Bug Fix Plan — PowerToys

**Base repo:** `crutkas/PowerToys` (fork)
**Target for PRs:** `crutkas/PowerToys` `main` branch
**Date:** 2025-07-17
**Status:** Implementation-ready

> **Process for each fix:**
> 1. Create a feature branch off `main` (e.g., `fix-stuck-keys/a1-fz-destroy`)
> 2. Implement the fix
> 3. Build full PowerToys solution — must compile clean
> 4. Run the "How to Reproduce" steps to confirm the bug exists on `main` and is fixed on the branch
> 5. Run the "How to Verify" steps to confirm no regressions
> 6. Fleet code review: launch 1 Opus 4.6 code-reviewer + 1 GPT-5.4 code-reviewer on the diff. Both must approve.
> 7. Push branch to `crutkas/PowerToys` and open an **independent PR** against `crutkas/PowerToys:main`
>
> Fixes are independent where possible. Each PR is mergeable on its own.
>
> Each fix below includes **How to Reproduce** and **How to Verify** sections inline, right after the root cause — before the code changes.

---

## Table of Contents

1. [Priority Matrix](#priority-matrix)
2. [Stream A: FancyZones Stuck Drag (P0)](#stream-a-fancyzones-stuck-drag-p0)
3. [Stream B: Centralized Hook Hardening (P0)](#stream-b-centralized-hook-hardening-p0)
4. [Stream C: Keyboard Manager Modifier Safety (P1)](#stream-c-keyboard-manager-modifier-safety-p1)
5. [Stream D: PowerAccent Injection Hygiene (P1)](#stream-d-poweraccent-injection-hygiene-p1)
6. [Stream E: Cross-Module Safety Net (P2)](#stream-e-cross-module-safety-net-p2)
7. [Test Plan](#test-plan)
8. [Risk Assessment](#risk-assessment)

---

## Priority Matrix

| ID | Stream | Bug | Priority | Risk | GitHub Issues |
|----|--------|-----|----------|------|---------------|
| A1 | FancyZones | No EVENT_OBJECT_DESTROY → drag stuck forever | P0 | Low | #410 (58 upvotes) |
| A2 | FancyZones | All Shift+key swallowed during drag | P0 | Low | #410 |
| A3 | FancyZones | Number keys swallowed by quickLayoutSwitch during stuck drag | P0 | Low | #410 |
| A4 | FancyZones | Hooks not cleaned up after stuck drag | P1 | Low | — |
| B1 | Centralized Hook | SendInput dummy 0xFF has no dwExtraInfo tag | P0 | Low | — |
| B2 | Centralized Hook | No LLKHF_INJECTED check | P0 | Low | — |
| B3 | Centralized Hook | PressedKeyTimerProc doesn't revalidate key state | P1 | Low | — |
| B4 | Centralized Hook | Stop() doesn't kill timers or reset state | P1 | Low | — |
| B5 | Centralized Hook | vkCodePressed race conditions | P1 | Low | — |
| C1 | Keyboard Manager | SendVirtualInput failure → key eaten, replacement lost | P1 | Medium | #17035, #29015 |
| C2 | Keyboard Manager | No state cleanup on focus/app change | P1 | Medium | #34714, #10963 |
| C3 | Keyboard Manager | Settings reload data race | P1 | Medium | — |
| C4 | Keyboard Manager | Single-key-to-text doesn't release source modifiers | P1 | Low | #9778 |
| D1 | PowerAccent | No LLKHF_INJECTED filter | P1 | Low | — |
| D2 | PowerAccent | Output SendInput has no dwExtraInfo tag | P1 | Low | — |
| D3 | PowerAccent | No cleanup on focus loss/session change | P2 | Low | — |
| D4 | PowerAccent | SendKeys.SendWait for arrow keys | P2 | Medium | — |
| E1 | Cross-Module | Add POWERTOYS_INJECTED_FLAG to shared_constants.h | P1 | Low | — |
| E2 | Cross-Module | Session-change cleanup in centralized hook | P2 | Low | — |

---

## Stream A: FancyZones Stuck Drag (P0)

**Root cause:** When a window is closed mid-drag (Alt+F4, task killed, etc.), EVENT_SYSTEM_MOVESIZEEND never fires, leaving `m_dragging = true` permanently. All subsequent Shift+key combos are swallowed, and number keys trigger layout switches.

---

### PR 1 — A1: Subscribe to EVENT_OBJECT_DESTROY and clean up drag state

**GitHub Issues:** #410 (58 👍), #43996

#### How to Reproduce
1. Enable FancyZones with Shift-drag mode (default).
2. Open a terminal and run: `powershell -command "Start-Process notepad; Start-Sleep 3; Stop-Process -Name notepad"`
3. Quickly start dragging the Notepad window while holding Shift (zone overlay appears).
4. Wait for Notepad to be killed (3 seconds).
5. Open a new Notepad and try typing `Hello 123`.
6. **Bug:** Number keys and Shift+key combos are eaten. Zone overlay may be stuck on screen.

#### How to Verify (after fix)
1. Repeat the same steps above.
2. **Expected:** After Notepad is killed mid-drag, zone overlay disappears. Typing works normally. No keys are stuck.
3. Also verify normal Shift+drag still works: hold Shift, drag a window, zones appear, drop to zone, zones disappear.

#### Code Review Gate
Before merging to integration branch: build full solution, run repro/verify steps, then launch 1 Opus + 1 GPT code-reviewer fleet on the diff. Both must approve.

#### Files
- `src/modules/fancyzones/FancyZones/FancyZonesApp.cpp`
- `src/modules/fancyzones/FancyZonesLib/FancyZones.cpp`
- `src/modules/fancyzones/FancyZonesLib/FancyZonesWinHookEventIDs.h`
- `src/modules/fancyzones/FancyZonesLib/FancyZonesWinHookEventIDs.cpp`

#### Changes

**Change 1a — Add EVENT_OBJECT_DESTROY to subscribed events:**

File: `src/modules/fancyzones/FancyZones/FancyZonesApp.cpp` (line 96)

```cpp
// BEFORE:
std::array<DWORD, 6> events_to_subscribe = {
    EVENT_SYSTEM_MOVESIZESTART,
    EVENT_SYSTEM_MOVESIZEEND,
    EVENT_OBJECT_NAMECHANGE,
    EVENT_OBJECT_UNCLOAKED,
    EVENT_OBJECT_SHOW,
    EVENT_OBJECT_CREATE
};

// AFTER:
std::array<DWORD, 7> events_to_subscribe = {
    EVENT_SYSTEM_MOVESIZESTART,
    EVENT_SYSTEM_MOVESIZEEND,
    EVENT_OBJECT_NAMECHANGE,
    EVENT_OBJECT_UNCLOAKED,
    EVENT_OBJECT_SHOW,
    EVENT_OBJECT_CREATE,
    EVENT_OBJECT_DESTROY
};
```

**Change 1b — Add WM_PRIV_WINDOWDESTROYED message ID:**

File: `src/modules/fancyzones/FancyZonesLib/FancyZonesWinHookEventIDs.h` — add after line 7:

```cpp
extern UINT WM_PRIV_WINDOWDESTROYED;
```

File: `src/modules/fancyzones/FancyZonesLib/FancyZonesWinHookEventIDs.cpp` — add declaration and registration:

```cpp
// Add with other declarations at top:
UINT WM_PRIV_WINDOWDESTROYED;

// Add inside InitializeWinhookEventIds():
WM_PRIV_WINDOWDESTROYED = RegisterWindowMessage(L"{a1b2c3d4-e5f6-7890-abcd-fancyzones01}");
```

**Change 1c — Route EVENT_OBJECT_DESTROY to window proc:**

File: `src/modules/fancyzones/FancyZonesLib/FancyZones.cpp` (inside `HandleWinHookEvent`, after the EVENT_OBJECT_CREATE block at line 131):

```cpp
// BEFORE (line 131):
        break;
    }

// AFTER:
        break;

    case EVENT_OBJECT_DESTROY:
        if (data->idObject == OBJID_WINDOW)
        {
            PostMessageW(m_window, WM_PRIV_WINDOWDESTROYED, wparam, lparam);
        }
        break;
    }
```

**Change 1d — Handle WM_PRIV_WINDOWDESTROYED in WndProc:**

File: `src/modules/fancyzones/FancyZonesLib/FancyZones.cpp` (after the `WM_PRIV_WINDOWCREATED` handler at ~line 701):

```cpp
    else if (message == WM_PRIV_WINDOWDESTROYED)
    {
        auto hwnd = reinterpret_cast<HWND>(wparam);
        // If the destroyed window was being dragged, force-end the drag
        if (m_windowMouseSnapper && m_windowMouseSnapper->GetDraggedWindow() == hwnd)
        {
            Logger::info(L"Window destroyed during drag — forcing MoveSizeEnd");
            MoveSizeEnd();
        }
    }
```

> **NOTE:** This requires `WindowMouseSnap` to expose a `GetDraggedWindow()` accessor.
> Check if `WindowMouseSnap` stores the dragged HWND (it does — it's passed via `Create(window, ...)`).
> Add a simple getter if one doesn't exist:

File: `src/modules/fancyzones/FancyZonesLib/WindowMouseSnap.h` — add public method:

```cpp
HWND GetDraggedWindow() const noexcept { return m_window; }
```

Verify that `m_window` is the member storing the dragged HWND in `WindowMouseSnap`. If the member has a different name, adjust accordingly.

---

### PR 2 — A2: Fix overbroad Shift+key swallowing during drag

**GitHub Issues:** #410

#### How to Reproduce
1. Enable FancyZones. Start dragging any window's title bar (don't release mouse button).
2. Hold Shift (zone overlay appears).
3. While still dragging with Shift held, try pressing Shift+A, Shift+1, or any Shift+key combo.
4. **Bug:** The keystroke is completely swallowed — nothing typed, no effect in any app.

#### How to Verify (after fix)
1. Repeat the same drag + Shift scenario.
2. **Expected:** Bare Shift key still toggles the zone overlay. But Shift+A, Shift+1, etc. pass through to the focused app.
3. Also verify: Shift-drag zone activation still works end-to-end (Shift+drag → drop to zone → window snaps).

#### Code Review Gate
Before merging to integration branch: build full solution, run repro/verify steps, then launch 1 Opus + 1 GPT code-reviewer fleet on the diff. Both must approve.

#### Changes

**File:** `src/modules/fancyzones/FancyZonesLib/FancyZones.cpp` (lines 522-525)

```cpp
// BEFORE:
    if (m_draggingState.IsDragging() && shift)
    {
        return true;
    }
    return false;

// AFTER:
    // Only suppress the bare Shift key itself during drag (used for drag-toggle).
    // Do NOT swallow Shift+<other key> combos — that steals keystrokes from apps.
    if (m_draggingState.IsDragging() && shift &&
        (info->vkCode == VK_LSHIFT || info->vkCode == VK_RSHIFT))
    {
        return true;
    }
    return false;
```

---

### PR 3 — A3: Require modifier for quickLayoutSwitch while dragging

**GitHub Issues:** #410

#### How to Reproduce
1. Assign layout hotkeys to number keys in FancyZones settings (e.g., 1=Focus, 2=Columns).
2. Trigger a stuck drag state (use the PR 1 repro: kill a window mid-drag).
3. Open Notepad and press keys 1, 2, 3, etc.
4. **Bug:** Number keys with hotkey assignments are eaten — they switch FancyZones layouts instead of typing.

#### How to Verify (after fix)
1. Even with a stuck drag state, pressing bare number keys should type normally.
2. Win+Ctrl+Alt+1 should still switch layouts (if desired).

#### Code Review Gate
Before merging to integration branch: build full solution, run repro/verify steps, then launch 1 Opus + 1 GPT code-reviewer fleet on the diff. Both must approve.

#### Changes

**File:** `src/modules/fancyzones/FancyZonesLib/FancyZones.cpp` (line 508)

```cpp
// BEFORE:
        bool changeLayoutWhileDragging = dragging && digitPressed != -1;

// AFTER:
        // Require Win+Ctrl+Alt even while dragging to prevent accidental layout switches
        // when drag state is stuck (root cause of #410 "steals number keys")
        bool changeLayoutWhileDragging = dragging && win && ctrl && alt && digitPressed != -1;
```

**Rationale:** When drag state is stuck (A1), bare number key presses trigger layout switches. Adding the same Win+Ctrl+Alt requirement as the non-dragging path prevents accidental layout changes even if drag state somehow leaks.

---

### PR 4 — A4: Tag SwallowKey SendInput with dwExtraInfo

**File:** `src/modules/fancyzones/FancyZonesLib/util.cpp` (lines 260-267)

```cpp
// BEFORE:
    void SwallowKey(const WORD key) noexcept
    {
        INPUT inputKey[1] = {};
        inputKey[0].type = INPUT_KEYBOARD;
        inputKey[0].ki.wVk = key;
        inputKey[0].ki.dwFlags = KEYEVENTF_KEYUP;
        SendInput(1, inputKey, sizeof(INPUT));
    }

// AFTER:
    void SwallowKey(const WORD key) noexcept
    {
        INPUT inputKey[1] = {};
        inputKey[0].type = INPUT_KEYBOARD;
        inputKey[0].ki.wVk = key;
        inputKey[0].ki.dwFlags = KEYEVENTF_KEYUP;
        inputKey[0].ki.dwExtraInfo = PowertoyModuleIface::CENTRALIZED_KEYBOARD_HOOK_DONT_TRIGGER_FLAG;
        SendInput(1, inputKey, sizeof(INPUT));
    }
```

Add `#include <modules/interface/powertoy_module_interface.h>` to the includes in `util.cpp` if not already present.

---

## Stream B: Centralized Hook Hardening (P0)

**All changes in:** `src/runner/centralized_kb_hook.cpp` and `src/runner/centralized_kb_hook.h`

### PR 5 — B1: Tag SendInput dummy 0xFF with dwExtraInfo

**File:** `src/runner/centralized_kb_hook.cpp` (lines 176-180)

```cpp
// BEFORE:
                INPUT dummyEvent[1] = {};
                dummyEvent[0].type = INPUT_KEYBOARD;
                dummyEvent[0].ki.wVk = 0xFF;
                dummyEvent[0].ki.dwFlags = KEYEVENTF_KEYUP;
                SendInput(1, dummyEvent, sizeof(INPUT));

// AFTER:
                INPUT dummyEvent[1] = {};
                dummyEvent[0].type = INPUT_KEYBOARD;
                dummyEvent[0].ki.wVk = 0xFF;
                dummyEvent[0].ki.dwFlags = KEYEVENTF_KEYUP;
                dummyEvent[0].ki.dwExtraInfo = PowertoyModuleIface::CENTRALIZED_KEYBOARD_HOOK_DONT_TRIGGER_FLAG;
                SendInput(1, dummyEvent, sizeof(INPUT));
```

#### How to Reproduce (before fix)
1. Enable Color Picker (Win+Shift+C) and Keyboard Manager with any remap active.
2. Press Win+Shift+C rapidly 20+ times.
3. **Bug:** The untagged dummy 0xFF event from the centralized hook can confuse KBM's state machine, occasionally leaving a modifier stuck.

#### How to Verify (after fix)
1. Repeat rapid hotkey presses. No ghost modifier behavior.
2. Use Spy++ or a key logger to verify the dummy 0xFF event now carries `dwExtraInfo=0x110`.

---

### PR 6 — B2: Add LLKHF_INJECTED early-out

**File:** `src/runner/centralized_kb_hook.cpp` (after line 89, before the dwExtraInfo check)

```cpp
// BEFORE:
        const auto& keyPressInfo = *reinterpret_cast<KBDLLHOOKSTRUCT*>(lParam);

        if (keyPressInfo.dwExtraInfo == PowertoyModuleIface::CENTRALIZED_KEYBOARD_HOOK_DONT_TRIGGER_FLAG)
        {
            // The new keystroke was generated from one of our actions. We should pass it along.
            return CallNextHookEx(hHook, nCode, wParam, lParam);
        }

// AFTER:
        const auto& keyPressInfo = *reinterpret_cast<KBDLLHOOKSTRUCT*>(lParam);

        if (keyPressInfo.dwExtraInfo == PowertoyModuleIface::CENTRALIZED_KEYBOARD_HOOK_DONT_TRIGGER_FLAG)
        {
            // The new keystroke was generated from one of our actions. We should pass it along.
            return CallNextHookEx(hHook, nCode, wParam, lParam);
        }

        // Skip events injected by other processes (OSK, macro tools, accessibility).
        // This prevents synthetic input from driving our hotkey/pressed-key state machines.
        if (keyPressInfo.flags & LLKHF_INJECTED)
        {
            return CallNextHookEx(hHook, nCode, wParam, lParam);
        }
```

---

### PR 7 — B3: Revalidate key state in PressedKeyTimerProc before firing

**File:** `src/runner/centralized_kb_hook.cpp` (lines 59-80)

```cpp
// BEFORE:
    void PressedKeyTimerProc(
        HWND hwnd,
        UINT /*message*/,
        UINT_PTR idTimer,
        DWORD /*dwTime*/)
    {
        std::multiset<PressedKeyDescriptor> copy;
        {
            // Make a copy, to look for the action to call.
            std::unique_lock lock{ pressedKeyMutex };
            copy = pressedKeyDescriptors;
        }
        for (const auto& it : copy)
        {
            if (it.idTimer == idTimer)
            {
                it.action();
            }
        }

        KillTimer(hwnd, idTimer);
    }

// AFTER:
    void PressedKeyTimerProc(
        HWND hwnd,
        UINT /*message*/,
        UINT_PTR idTimer,
        DWORD /*dwTime*/)
    {
        std::multiset<PressedKeyDescriptor> copy;
        {
            std::unique_lock lock{ pressedKeyMutex };
            copy = pressedKeyDescriptors;
        }
        for (const auto& it : copy)
        {
            if (it.idTimer == idTimer)
            {
                // Revalidate: only fire if the key is still physically held down.
                // Prevents ghost activations after the key was already released.
                if (GetAsyncKeyState(static_cast<int>(it.virtualKey)) & 0x8000)
                {
                    it.action();
                }
            }
        }

        KillTimer(hwnd, idTimer);
    }
```

---

### PR 8 — B4: Harden Stop() to kill pending timers and reset state

**File:** `src/runner/centralized_kb_hook.cpp` (lines 264-270)

```cpp
// BEFORE:
    void Stop() noexcept
    {
        if (hHook && UnhookWindowsHookEx(hHook))
        {
            hHook = NULL;
        }
    }

// AFTER:
    void Stop() noexcept
    {
        if (hHook)
        {
            UnhookWindowsHookEx(hHook);
            hHook = NULL;
        }

        // Kill all active pressed-key timers and reset state
        {
            std::unique_lock lock{ pressedKeyMutex };
            if (runnerWindow)
            {
                for (const auto& desc : pressedKeyDescriptors)
                {
                    KillTimer(runnerWindow, desc.idTimer);
                }
            }
            vkCodePressed = VK_DISABLED;
        }
    }
```

---

### PR 9 — B5: Fix vkCodePressed race conditions

**File:** `src/runner/centralized_kb_hook.cpp`

Replace the global `DWORD vkCodePressed` (line 45) with `std::atomic<DWORD>`:

```cpp
// BEFORE (line 45):
    DWORD vkCodePressed = VK_DISABLED;

// AFTER:
    std::atomic<DWORD> vkCodePressed{ VK_DISABLED };
```

Then hoist the lock to cover the entire pressed-key section in `KeyboardHookProc` (lines 98-138):

```cpp
// BEFORE (lines 98-138):
        if (!pressedKeyDescriptors.empty())
        {
            bool wasKeyPressed = vkCodePressed != VK_DISABLED;
            // Hold the lock for the shortest possible duration
            if ((wParam == WM_KEYDOWN || wParam == WM_SYSKEYDOWN))
            {
                if (!wasKeyPressed)
                {
                    // If no key was pressed before, let's start a timer ...
                    std::unique_lock lock{ pressedKeyMutex };
                    PressedKeyDescriptor dummy{ .virtualKey = keyPressInfo.vkCode };
                    auto [it, last] = pressedKeyDescriptors.equal_range(dummy);
                    for (; it != last; ++it)
                    {
                        SetTimer(runnerWindow, it->idTimer, it->millisecondsToPress, PressedKeyTimerProc);
                    }
                }
                else if (vkCodePressed != keyPressInfo.vkCode)
                {
                    // If a different key was pressed, clear timers for previous key
                    std::unique_lock lock{ pressedKeyMutex };
                    PressedKeyDescriptor dummy{ .virtualKey = vkCodePressed };
                    auto [it, last] = pressedKeyDescriptors.equal_range(dummy);
                    for (; it != last; ++it)
                    {
                        KillTimer(runnerWindow, it->idTimer);
                    }
                }
                vkCodePressed = keyPressInfo.vkCode;
            }
            if (wParam == WM_KEYUP || wParam == WM_SYSKEYUP)
            {
                std::unique_lock lock{ pressedKeyMutex };
                PressedKeyDescriptor dummy{ .virtualKey = keyPressInfo.vkCode };
                auto [it, last] = pressedKeyDescriptors.equal_range(dummy);
                for (; it != last; ++it)
                {
                    KillTimer(runnerWindow, it->idTimer);
                }
                vkCodePressed = 0x100;
            }
        }

// AFTER:
        if (!pressedKeyDescriptors.empty())
        {
            std::unique_lock lock{ pressedKeyMutex };
            bool wasKeyPressed = vkCodePressed.load() != VK_DISABLED;

            if ((wParam == WM_KEYDOWN || wParam == WM_SYSKEYDOWN))
            {
                if (!wasKeyPressed)
                {
                    PressedKeyDescriptor dummy{ .virtualKey = keyPressInfo.vkCode };
                    auto [it, last] = pressedKeyDescriptors.equal_range(dummy);
                    for (; it != last; ++it)
                    {
                        SetTimer(runnerWindow, it->idTimer, it->millisecondsToPress, PressedKeyTimerProc);
                    }
                }
                else if (vkCodePressed.load() != keyPressInfo.vkCode)
                {
                    PressedKeyDescriptor dummy{ .virtualKey = vkCodePressed.load() };
                    auto [it, last] = pressedKeyDescriptors.equal_range(dummy);
                    for (; it != last; ++it)
                    {
                        KillTimer(runnerWindow, it->idTimer);
                    }
                }
                vkCodePressed.store(keyPressInfo.vkCode);
            }
            else if (wParam == WM_KEYUP || wParam == WM_SYSKEYUP)
            {
                PressedKeyDescriptor dummy{ .virtualKey = keyPressInfo.vkCode };
                auto [it, last] = pressedKeyDescriptors.equal_range(dummy);
                for (; it != last; ++it)
                {
                    KillTimer(runnerWindow, it->idTimer);
                }
                vkCodePressed.store(VK_DISABLED);
            }
        }
```

Add `#include <atomic>` to the includes if not already present.

---

## Stream C: Keyboard Manager Modifier Safety (P1)

**Owner:** 1 developer
**Estimated effort:** 2-3 days (higher complexity due to state machine)

### PR 10 — C1: Handle SendVirtualInput failure — restore original key

**File:** `src/modules/keyboardmanager/common/Input.h` (lines 15-25)

```cpp
// BEFORE:
        void SendVirtualInput(const std::vector<INPUT>& inputs)
        {
            std::vector<INPUT> copy = inputs;
            UINT eventCount = SendInput(static_cast<UINT>(copy.size()), copy.data(), sizeof(INPUT));
            if (eventCount != copy.size())
            {
                Logger::error(
                    L"Failed to send input events. {}",
                    get_last_error_or_default(GetLastError()));
            }
        }

// AFTER:
        // Returns true if all inputs were sent successfully.
        bool SendVirtualInput(const std::vector<INPUT>& inputs)
        {
            if (inputs.empty())
            {
                return true;
            }

            std::vector<INPUT> copy = inputs;
            UINT eventCount = SendInput(static_cast<UINT>(copy.size()), copy.data(), sizeof(INPUT));
            if (eventCount != copy.size())
            {
                Logger::error(
                    L"SendVirtualInput: only {}/{} events sent. {}",
                    eventCount, copy.size(),
                    get_last_error_or_default(GetLastError()));
                return false;
            }
            return true;
        }
```

> **Important:** `SendVirtualInput` is a virtual method defined in `InputInterface`. The return type change must also be applied to:
> - `src/modules/keyboardmanager/common/InputInterface.h` — change `virtual void SendVirtualInput(...)` → `virtual bool SendVirtualInput(...)`
> - Any mock/test implementations of `InputInterface`
>
> Callers that currently ignore the return value will still compile. High-priority callers (single key remap, shortcut remap) should check the return and fall through to `return 0` (pass key through) on failure instead of `return 1` (swallow key).

**Caller change example** — `src/modules/keyboardmanager/KeyboardManagerEngineLibrary/KeyboardEventHandlers.cpp`, single key remap path:

```cpp
// Where the handler currently does:
//   ii.SendVirtualInput(keyEventList);
//   return 1;  // swallow original
// Change to:
    if (!ii.SendVirtualInput(keyEventList))
    {
        // SendInput failed — don't swallow original key or it will be lost
        Logger::warn(L"SendInput failed for single key remap; passing original key through");
        return 0;
    }
    return 1;
```

Apply the same pattern to the shortcut remap handler paths.

---

### PR 11 — C2: Force state cleanup on foreground app change

**File:** `src/modules/keyboardmanager/KeyboardManagerEngineLibrary/KeyboardEventHandlers.cpp`

Add a foreground-app-change detector to the main handler entry point. When the foreground app changes between hook calls, inject key-up events for any modifiers that are logically "held" by the state machine but not physically held.

Near the top of `HandleShortcutRemapEvent` (and `HandleAppSpecificShortcutRemapEvent`), add:

```cpp
    // Detect foreground app change and reset stuck modifier state
    static std::wstring s_lastForegroundApp;
    std::wstring currentApp;
    ii.GetForegroundProcess(currentApp);

    if (!s_lastForegroundApp.empty() && s_lastForegroundApp != currentApp)
    {
        // App changed — any shortcut in-progress needs cleanup
        for (auto& [shortcut, remapState] : reMap)
        {
            if (remapState.isShortcutInvoked)
            {
                Logger::info(L"Foreground app changed while shortcut active — resetting state");
                // Release any target modifiers we injected
                std::vector<INPUT> releaseEvents;
                Helpers::SetModifierKeyEvents(
                    std::get<Shortcut>(remapState.targetShortcut),
                    remapState.modifierKeysInvoked,
                    releaseEvents,
                    false,  // keyUp
                    KeyboardManagerConstants::KEYBOARDMANAGER_SHORTCUT_FLAG);
                ii.SendVirtualInput(releaseEvents);

                remapState.isShortcutInvoked = false;
                remapState.modifierKeysInvoked.Reset();
                remapState.isOriginalActionKeyPressed = false;
            }
        }
    }
    s_lastForegroundApp = currentApp;
```

> **Note:** This adds a `GetForegroundProcess` call per hook invocation. This is a quick Win32 call and acceptable at LL hook frequency. If profiling shows impact, debounce by checking only on WM_KEYDOWN (not WM_KEYUP).

---

### C3: Protect settings reload with a reader-writer lock

**File:** `src/modules/keyboardmanager/KeyboardManagerEngineLibrary/KeyboardManager.cpp`

The `loadingSettings` atomic bool is already used but creates a window where settings are partially written. Replace with a shared mutex:

```cpp
// Add to KeyboardManager class members:
    mutable std::shared_mutex m_settingsLock;
```

**In the settings reload callback:**
```cpp
// BEFORE:
    loadingSettings = true;
    // ... LoadSettings() ...
    loadingSettings = false;

// AFTER:
    {
        std::unique_lock lock{ m_settingsLock };  // Exclusive lock for writes
        try
        {
            LoadSettings();
        }
        catch (...)
        {
            Logger::error("Failed to load settings");
        }
    }
    // Settings fully loaded, now safe for hook thread
```

**In HandleKeyboardHookEvent:**
```cpp
// BEFORE:
    if (loadingSettings)
    {
        return 0;
    }

// AFTER:
    std::shared_lock lock{ m_settingsLock };  // Shared lock for reads
    // (remove the loadingSettings check — the shared_mutex handles it)
```

> **Caution:** This is a higher-risk change since it introduces lock contention on the hot path. The shared_mutex allows concurrent reads (hook invocations) while only blocking during the rare write (settings reload). Profile after implementing.

---

### PR 13 — C4: Release source modifiers before text injection for single-key-to-text

**File:** `src/modules/keyboardmanager/KeyboardManagerEngineLibrary/KeyboardEventHandlers.cpp`

In `HandleSingleKeyToTextRemapEvent`, release modifiers before sending text:

```cpp
// BEFORE (in HandleSingleKeyToTextRemapEvent, near the SendTextInput call):
    Helpers::SendTextInput(*remapping);
    return 1;

// AFTER:
    // Release any held modifiers before injecting text to prevent
    // Ctrl+text, Alt+text, etc. corruption
    std::vector<INPUT> modReleaseEvents;
    const DWORD modifiers[] = { VK_LCONTROL, VK_RCONTROL, VK_LMENU, VK_RMENU, VK_LSHIFT, VK_RSHIFT, VK_LWIN, VK_RWIN };
    std::vector<DWORD> heldModifiers;
    for (DWORD mod : modifiers)
    {
        if (GetAsyncKeyState(static_cast<int>(mod)) & 0x8000)
        {
            heldModifiers.push_back(mod);
            INPUT input = {};
            input.type = INPUT_KEYBOARD;
            input.ki.wVk = static_cast<WORD>(mod);
            input.ki.dwFlags = KEYEVENTF_KEYUP;
            input.ki.dwExtraInfo = KeyboardManagerConstants::KEYBOARDMANAGER_SINGLEKEY_FLAG;
            modReleaseEvents.push_back(input);
        }
    }
    if (!modReleaseEvents.empty())
    {
        ii.SendVirtualInput(modReleaseEvents);
    }

    Helpers::SendTextInput(*remapping);

    // Re-press the modifiers so physical key state remains consistent
    std::vector<INPUT> modRestoreEvents;
    for (DWORD mod : heldModifiers)
    {
        INPUT input = {};
        input.type = INPUT_KEYBOARD;
        input.ki.wVk = static_cast<WORD>(mod);
        input.ki.dwFlags = 0; // key down
        input.ki.dwExtraInfo = KeyboardManagerConstants::KEYBOARDMANAGER_SINGLEKEY_FLAG;
        modRestoreEvents.push_back(input);
    }
    if (!modRestoreEvents.empty())
    {
        ii.SendVirtualInput(modRestoreEvents);
    }

    return 1;
```

---

## Stream D: PowerAccent Injection Hygiene (P1)

**Owner:** 1 developer
**Estimated effort:** 1 day

### PR 14 — D1: Add LLKHF_INJECTED filter to keyboard hook

**File:** `src/modules/poweraccent/PowerAccentKeyboardService/KeyboardListener.cpp` (lines 291-318)

```cpp
// BEFORE:
    LRESULT KeyboardListener::LowLevelKeyboardProc(int nCode, WPARAM wParam, LPARAM lParam)
    {
        if (nCode == HC_ACTION && s_instance != nullptr)
        {
            KBDLLHOOKSTRUCT* key = reinterpret_cast<KBDLLHOOKSTRUCT*>(lParam);
            switch (wParam)
            {

// AFTER:
    LRESULT KeyboardListener::LowLevelKeyboardProc(int nCode, WPARAM wParam, LPARAM lParam)
    {
        if (nCode == HC_ACTION && s_instance != nullptr)
        {
            KBDLLHOOKSTRUCT* key = reinterpret_cast<KBDLLHOOKSTRUCT*>(lParam);

            // Ignore injected events (from KBM, other modules, OSK, macro tools).
            // This prevents KBM-remapped letters from triggering accent mode.
            if (key->flags & LLKHF_INJECTED)
            {
                return CallNextHookEx(NULL, nCode, wParam, lParam);
            }

            switch (wParam)
            {
```

---

### PR 15 — D2: Tag output SendInput calls with dwExtraInfo

**File:** `src/modules/poweraccent/PowerAccent.Core/Tools/WindowsFunctions.cs`

Add a constant and set `dwExtraInfo` on all SendInput calls:

```csharp
// Add at class level:
    // Must match PowertoyModuleIface::CENTRALIZED_KEYBOARD_HOOK_DONT_TRIGGER_FLAG (0x110)
    private const nuint POWERTOYS_INJECTED_TAG = 0x110;
```

**Change the backspace INPUT (lines 30-33):**
```csharp
// BEFORE:
                            ki = new KEYBDINPUT
                            {
                                wVk = VIRTUAL_KEY.VK_BACK,
                            },

// AFTER:
                            ki = new KEYBDINPUT
                            {
                                wVk = VIRTUAL_KEY.VK_BACK,
                                dwExtraInfo = POWERTOYS_INJECTED_TAG,
                            },
```

**Change the backspace KEYUP INPUT (lines 41-44):**
```csharp
// BEFORE:
                            ki = new KEYBDINPUT
                            {
                                wVk = VIRTUAL_KEY.VK_BACK,
                                dwFlags = KEYBD_EVENT_FLAGS.KEYEVENTF_KEYUP,
                            },

// AFTER:
                            ki = new KEYBDINPUT
                            {
                                wVk = VIRTUAL_KEY.VK_BACK,
                                dwFlags = KEYBD_EVENT_FLAGS.KEYEVENTF_KEYUP,
                                dwExtraInfo = POWERTOYS_INJECTED_TAG,
                            },
```

**Change the character INPUT (lines 64-68):**
```csharp
// BEFORE:
                            ki = new KEYBDINPUT
                            {
                                wScan = c,
                                dwFlags = KEYBD_EVENT_FLAGS.KEYEVENTF_UNICODE,
                            },

// AFTER:
                            ki = new KEYBDINPUT
                            {
                                wScan = c,
                                dwFlags = KEYBD_EVENT_FLAGS.KEYEVENTF_UNICODE,
                                dwExtraInfo = POWERTOYS_INJECTED_TAG,
                            },
```

**Change the character KEYUP INPUT (lines 76-80):**
```csharp
// BEFORE:
                            ki = new KEYBDINPUT
                            {
                                wScan = c,
                                dwFlags = KEYBD_EVENT_FLAGS.KEYEVENTF_UNICODE | KEYBD_EVENT_FLAGS.KEYEVENTF_KEYUP,
                            },

// AFTER:
                            ki = new KEYBDINPUT
                            {
                                wScan = c,
                                dwFlags = KEYBD_EVENT_FLAGS.KEYEVENTF_UNICODE | KEYBD_EVENT_FLAGS.KEYEVENTF_KEYUP,
                                dwExtraInfo = POWERTOYS_INJECTED_TAG,
                            },
```

---

### PR 16 — D3: Add focus-loss and session-change cleanup

**File:** `src/modules/poweraccent/PowerAccentKeyboardService/KeyboardListener.cpp`

Add a public method to force-reset all state:

```cpp
    void KeyboardListener::ForceReset() noexcept
    {
        if (m_toolbarVisible)
        {
            Logger::info(L"PowerAccent: ForceReset — hiding toolbar");
            m_hideToolbarCb(InputType::None);
        }
        m_toolbarVisible = false;
        letterPressed = LetterKey::None;
        m_triggeredWithSpace = false;
        m_triggeredWithLeftArrow = false;
        m_triggeredWithRightArrow = false;
        m_leftShiftPressed = false;
        m_rightShiftPressed = false;
    }
```

Add declaration to `KeyboardListener.h`:
```cpp
    void ForceReset() noexcept;
```

**Usage:** Call `ForceReset()` from:
- The C# `Selector.xaml.cs` `OnDeactivated` handler
- WM_WTSSESSION_CHANGE (if wired up in Stream E)
- Module disable path

In `Selector.xaml.cs`, add:
```csharp
    protected override void OnDeactivated(EventArgs e)
    {
        base.OnDeactivated(e);
        // Force-hide toolbar on focus loss to prevent stuck state
        _powerAccent?.ForceResetFromUI();
    }
```

---

### PR 17 — D4: Replace SendKeys.SendWait with tagged SendInput for arrow keys

**File:** `src/modules/poweraccent/PowerAccent.Core/PowerAccent.cs` (lines 208-217)

```csharp
// BEFORE:
            case InputType.Right:
                {
                    SendKeys.SendWait("{RIGHT}");
                    break;
                }

            case InputType.Left:
                {
                    SendKeys.SendWait("{LEFT}");
                    break;
                }

// AFTER:
            case InputType.Right:
                {
                    WindowsFunctions.SendArrowKey(VIRTUAL_KEY.VK_RIGHT);
                    break;
                }

            case InputType.Left:
                {
                    WindowsFunctions.SendArrowKey(VIRTUAL_KEY.VK_LEFT);
                    break;
                }
```

**Add to `WindowsFunctions.cs`:**
```csharp
    public static void SendArrowKey(VIRTUAL_KEY key)
    {
        var inputs = new INPUT[]
        {
            new INPUT
            {
                type = INPUT_TYPE.INPUT_KEYBOARD,
                Anonymous = new INPUT._Anonymous_e__Union
                {
                    ki = new KEYBDINPUT
                    {
                        wVk = key,
                        dwExtraInfo = POWERTOYS_INJECTED_TAG,
                    },
                },
            },
            new INPUT
            {
                type = INPUT_TYPE.INPUT_KEYBOARD,
                Anonymous = new INPUT._Anonymous_e__Union
                {
                    ki = new KEYBDINPUT
                    {
                        wVk = key,
                        dwFlags = KEYBD_EVENT_FLAGS.KEYEVENTF_KEYUP,
                        dwExtraInfo = POWERTOYS_INJECTED_TAG,
                    },
                },
            },
        };

        _ = PInvoke.SendInput(inputs, Marshal.SizeOf<INPUT>());
    }
```

Remove `using System.Windows.Forms;` import if `SendKeys` was the only user.

---

## Stream E: Cross-Module Safety Net (P2)

**Owner:** 1 developer
**Estimated effort:** 1-2 days

### PR 18 — E1: Add shared injection tag constant

**File:** `src/common/interop/shared_constants.h`

```cpp
// Add after KEYBOARDMANAGER_INJECTED_FLAG (line 8):

    // Tag set by any PowerToys module on injected input to prevent cross-module interference.
    // Modules should check this in their LL hooks to skip events from sibling modules.
    const uintptr_t POWERTOYS_INJECTED_FLAG = 0x110;
    // NOTE: This equals CENTRALIZED_KEYBOARD_HOOK_DONT_TRIGGER_FLAG for backward compat.
    // Modules that previously used the flag from PowertoyModuleIface should migrate to this.
```

This unifies the existing `CENTRALIZED_KEYBOARD_HOOK_DONT_TRIGGER_FLAG` (0x110) with a clearer cross-module name. Existing code using the old constant continues to work since the value is the same.

---

### PR 19 — E2: Add session-change cleanup to centralized hook

**File:** `src/runner/centralized_kb_hook.cpp`

Add a new function to handle session changes:

```cpp
    void OnSessionChange(DWORD reason) noexcept
    {
        // On lock/disconnect, release any pending pressed-key timers to prevent
        // ghost activations when the user returns.
        if (reason == WTS_SESSION_LOCK || reason == WTS_SESSION_LOGOFF ||
            reason == WTS_SESSION_REMOTE_DISCONNECT)
        {
            Logger::info(L"CentralizedKBHook: session change ({}) — clearing pressed key state", reason);
            std::unique_lock lock{ pressedKeyMutex };
            if (runnerWindow)
            {
                for (const auto& desc : pressedKeyDescriptors)
                {
                    KillTimer(runnerWindow, desc.idTimer);
                }
            }
            vkCodePressed.store(VK_DISABLED);
        }
    }
```

**File:** `src/runner/centralized_kb_hook.h` — add declaration:

```cpp
    void OnSessionChange(DWORD reason) noexcept;
```

**Wire up in the runner's window proc** — wherever WM_WTSSESSION_CHANGE is handled (or add it):

```cpp
    case WM_WTSSESSION_CHANGE:
        CentralizedKeyboardHook::OnSessionChange(static_cast<DWORD>(wParam));
        break;
```

Make sure `WTSRegisterSessionNotification(hwnd, NOTIFY_FOR_THIS_SESSION)` is called during runner window creation. Search for existing WTS registration in the runner and add if missing.

---

## Test Plan

> **Note:** Each PR above includes inline "How to Reproduce" and "How to Verify" sections.
> Below is the consolidated regression test matrix and automated test ideas.

### Manual Regression Tests

#### FancyZones (Stream A)

| Test | Steps | Expected |
|------|-------|----------|
| **A1: Window close mid-drag** | 1. Enable FancyZones with Shift-drag mode. 2. Start dragging a window (hold Shift). 3. While dragging, press Alt+F4 to close the window. 4. Open Notepad and type "Hello 123". | Text types normally. No keys stolen. Zone overlay disappears. |
| **A1: Process kill mid-drag** | 1. Start dragging a window. 2. Use Task Manager to kill the dragged process. 3. Try typing. | Keyboard works normally after kill. |
| **A2: Shift+key during drag** | 1. Start a FancyZones drag. 2. While dragging, press Shift+A in another window. | Only bare Shift is suppressed (drag toggle). Shift+A passes through. |
| **A3: Number keys after stuck drag** | 1. (If A1 fix not applied) Force stuck drag state. 2. Press number keys 0-9. | Number keys pass through to the app. No layout switch without Win+Ctrl+Alt. |
| **A: Normal drag works** | 1. Hold Shift and drag a window. 2. Release. 3. Drag without Shift. | Zone highlights appear/disappear correctly based on Shift state. |

#### Centralized Hook (Stream B)

| Test | Steps | Expected |
|------|-------|----------|
| **B1: Hotkey after dummy key** | 1. Set a PowerToys hotkey (e.g., Win+Shift+C for Color Picker). 2. Press the hotkey 50 times rapidly. | Hotkey fires reliably. No ghost activations. Start menu never appears. |
| **B2: On-Screen Keyboard** | 1. Open Windows OSK. 2. Press keys on OSK. 3. Verify PowerToys hotkeys don't trigger from OSK input. | OSK input passes through. No hotkey false triggers. |
| **B3: Pressed-key timer** | 1. Configure a pressed-key action (e.g., long-press for CmdPal). 2. Press and quickly release the key before the timer. | Timer does NOT fire after release. No ghost activation. |
| **B4: Module disable** | 1. Disable a module that uses centralized hook. 2. Re-enable it. | No stale timers. No leftover state. |

#### Keyboard Manager (Stream C)

| Test | Steps | Expected |
|------|-------|----------|
| **C1: SendInput failure** | 1. Remap A→B. 2. Type 'A' normally. 3. (Hard to repro failure, but verify logging.) | If SendInput fails, original 'A' passes through instead of being lost. |
| **C2: Focus change mid-shortcut** | 1. Remap Ctrl+A → Ctrl+C. 2. Press Ctrl+A. 3. While holding Ctrl, Alt+Tab to another app. 4. Release all keys. 5. Type normally. | No stuck Ctrl or other modifiers. Typing works normally in new app. |
| **C3: Settings reload** | 1. While using KBM actively, change a remap in settings. 2. Continue typing. | No crash, no partial remap state. Settings apply cleanly. |
| **C4: Text remap with modifier** | 1. Remap key X → text "hello". 2. Hold Ctrl and press X. | "hello" is typed as plain text, not "Ctrl+h, Ctrl+e, ..." |

#### PowerAccent (Stream D)

| Test | Steps | Expected |
|------|-------|----------|
| **D1: KBM remap to letter** | 1. In KBM, remap Q→A. 2. Press and hold Q, then press Space. | Accent toolbar does NOT appear (injected 'A' is filtered). |
| **D2: Accent output no cross-trigger** | 1. Set a PowerToys hotkey on a letter key. 2. Use PowerAccent to insert an accented char. | Hotkey does not fire from accent output. |
| **D3: Alt+Tab during accent** | 1. Trigger accent toolbar. 2. Press Alt+Tab. | Toolbar hides. State resets. No stuck accent mode. |
| **D4: Arrow key navigation** | 1. Trigger accent toolbar. 2. Use Left/Right arrows to select. | Arrow navigation works. No blocking. Correct char inserted. |

### Automated Test Ideas

| Area | Test Type | Description |
|------|-----------|-------------|
| FancyZones | Unit | Call `MoveSizeStart()`, simulate `EVENT_OBJECT_DESTROY`, verify `IsDragging() == false` |
| FancyZones | Unit | Call `OnKeyDown` with Shift+A during drag, verify returns `false` (not swallowed) |
| FancyZones | Unit | Call `OnKeyDown` with bare VK_LSHIFT during drag, verify returns `true` |
| Centralized Hook | Unit | Verify `PressedKeyTimerProc` calls `GetAsyncKeyState` and skips action when key is up |
| Centralized Hook | Unit | Verify `Stop()` kills all pending timers and resets `vkCodePressed` |
| KBM | Unit | Call `HandleSingleKeyToTextRemapEvent` with Ctrl held, verify Ctrl is released before text |
| KBM | Unit | Mock `SendVirtualInput` to return false, verify hook returns 0 (pass-through) |
| PowerAccent | Unit | Call `LowLevelKeyboardProc` with `LLKHF_INJECTED` flag, verify `CallNextHookEx` is called |
| Cross-module | Integration | Inject tagged input (dwExtraInfo=0x110), verify all hooks pass it through |

---

## Risk Assessment

| Change | Risk | Rationale |
|--------|------|-----------|
| **A1: EVENT_OBJECT_DESTROY** | 🟢 Low | Additive — new event subscription + new handler. No existing paths changed. |
| **A2: Shift swallow fix** | 🟢 Low | Narrowing a condition. If wrong, worst case is Shift-drag toggle stops working (easily noticed in testing). |
| **A3: Digit modifier requirement** | 🟢 Low | Strictly additive condition. Non-dragging path unchanged. |
| **A4: SwallowKey tagging** | 🟢 Low | Adding dwExtraInfo to an unused function. No behavioral change. |
| **B1: Dummy key tag** | 🟢 Low | One-line addition. Prevents re-entry. |
| **B2: LLKHF_INJECTED filter** | 🟡 Medium | Could break workflows where users intentionally use macro tools with PowerToys hotkeys. Monitor feedback. Consider making configurable later. |
| **B3: Timer revalidation** | 🟢 Low | Additive check. Worst case: pressed-key action doesn't fire (better than false fire). |
| **B4: Stop() hardening** | 🟢 Low | Adding cleanup that was missing. |
| **B5: vkCodePressed atomic** | 🟢 Low | Mechanical — same logic, proper synchronization. |
| **C1: SendVirtualInput return** | 🟡 Medium | Interface change. Must update all implementations. But behavior change is pass-through-on-failure, which is safer than swallow-on-failure. |
| **C2: Focus change cleanup** | 🟡 Medium | Adds GetForegroundProcess to hot path. Could have perf impact. Adds state transitions that need thorough testing. |
| **C3: Settings lock** | 🟠 Higher | Introduces shared_mutex on LL hook hot path. Must verify no deadlock with other locks. Profile latency. |
| **C4: Text remap modifier release** | 🟢 Low | Well-understood pattern (shortcut-to-text already does this). |
| **D1: LLKHF_INJECTED filter** | 🟢 Low | Additive early-out. |
| **D2: dwExtraInfo tagging** | 🟢 Low | One field set on existing SendInput calls. |
| **D3: Focus loss cleanup** | 🟢 Low | Additive handler. |
| **D4: SendKeys→SendInput** | 🟡 Medium | Behavioral change in arrow key injection. Must verify all apps handle it the same. |
| **E1: Shared constant** | 🟢 Low | Additive constant definition. |
| **E2: Session change hook** | 🟢 Low | Additive handler for rare event. |

### Recommended Implementation Order

1. **Stream A** (FancyZones) — Highest user impact, lowest risk, most self-contained
2. **Stream B** (Centralized Hook) — All low risk, fixes foundation for other modules
3. **Stream D** (PowerAccent) — Simple, self-contained, no cross-module dependencies
4. **Stream E** (Cross-Module) — Depends on B being done first
5. **Stream C** (Keyboard Manager) — Most complex, highest risk, benefits from A+B being stable

### Dependencies Between Streams

```
Stream A ──→ (independent)
Stream B ──→ (independent, but should land before E)
Stream C ──→ (independent, but benefits from E1 shared constant)
Stream D ──→ (independent, but benefits from E1 shared constant)
Stream E ──→ depends on B (for session-change wiring into centralized hook)
```

All five streams can be developed in **parallel** since they touch different files. Integration testing should happen after all streams merge.

