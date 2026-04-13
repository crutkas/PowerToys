# Stuck Key Unit Test Specification

## Search summary

- `centralized_kb_hook` in test files: only `src\StuckKeyTestPlan.md`
- `PowerAccent` in test files: `src\StuckKeyTestPlan.md` plus unrelated C# settings/DSC tests
- `src\runner\`: no test directory or test project
- `src\modules\poweraccent\`: no test directory or test project

Because there is no module-local test harness for either target, the tests below could not be added as runnable unit tests in this branch without first introducing new test projects and test seams.

## Recommended test project locations

### Centralized hook

- Project: `src\runner\CentralizedKeyboardHook.UnitTests\CentralizedKeyboardHook.UnitTests.vcxproj`
- Framework: native MSTest (`NativeUnitTestProject`), mirroring `src\modules\keyboardmanager\KeyboardManagerEngineTest\KeyboardManagerEngineTest.vcxproj`
- Suggested test files:
  - `CentralizedKeyboardHookTimerTests.cpp`
  - `CentralizedKeyboardHookLifecycleTests.cpp`
  - `CentralizedKeyboardHookInjectedEventTests.cpp`

### PowerAccent

- Project: `src\modules\poweraccent\PowerAccentKeyboardService.Tests\PowerAccentKeyboardService.Tests.vcxproj`
- Framework: native MSTest with C++/WinRT enabled
- Suggested test files:
  - `KeyboardListenerInjectedEventTests.cpp`
  - `KeyboardListenerResetTests.cpp`

## Test seams needed before implementation

### Centralized hook

Add internal/test-only seams for:

- `GetAsyncKeyState`
- `SetTimer`
- `KillTimer`
- `CallNextHookEx`
- `SendInput`
- access to `pressedKeyDescriptors`, `vkCodePressed`, and `runnerWindow`

Without those seams, the current implementation is tightly coupled to process-global Win32 state and timer callbacks.

### PowerAccent

Add internal/test-only seams for:

- `CallNextHookEx`
- optional foreground/game mode checks if `OnKeyDown` is exercised directly
- controlled access to listener state (`letterPressed`, toolbar flags, shift flags)

`KeyboardListener` already exposes callback registration methods, so most observable behavior can be asserted through lambdas once the hook-forwarding seam exists.

## Tests that should be written

### 1. Centralized hook: `PressedKeyTimerProc` revalidates key state

- **File:** `CentralizedKeyboardHookTimerTests.cpp`
- **Setup:** register a pressed-key action for one VK and capture whether the action runs
- **Arrange:** fake `GetAsyncKeyState(vk)` returns "not pressed" when the timer callback executes
- **Act:** call `PressedKeyTimerProc(...)` with the registered timer id
- **Assert:**
  - action is **not** invoked
  - `KillTimer` is still called for that timer id

### 2. Centralized hook: `Stop()` kills timers and resets `vkCodePressed`

- **File:** `CentralizedKeyboardHookLifecycleTests.cpp`
- **Setup:** register multiple pressed-key actions, set `vkCodePressed` to a non-disabled value
- **Act:** call `Stop()`
- **Assert:**
  - `KillTimer` is called once per registered timer id
  - `vkCodePressed == CommonSharedConstants::VK_DISABLED`
  - hook handle is cleared when `UnhookWindowsHookEx` succeeds

### 3. Centralized hook: injected events are passed through

- **File:** `CentralizedKeyboardHookInjectedEventTests.cpp`
- **Setup:** create a `KBDLLHOOKSTRUCT` with `flags |= LLKHF_INJECTED` and `dwExtraInfo == 0`
- **Act:** call `KeyboardHookProc(HC_ACTION, WM_KEYDOWN, ...)`
- **Assert:**
  - return value matches the fake `CallNextHookEx` result
  - no hotkey action runs
  - no timer is created or killed

### 4. PowerAccent: injected events do not trigger accent mode

- **File:** `KeyboardListenerInjectedEventTests.cpp`
- **Setup:**
  - create a `KeyboardListener`
  - register `ShowToolbar`/`HideToolbar` callbacks that record calls
  - pass an injected letter or trigger key to `LowLevelKeyboardProc`
- **Act:** call `LowLevelKeyboardProc(HC_ACTION, WM_KEYDOWN, ...)` with `LLKHF_INJECTED`
- **Assert:**
  - return value matches the fake `CallNextHookEx` result
  - toolbar callbacks are never invoked
  - listener state remains idle (`letterPressed == None`, toolbar hidden)

### 5. PowerAccent: `ForceReset()` clears all state

- **File:** `KeyboardListenerResetTests.cpp`
- **Setup:** drive the listener into an active state:
  - set a letter key
  - mark toolbar visible
  - mark shift and trigger flags true
  - register a `HideToolbar` callback that records the input type
- **Act:** call `ForceReset()`
- **Assert:**
  - `letterPressed == LetterKey::None`
  - toolbar visibility is false
  - `m_triggeredWithSpace`, `m_triggeredWithLeftArrow`, `m_triggeredWithRightArrow` are false
  - `m_leftShiftPressed` and `m_rightShiftPressed` are false
  - `HideToolbar` is invoked once with `InputType::None`

## Suggested acceptance criteria

- New unit test projects build in `Debug|x64` and `Release|x64`
- Tests run under `vstest.console.exe`
- No test relies on real keyboard hooks, real timers, or desktop focus
- All Win32 interactions are mocked or routed through test seams
