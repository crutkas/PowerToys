# DETAILED CROPANDLOCK PORT PLAN FOR RUST

## EXECUTIVE SUMMARY

CropAndLock has **THREE DISTINCT RENDERING MODES** that must be ported with exact UX parity:

1. **THUMBNAIL MODE** - DWM thumbnail rendering (live window preview via `DwmRegisterThumbnail`)
2. **REPARENT MODE** - Window hierarchy manipulation (native `SetParent` reparenting)  
3. **SCREENSHOT MODE** - Bitmap capture & render (static snapshot via `PrintWindow` + `BitBlt`)

All three modes share:
- **Identical overlay UI** - WinRT Composition-based selection rectangle with red border
- **Same hotkey system** - Win+Ctrl+Shift+R/T/S (configurable via JSON settings)
- **Event-driven activation** - Named events from Module DLL
- **Single message pump** - Win32 message loop + WinRT DispatcherQueue

---

## 1. TWO ACTIVATION LAYERS (Not Just UI!)

### Layer 1: HOTKEY RECOGNITION (Module DLL - C++)
```
User presses Win+Ctrl+Shift+R
    ↓
CropAndLockModuleInterface.dll (running in runner process) intercepts
    ↓
on_hotkey() called with hotkey ID (0=Reparent, 1=Thumbnail, 2=Screenshot)
    ↓
SetEvent() on appropriate named event:
    - CROP_AND_LOCK_REPARENT_EVENT
    - CROP_AND_LOCK_THUMBNAIL_EVENT  
    - CROP_AND_LOCK_SCREENSHOT_EVENT
```

### Layer 2: RENDERING (CropAndLock.exe - Rust)
```
CropAndLock.exe background thread waiting on MsgWaitForMultipleObjects()
    ↓
Event is set, thread wakes up
    ↓
dispatcher_queue.TryEnqueue() → UI thread
    ↓
ProcessCommand(mode) executes
    ↓
GetForegroundWindow() → get target window
    ↓
Create OverlayWindow (if not already cropped window)
    ↓
[Rest is selection UI...]
```

---

## 2. COMPLETE USER FLOW (All Three Modes)

### Shared Selection Flow
```
Mode: OVERLAY SELECTION
━━━━━━━━━━━━━━━━━━━━━━━━

1. OverlayWindow Created
   - Full-screen window (union of all displays)
   - Alpha-blended, HWND_TOPMOST
   - Covers entire desktop
   - WinRT Composition visual tree attached

2. OverlayWindow Visual Structure
   Root Visual (100% screen size)
   ├─ Shade Visual (semi-transparent black overlay)
   │  └─ NineGridBrush (hollow center shows window)
   │     - Left/Right/Top/Bottom insets = window bounds
   │     - Center is hollow (60% opacity)
   └─ Window Area Visual (positioned at target window client area)
      └─ Selection Visual (red border, initially invisible)
         └─ NineGridBrush (5px border, hollow center)

3. User Interaction (Mouse)
   - WM_MOUSEMOVE: Update cursor (crosshair when over target window)
   - WM_LBUTTONDOWN: Start selection (within window bounds)
     → m_cropStatus = Ongoing
     → m_startPosition = (x, y)
     → Update Selection Visual offset
   - WM_MOUSEMOVE (while dragging): Update Selection Visual size/position
     → Compute width, height as user drags
     → NineGrid border stretches with box
   - WM_LBUTTONUP: Finalize selection
     → Compute final RECT (normalized)
     → Validate (non-zero dimensions)
     → Invoke callback(HWND target, RECT crop)
     → Hide overlay
     → Exit

4. Callback → Create Cropped Window
   Based on mode selection at hotkey time:
   ├─ Reparent: Create ReparentCropAndLockWindow
   ├─ Thumbnail: Create ThumbnailCropAndLockWindow
   └─ Screenshot: Create ScreenshotCropAndLockWindow
```

### MODE-SPECIFIC: THUMBNAIL
```
ThumbnailCropAndLockWindow::CropAndLock(HWND target, RECT crop)
    ↓
1. Disconnect any existing thumbnail (DwmUnregisterThumbnail)
    ↓
2. Get target window EXTENDED FRAME BOUNDS
   DwmGetWindowAttribute(target, DWMWA_EXTENDED_FRAME_BOUNDS)
   → Returns full window rect including invisible DWM margins
    ↓
3. Adjust crop rect from CLIENT space to WINDOW FRAME space
   crop_frame = crop_client + (frame_offset_x, frame_offset_y)
    ↓
4. Resize our window to crop dimensions
   SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, width, height, SWP_NOMOVE|SWP_SHOWWINDOW)
    ↓
5. Register thumbnail with DWM
   DwmRegisterThumbnail(our_window, target, &thumbnail_handle)
   → Creates live link (GPU-accelerated)
    ↓
6. Configure thumbnail rendering
   DwmUpdateThumbnailProperties(thumbnail, {
       dwFlags: DWM_TNP_SOURCECLIENTAREAONLY | DWM_TNP_VISIBLE | DWM_TNP_OPACITY 
                | DWM_TNP_RECTDESTINATION | DWM_TNP_RECTSOURCE,
       fSourceClientAreaOnly: false,  (rcSource in frame coords, not client)
       rcSource: crop_frame,           (region in target window to show)
       rcDestination: client_area,     (where to render in our window)
       opacity: 255,                   (opaque)
       fVisible: true
   })
    ↓
7. WM_SIZE/WM_SIZING handler
   → Recompute aspect-ratio-preserving scale factor
   → Adjust rcDestination to center cropped content
   → Call DwmUpdateThumbnailProperties again
   → DWM recomposes automatically
    ↓
RESULT: Live thumbnail display
        If target window moves → thumbnail doesn't follow (source doesn't)
        If target resizes → thumbnail updates
        If target minimized → thumbnail grays out
        LIVE UPDATE: No explicit painting needed!
```

### MODE-SPECIFIC: REPARENT
```
ReparentCropAndLockWindow::CropAndLock(HWND target, RECT crop)
    ↓
1. Save original window state
   original_style = GetWindowLongPtr(target, GWL_STYLE)
   original_ex_style = GetWindowLongPtr(target, GWL_EXSTYLE)
   GetWindowPlacement(target, &original_placement)
   GetWindowRect(target, &original_rect)
    ↓
2. Create child window (to contain cropped target)
   ChildWindow::new(crop_width, crop_height, our_window)
   → Creates WS_CHILD window with no title bar
    ↓
3. Reparent target window to our child window
   SetParent(target, child_window)
   → Changes window parent, moves to our hierarchy
    ↓
4. Modify target window style
   style |= WS_CHILD          (marks as child, hides from taskbar)
   SetWindowLongPtr(target, GWL_STYLE, style)
    ↓
5. Position target at offset within child
   SetWindowPos(target, nullptr, -crop.left, -crop.top, 0, 0, SWP_NOSIZE|SWP_FRAMECHANGED)
   → Negative offsets hide left/top portion of target
   → Child window frame clips the content
    ↓
6. RESULT: Target window visible only in cropped region
   - User can move our window (target moves with it)
   - User can resize our window (crops more or less)
   - Target stays at same Z-order relative to us
    ↓
7. On close (destructor or user action)
   RestoreOriginalState():
   → SetWindowPos(target, nullptr, original_rect.left, original_rect.top, ...)
   → SetParent(target, nullptr)  (reparent back to desktop)
   → style &= ~WS_CHILD
   → SetWindowLongPtr(target, GWL_STYLE, style)
   → SetWindowPlacement(target, &original_placement)
   → Restore original ex_style
    ↓
RESULT: Target window back to original state
        DESTRUCTIVE but fully reversible
        If our window crashes, target is left reparented (problem!)
```

### MODE-SPECIFIC: SCREENSHOT
```
ScreenshotCropAndLockWindow::CropAndLock(HWND target, RECT crop)
    ↓
1. If already captured, exit early
    ↓
2. Get target window extended frame bounds
   DwmGetWindowAttribute(target, DWMWA_EXTENDED_FRAME_BOUNDS)
    ↓
3. Adjust crop to frame space
    ↓
4. Create two compatible DCs (device contexts)
   fullDC = CreateCompatibleDC(nullptr)
   cropDC = CreateCompatibleDC(nullptr)
    ↓
5. Render full window to bitmap
   fullBitmap = CreateCompatibleBitmap(screenDC, full_width, full_height)
   SelectObject(fullDC, fullBitmap)
   PrintWindow(target, fullDC, PW_RENDERFULLCONTENT)
   → Captures entire window (including title bar, etc.)
   → Non-destructive, works even with minimized windows
    ↓
6. Crop to selected region via BitBlt
   cropBitmap = CreateCompatibleBitmap(screenDC, crop_width, crop_height)
   SelectObject(cropDC, cropBitmap)
   BitBlt(cropDC, 0, 0, crop_width, crop_height,
          fullDC, crop_rect.left, crop_rect.top, SRCCOPY)
   → Copies cropped rectangle from full bitmap to cropped bitmap
    ↓
7. Cleanup full bitmap, keep cropped
   DeleteObject(fullBitmap)
   DeleteDC(fullDC)
   m_bitmap = cropBitmap
    ↓
8. Resize our window to crop dimensions
   SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, width, height, SWP_NOMOVE|SWP_SHOWWINDOW)
    ↓
9. WM_PAINT handler
   BeginPaint() → get window DC
   memDC = CreateCompatibleDC(windowDC)
   SelectObject(memDC, m_bitmap)
   StretchBlt(windowDC, offsetX, offsetY, drawWidth, drawHeight,
              memDC, 0, 0, bmp_width, bmp_height, SRCCOPY)
   → Preserves aspect ratio, centers in window
   → If user resizes window, paint handler recomputes scale/offset
    ↓
RESULT: Static bitmap display
        No live updates
        Works with any window (even if minimized)
        If target deleted, image persists
```

---

## 3. WIN32 API INVENTORY (Grouped by Purpose)

### A. WINDOW CREATION & LIFECYCLE
```
RegisterClassExW()              -- Register window class (once per mode)
CreateWindowExW()               -- Create main/child/overlay windows
AdjustWindowRectEx()            -- Frame size calculation
AdjustWindowRectExForDpi()      -- DPI-aware frame size
SetWindowPos()                  -- Position, size, z-order
ShowWindow() / UpdateWindow()   -- Visibility
DestroyWindow()                 -- Destroy (calls WM_DESTROY)
IsWindow(hwnd)                  -- Validate window still exists
```

### B. WINDOW PROPERTY QUERY/MODIFY
```
GetWindowLongPtr(hwnd, GWL_STYLE)        -- Window style flags
GetWindowLongPtr(hwnd, GWL_EXSTYLE)      -- Extended style flags
SetWindowLongPtr(hwnd, offset, value)    -- Modify window properties
GetWindowPlacement()                      -- Position + max/min/show state
SetWindowPlacement()                      -- Restore position + state
GetWindowRect(hwnd, &rect)               -- Full window bounds (screen coords)
GetClientRect(hwnd, &rect)               -- Client area bounds (window coords)
GetForegroundWindow()                    -- Currently active window
SetForegroundWindow(hwnd)                -- Activate window
GetMonitorInfo(monitor)                  -- Monitor bounds
MonitorFromWindow()                      -- Get monitor HMONITOR for window
```

### C. COORDINATE SYSTEM TRANSFORMS
```
ClientToScreen(hwnd, &point)            -- Convert client coords to screen
ScreenToClient(hwnd, &point)            -- Convert screen coords to client
GetDpiForMonitor(monitor, &dpiX, &dpiY) -- Query monitor DPI
```

### D. DWM (DESKTOP WINDOW MANAGER) - THUMBNAIL MODE
```
DwmRegisterThumbnail(parent, source, &thumbnail)
    → Creates HTHUMBNAIL handle linking source window to parent
    → GPU-accelerated, maintains live link
    → Returns HRESULT

DwmUpdateThumbnailProperties(thumbnail, &properties)
    → Set source region (rcSource), dest region (rcDestination)
    → Set opacity (0-255)
    → Set visibility flag
    → Set "source client area only" flag
    → Recomposes immediately

DwmUnregisterThumbnail(thumbnail)
    → Closes thumbnail link
    → Called automatically in unique_hthumbnail RAII wrapper

DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &rect, sizeof)
    → Get full window bounds including invisible DWM margins
    → Used to detect frame offset for crop adjustment
```

### E. WINDOW REPARENTING - REPARENT MODE
```
SetParent(hwnd_child, hwnd_new_parent)
    → Change window parent
    → Moves window to new hierarchy
    → Updates z-order relative to new parent
    → Triggers WM_WINDOWPOSCHANGED

GetWindowLongPtr() + SetWindowLongPtr(GWL_STYLE, WS_CHILD)
    → Add/remove WS_CHILD flag
    → Controls whether window is managed as child
```

### F. SCREENSHOT CAPTURE - SCREENSHOT MODE
```
PrintWindow(hwnd, hdc, flags)
    → Render window to device context
    → PW_RENDERFULLCONTENT = render all pixels (even behind other windows)
    → Works with minimized windows
    → Output is bitmap in hdc

CreateCompatibleDC(hdc)
    → Create memory device context (off-screen drawing)
    
CreateCompatibleBitmap(hdc, width, height)
    → Create bitmap compatible with hdc
    → Initially filled with 0s (black)
    
SelectObject(hdc, hbitmap)
    → Select bitmap into DC for drawing
    → Returns previous object (must clean up)
    
BitBlt(dest_dc, x, y, width, height, src_dc, src_x, src_y, rop)
    → Copy rectangular region from src to dest
    → SRCCOPY = straight copy
    → rop (raster operation) determines pixel combination

StretchBlt(dest_dc, x, y, width, height, src_dc, src_x, src_y, src_width, src_height, rop)
    → Copy with scaling
    → Stretches source to fit dest dimensions
    
BeginPaint() / EndPaint()
    → Get DC for WM_PAINT handler
    → Validates dirty region
    
GetDC() / ReleaseDC()
    → Get screen DC
    → Must release when done

DeleteObject(hbitmap/hdc)
    → Free GDI object
    → Critical for cleanup!
    
DeleteDC(hdc)
    → Destroy device context
```

### G. WinRT COMPOSITION - OVERLAY VISUAL RENDERING
```
Compositor::new()
    → Create composition engine (tied to DispatcherQueue)

Compositor.CreateContainerVisual()
    → Hierarchy node, no rendering, just contains children

Compositor.CreateSpriteVisual()
    → Drawable visual, renders brush to surface

Compositor.CreateNineGridBrush()
    → Divides source into 9 regions:
      [TL] [T] [TR]
      [L]  [C] [R]
      [BL] [B] [BR]
    → Can make center hollow (for border effect)
    → SetInsets(5px) → 5px borders

Compositor.CreateColorBrush(color)
    → Solid color brush (BGRA format)

Visual.Size(Vector2)
    → Set visual width/height in DIPs

Visual.Offset(Vector3)
    → Set visual position (x, y, z)

Visual.Brush(brush)
    → Set which brush renders

Visual.Opacity(float 0-1)
    → Alpha blending

Visual.RelativeSizeAdjustment(Vector2 x, y)
    → If (1.0, 1.0), visual is 100% of parent size
    → Automatically scales with parent

Visual.Children()
    → Get container for child visuals
    
Children.InsertAtBottom()
    → Add visual behind other children (first to render)

Children.InsertAtTop()
    → Add visual in front (last to render, draws over)

CreateWindowTarget(compositor)
    → Bind composition to HWND
    → Returns CompositionTarget

CompositionTarget.Root(visual)
    → Set scene root visual
```

### H. INPUT HANDLING
```
WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE
    → Message IDs for mouse events
    → lparam contains coordinates (GET_X_LPARAM, GET_Y_LPARAM macros)

WM_SETCURSOR
    → Message sent to set cursor shape
    → Return true to indicate handled

WM_KEYUP
    → Message for key release
    → wparam = virtual key code (VK_ESCAPE = 27)

GetCursorPos(&point)
    → Get current cursor position (screen coords)

SetCursor(hcursor)
    → Set cursor shape for window

LoadCursorW(nullptr, IDC_ARROW / IDC_CROSS)
    → Load system cursor
```

### I. EVENTS & THREADING
```
CreateEventW(nullptr, manual_reset, initial_state, name)
    → Create named event (synchronization primitive)
    → manual_reset = true: caller must ResetEvent
    → manual_reset = false: auto-resets after WaitForSingleObject

OpenEventW(access, inherit, name)
    → Open existing named event

SetEvent(hevent)
    → Set event to signaled state

ResetEvent(hevent)
    → Set event to unsignaled state

WaitForSingleObject(hevent, timeout_ms)
    → Wait for event to be signaled (blocking)
    → Returns WAIT_OBJECT_0 when signaled

MsgWaitForMultipleObjects(handles[], count, wait_all, timeout, input_flags)
    → Wait for multiple events OR window messages
    → Returns which event was signaled OR messages available
    → input_flags = QS_ALLINPUT = any message type
    → Used in event thread to multiplex hotkeys + exit signal

CloseHandle(hevent)
    → Close event handle
```

### J. DPI AWARENESS
```
SetProcessDpiAwarenessContext(context)
    → DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2
    → Per-monitor DPI scaling enabled
    → Critical for correct window positioning on multi-monitor setups
```

### K. THEME SUPPORT
```
SetImmersiveDarkMode(hwnd, bool)
    → Enable/disable dark theme for window
    → Updates title bar and system controls
```

---

## 4. RENDERING APPROACH & ARCHITECTURE

### Tech Stack Selection

| Component | Technology | Why |
|-----------|-----------|-----|
| **Overlay Selection UI** | WinRT Composition | GPU-accelerated, visual tree, smooth animations |
| **Thumbnail Display** | DWM Thumbnail API | Direct GPU access to source window, zero-copy |
| **Reparent Display** | Win32 window hierarchy | Native OS feature, no rendering needed |
| **Screenshot Display** | GDI BitBlt | Simple CPU-based rendering, works everywhere |
| **Main Window** | Win32 HWND | Standard Windows message-driven |
| **Async Coordination** | WinRT DispatcherQueue | Single-threaded UI, event-driven work scheduling |

### Visual Tree Architecture (Overlay)

```
Main Window HWND (invisible, HWND_TOPMOST)
│
├─ WinRT Compositor
│  └─ Composition Target (renders visual tree to HWND)
│     └─ Root Container Visual (100% screen size)
│         ├─ Shade Sprite Visual (semi-transparent overlay)
│         │  └─ NineGridBrush
│         │     ├─ Source: Black ColorBrush
│         │     ├─ LeftInset: window_rect.left
│         │     ├─ TopInset: window_rect.top
│         │     ├─ RightInset: screen_width - window_rect.right
│         │     ├─ BottomInset: screen_height - window_rect.bottom
│         │     └─ Center: hollow (60% opacity shows through)
│         │
│         └─ Window Area Container Visual (positioned at target window bounds)
│            └─ Selection Sprite Visual (red border)
│               └─ NineGridBrush
│                  ├─ Source: Red ColorBrush
│                  ├─ SetInsets(5px) → 5px borders
│                  └─ Center: hollow (transparent)
│
└─ Message Loop (Win32)
   ├─ WM_MOUSEMOVE → Update Selection Visual Offset/Size
   ├─ WM_LBUTTONDOWN → Start selection
   ├─ WM_LBUTTONUP → Finalize selection, invoke callback
   └─ WM_KEYUP (VK_ESCAPE) → Cancel selection
```

### Cropped Window Display

Each mode renders differently:

```
THUMBNAIL MODE:
  Main Window
  └─ DWM Thumbnail (rendered by GPU, updated by DwmUpdateThumbnailProperties)
     └─ Shows live stream from target window
     └─ No explicit Win32 rendering

REPARENT MODE:
  Main Window
  └─ ChildWindow
     └─ Target Window (reparented, clipped by child)
        └─ Native rendering (target's own rendering path)

SCREENSHOT MODE:
  Main Window
  ├─ WM_PAINT handler
  └─ StretchBlt from bitmap
     └─ Bitmap captured via PrintWindow + BitBlt
```

### Coordinate System Transformations

Three coordinate spaces to track:

```
┌─────────────────────────────────────┐
│ SCREEN SPACE                        │ Monitor pixels (0,0 at screen origin)
│ ┌──────────────────────────────────┐│
│ │ WINDOW FRAME SPACE               ││ Includes title bar, borders (DWM margins)
│ │  ┌────────────────────────────┐  ││
│ │  │ CLIENT AREA SPACE          │  ││ Content area (excludes frame)
│ │  │ (0,0 at client origin)     │  ││
│ │  └────────────────────────────┘  ││
│ └──────────────────────────────────┘│
└─────────────────────────────────────┘
```

**Conversion Example**:
```
Crop rect from overlay (in CLIENT space of target):  (50, 50, 150, 150)
  ↓
Get frame offset from DwmGetWindowAttribute:         frame_offset = (10, 10)
  ↓
Adjust to WINDOW FRAME space:                        (60, 60, 160, 160)
  ↓
For DwmUpdateThumbnailProperties(rcSource):          Use (60, 60, 160, 160)
                                                     (fSourceClientAreaOnly = false)
  ↓
For PrintWindow + crop:                              Adjust again by offset
```

---

## 5. SETTINGS & CONFIGURATION

### JSON Structure (PowerToysRun.json)

```json
{
  "CropAndLock": {
    "properties": {
      "reparent-hotkey": {
        "value": {
          "win": true,
          "ctrl": true,
          "shift": true,
          "alt": false,
          "code": 82  // VK_R
        }
      },
      "thumbnail-hotkey": {
        "value": {
          "win": true,
          "ctrl": true,
          "shift": true,
          "alt": false,
          "code": 84  // VK_T
        }
      },
      "screenshot-hotkey": {
        "value": {
          "win": true,
          "ctrl": true,
          "shift": true,
          "alt": false,
          "code": 83  // VK_S
        }
      }
    }
  }
}
```

### Hotkey Registration Flow

1. Module DLL loads settings file on startup
2. Parses JSON to extract three hotkey objects
3. Registers hotkeys with PowerToys runner
4. Runner calls `on_hotkey(id)` where id ∈ {0, 1, 2}
5. Module DLL sets appropriate event (Reparent/Thumbnail/Screenshot)
6. CropAndLock.exe event thread detects and processes

### Configurable Options

- **reparent-hotkey**: Modifiers + key code (default: Win+Ctrl+Shift+R)
- **thumbnail-hotkey**: Modifiers + key code (default: Win+Ctrl+Shift+T)
- **screenshot-hotkey**: Modifiers + key code (default: Win+Ctrl+Shift+S)

All three must be distinct key codes (currently R=82, T=84, S=83).

---

## 6. TEST COVERAGE REQUIREMENTS

### Phase 1: UNIT TESTS (Pure Logic - 100% coverage)

**File: `crates/cropandlock-core/tests/geometry_tests.rs`**

```rust
#[test]
fn test_crop_rect_normalization_bottom_right() {
    // User drags from (100,100) to (200,200) → normalized same
    let rect = normalize_crop_rect(100, 100, 200, 200);
    assert_eq!(rect, (100, 100, 200, 200));
}

#[test]
fn test_crop_rect_normalization_inverted() {
    // User drags from (200,200) to (100,100) → normalized to (100,100,200,200)
    let rect = normalize_crop_rect(200, 200, 100, 100);
    assert_eq!(rect, (100, 100, 200, 200));
}

#[test]
fn test_scale_factor_equal_aspect() {
    // Window 800x600 (4:3), content 400x300 (4:3)
    // Should scale 2.0x
    let scale = compute_scale_factor(800, 600, 400, 300);
    assert_eq!(scale, 2.0);
}

#[test]
fn test_scale_factor_wider_window() {
    // Window 1000x600, content 400x300
    // Window ratio 5:3, content ratio 4:3
    // Window is wider, limit by height: 600/300 = 2.0
    let scale = compute_scale_factor(1000, 600, 400, 300);
    assert_eq!(scale, 2.0);
}

#[test]
fn test_dest_rect_centering() {
    // 800x600 window, 200x100 content (scaled 2x → 400x200)
    // Centered: (200, 200, 600, 400)
    let dest = compute_dest_rect(800, 600, 200, 100);
    assert_eq!(dest, (200, 200, 600, 400));
}

#[test]
fn test_frame_to_client_coordinate_transform() {
    // Frame offset (10, 10), crop frame (50,50,150,150)
    // Client coords: (40,40,140,140)
    let client = transform_frame_to_client(50, 50, 150, 150, 10, 10);
    assert_eq!(client, (40, 40, 140, 140));
}

#[test]
fn test_empty_rect_rejected() {
    assert!(!is_valid_crop_rect(50, 50, 50, 100));  // zero width
    assert!(!is_valid_crop_rect(50, 50, 150, 50));  // zero height
    assert!(is_valid_crop_rect(50, 50, 150, 150));  // valid
}

#[test]
fn test_point_clamp_to_bounds() {
    let bounds = (0, 0, 400, 300);
    assert_eq!(clamp_point(500, 400, bounds), (400, 300));
    assert_eq!(clamp_point(-50, -50, bounds), (0, 0));
    assert_eq!(clamp_point(200, 150, bounds), (200, 150));
}

#[test]
fn test_all_displays_union_single() {
    let displays = vec![(0, 0, 1920, 1080)];
    let union = compute_displays_union(&displays);
    assert_eq!(union, (0, 0, 1920, 1080));
}

#[test]
fn test_all_displays_union_multiple() {
    // Two 1920x1080 monitors side-by-side
    let displays = vec![(0, 0, 1920, 1080), (1920, 0, 3840, 1080)];
    let union = compute_displays_union(&displays);
    assert_eq!(union, (0, 0, 3840, 1080));
}

#[test]
fn test_all_displays_union_stacked() {
    // Two monitors stacked vertically
    let displays = vec![(0, 0, 1920, 1080), (0, 1080, 1920, 2160)];
    let union = compute_displays_union(&displays);
    assert_eq!(union, (0, 0, 1920, 2160));
}
```

**File: `crates/cropandlock-core/tests/hotkey_tests.rs`**

```rust
#[test]
fn test_parse_reparent_hotkey_json() {
    let json = r#"{
        "win": true, "ctrl": true, "shift": true, "alt": false, "code": 82
    }"#;
    let hotkey = parse_hotkey_json(json).unwrap();
    assert_eq!(hotkey.win, true);
    assert_eq!(hotkey.ctrl, true);
    assert_eq!(hotkey.shift, true);
    assert_eq!(hotkey.key_code, 82);
}

#[test]
fn test_three_modes_distinct_hotkeys() {
    let modes = vec![
        CropAndLockMode::Reparent,
        CropAndLockMode::Thumbnail,
        CropAndLockMode::Screenshot,
    ];
    let key_codes: Vec<_> = modes.iter().map(|m| default_hotkey(m).key_code).collect();
    assert_ne!(key_codes[0], key_codes[1]);
    assert_ne!(key_codes[1], key_codes[2]);
}
```

### Phase 2: INTEGRATION TESTS (Win32 APIs)

**File: `crates/cropandlock-exe/tests/window_registration.rs`** `[ignore]`

```rust
#[test]
#[ignore]  // Requires UI, manual test
fn test_overlay_window_registers_and_creates() {
    // 1. Call RegisterWindowClass for OverlayWindow
    // 2. Create window via CreateWindowExW
    // 3. Verify window handle is valid
    // 4. Verify window style is WS_POPUP | WS_EX_TOPMOST
    // 5. Destroy window
}

#[test]
#[ignore]
fn test_thumbnail_mode_dwm_registration() {
    // 1. Get a real window (e.g., Notepad)
    // 2. Create ThumbnailCropAndLockWindow
    // 3. Call DwmRegisterThumbnail
    // 4. Verify thumbnail handle is non-null
    // 5. Call DwmUpdateThumbnailProperties
    // 6. Verify WM_SIZE/WM_SIZING updates
    // 7. Verify cleanup (DwmUnregisterThumbnail)
}

#[test]
#[ignore]
fn test_reparent_mode_window_state_save_restore() {
    // 1. Get target window (Notepad)
    // 2. Capture original state (style, rect, placement)
    // 3. Create ReparentCropAndLockWindow
    // 4. Call CropAndLock() to reparent
    // 5. Verify target is now child of our window
    // 6. Destroy our window
    // 7. Verify target is back to original state
    // 8. Verify window rect matches original
    // 9. Verify window style matches original
}

#[test]
#[ignore]
fn test_screenshot_mode_bitmap_capture() {
    // 1. Get target window (colorful window, e.g., Edge with webpage)
    // 2. Create ScreenshotCropAndLockWindow
    // 3. Call CropAndLock() to capture
    // 4. Verify bitmap is non-null
    // 5. Verify bitmap dimensions match crop rect
    // 6. Display window and verify screenshot renders
    // 7. Manually verify colors match original window
}
```

### Phase 3: E2E TESTS (Full Flow)

**File: `crates/cropandlock-exe/tests/e2e_tests.rs`** `[ignore]`

```rust
#[test]
#[ignore]  // Requires interactive UI testing
fn test_thumbnail_mode_end_to_end() {
    // 1. Open target window (Notepad with text)
    // 2. Simulate Win+Ctrl+Shift+T hotkey
    // 3. Verify overlay appears
    // 4. Simulate user drag selection
    // 5. Verify ThumbnailCropAndLockWindow created
    // 6. Verify thumbnail shows selected region
    // 7. Simulate target window move
    // 8. Verify thumbnail doesn't follow (source doesn't move)
    // 9. Close cropped window
}

#[test]
#[ignore]
fn test_reparent_mode_end_to_end() {
    // 1. Open target window
    // 2. Simulate Win+Ctrl+Shift+R hotkey
    // 3. Verify overlay appears
    // 4. Simulate user drag selection
    // 5. Verify ReparentCropAndLockWindow created
    // 6. Verify target is reparented and cropped
    // 7. Simulate dragging cropped window
    // 8. Verify target moves with it
    // 9. Close cropped window
    // 10. Verify target restored and visible on desktop again
}

#[test]
#[ignore]
fn test_screenshot_mode_end_to_end() {
    // 1. Open target window
    // 2. Simulate Win+Ctrl+Shift+S hotkey
    // 3. Verify overlay appears
    // 4. Simulate user drag selection
    // 5. Verify ScreenshotCropAndLockWindow created
    // 6. Verify bitmap screenshot displays
    // 7. Simulate target window move
    // 8. Verify screenshot doesn't change (static)
    // 9. Close target window
    // 10. Verify screenshot still displayed (persistence)
}

#[test]
#[ignore]
fn test_multi_crop_windows() {
    // 1. Create 3 cropped windows (different modes)
    // 2. Verify all 3 displayed simultaneously
    // 3. Close one, verify others persist
    // 4. Close all
}
```

---

## 7. FEASIBILITY & EFFORT ESTIMATION

### Porting Difficulty: **MODERATE-LOW**

| Component | Difficulty | Reason |
|-----------|------------|--------|
| **Coordinate Math** | ✅ EASY | Pure Rust, no dependencies |
| **Hotkey Settings** | ✅ EASY | JSON parsing (serde) |
| **Overlay UI (Composition)** | ✅ MODERATE | WinRT via `windows` crate, fully documented |
| **DWM Thumbnail** | ✅ MODERATE | APIs in `windows-sys`, straightforward C binding |
| **Window Reparenting** | ✅ MODERATE | SetParent, GetWindowLongPtr available, standard Win32 |
| **Screenshot Capture** | ✅ MODERATE | PrintWindow, BitBlt available, GDI cleanup via RAII |
| **Message Loop & Events** | ✅ MODERATE | Standard Win32 pattern, event multiplexing easy |
| **Module Interface** | 🔄 DONE | Already ported (C++ COM) |

### Effort Breakdown

| Phase | Component | Estimated Time |
|-------|-----------|-----------------|
| **Phase 1** | Core Library (geometry, settings) | **1-2 weeks** |
| **Phase 2** | Module Interface | ✅ **Done** (0 weeks) |
| **Phase 3a** | Overlay Window (Composition, selection) | **2-3 weeks** |
| **Phase 3b** | Thumbnail Mode (DWM) | **1-2 weeks** |
| **Phase 3c** | Reparent Mode (SetParent, state save/restore) | **1-2 weeks** |
| **Phase 3d** | Screenshot Mode (PrintWindow, BitBlt) | **1-2 weeks** |
| **Phase 3e** | Event Handling & Threading | **1 week** |
| **Phase 3f** | Main Loop & Integration | **1 week** |
| **Testing** | Unit, integration, E2E | **1-2 weeks** |
| **Polish & Debugging** | Error handling, edge cases | **1 week** |
| | **TOTAL** | **10-14 weeks** |

### No Blocking Issues

- ✅ All Win32 APIs available in `windows-sys`
- ✅ All WinRT Composition APIs available in `windows` crate
- ✅ No COM Automation (IDispatch) required
- ✅ No legacy Win32 Composition Engine needed
- ✅ All coordinate math straightforward

---

## 8. RISK MITIGATION

| Risk | Impact | Likelihood | Mitigation |
|------|--------|-----------|-----------|
| **DPI Scaling Edge Cases** | Crop rect misalignment on multi-monitor | MEDIUM | Extensive coordinate math tests, manual testing on 125%, 150% DPI |
| **Window State Corruption on Reparent** | Target window left in broken state | LOW | Always restore state before close, add safety checks |
| **Thumbnail Not Rendering (DWM Version)** | Old Windows 7, no DWM | VERY LOW | Add fallback to screenshot mode, version check |
| **GDI Resource Leak** | Memory leak, handles not freed | MEDIUM | Use `wil` crate RAII helpers for DC/bitmap, verify Valgrind clean |
| **DispatcherQueue TryEnqueue Failure** | Hotkey ignored | VERY LOW | Add retry logic, log failures |
| **Window Deleted While Reparented** | Access violation | LOW | Check IsWindow() before accessing, handle gracefully |
| **Multi-Monitor Negative Coordinates** | Clipping issues | MEDIUM | Test on multi-monitor setups, validate RECT math |
| **Composition Visual Tree Corruption** | Overlay not rendering | LOW | Validate all Visual creation calls, add error logging |

---

## 9. WORKSPACE STRUCTURE

```
PowerToysRust/src/rust/
├─ Cargo.toml                           (workspace root)
├─ crates/
│  ├─ cropandlock-core/                 (Phase 1: Pure logic)
│  │  ├─ Cargo.toml
│  │  ├─ src/
│  │  │  ├─ lib.rs
│  │  │  ├─ geometry/
│  │  │  │  ├─ mod.rs
│  │  │  │  ├─ scaling.rs               (scale factor, dest rect)
│  │  │  │  └─ coordinate.rs            (frame ↔ client transforms)
│  │  │  ├─ crop/
│  │  │  │  ├─ mod.rs
│  │  │  │  └─ rect.rs                  (normalize, validate, clamp)
│  │  │  ├─ display/
│  │  │  │  ├─ mod.rs
│  │  │  │  └─ monitors.rs              (union, DisplayInfo)
│  │  │  ├─ settings/
│  │  │  │  ├─ mod.rs
│  │  │  │  └─ hotkeys.rs               (parse JSON, default hotkeys)
│  │  │  ├─ types.rs                    (CropAndLockMode, Rect, Point)
│  │  │  └─ lib.rs
│  │  └─ tests/
│  │     ├─ geometry_tests.rs
│  │     ├─ scaling_tests.rs
│  │     ├─ coordinate_tests.rs
│  │     ├─ crop_tests.rs
│  │     ├─ display_tests.rs
│  │     └─ hotkey_tests.rs
│  │
│  └─ cropandlock-exe/                  (Phase 3: EXE with Win32 + WinRT)
│     ├─ Cargo.toml
│     ├─ src/
│     │  ├─ main.rs                     (entry point, message loop)
│     │  ├─ window/
│     │  │  ├─ mod.rs
│     │  │  ├─ registration.rs          (RegisterWindowClass for each type)
│     │  │  ├─ message_loop.rs          (Win32 GetMessage pump)
│     │  │  └─ manager.rs               (WindowManager, lifecycle)
│     │  ├─ overlay/
│     │  │  ├─ mod.rs                   (OverlayWindow struct)
│     │  │  ├─ composition.rs           (build visual tree, nine-grid)
│     │  │  ├─ input.rs                 (WM_LBUTTONDOWN, etc.)
│     │  │  └─ rendering.rs             (UpdateWindow, composition)
│     │  ├─ cropped_window/
│     │  │  ├─ mod.rs                   (CropAndLockWindow trait)
│     │  │  ├─ thumbnail.rs             (DwmRegisterThumbnail flow)
│     │  │  ├─ reparent.rs              (SetParent, save/restore)
│     │  │  ├─ screenshot.rs            (PrintWindow + BitBlt)
│     │  │  └─ child_window.rs          (ChildWindow for reparent)
│     │  ├─ event/
│     │  │  ├─ mod.rs
│     │  │  └─ handler.rs               (MsgWaitForMultipleObjects thread)
│     │  ├─ theme/
│     │  │  └─ mod.rs                   (SetImmersiveDarkMode)
│     │  ├─ util/
│     │  │  ├─ mod.rs
│     │  │  ├─ dpi.rs                   (SetProcessDpiAwarenessContext)
│     │  │  ├─ window_rect.rs           (ClientAreaInScreenSpace)
│     │  │  ├─ display.rs               (ComputeAllDisplaysUnion)
│     │  │  └─ error.rs                 (Win32 error handling)
│     │  └─ lib.rs
│     └─ tests/
│        ├─ integration_tests.rs        ([ignore]d E2E tests)
│        └─ window_registration.rs
```

---

## 10. IMPLEMENTATION CHECKLIST

### Phase 1: Core Library ✅

- [ ] Create `cropandlock-core` crate
- [ ] Implement `types.rs` (Rect, Point, CropAndLockMode)
- [ ] Implement `geometry/mod.rs` + `scaling.rs`
- [ ] Implement `geometry/coordinate.rs`
- [ ] Implement `crop/rect.rs`
- [ ] Implement `display/monitors.rs`
- [ ] Implement `settings/hotkeys.rs` (JSON parsing)
- [ ] Write all unit tests (95%+ coverage)
- [ ] Verify tests pass locally

### Phase 3a: Overlay Window ✅

- [ ] Create `overlay/mod.rs` (OverlayWindow struct)
- [ ] Implement `overlay/composition.rs` (visual tree building)
- [ ] Register OverlayWindow class
- [ ] Create overlay window (full-screen, topmost)
- [ ] Implement `overlay/input.rs` (mouse handlers)
- [ ] Test selection rectangle rendering
- [ ] Test cursor changes (crosshair over window)
- [ ] Test ESC to cancel

### Phase 3b: Thumbnail Mode ✅

- [ ] Create `cropped_window/thumbnail.rs`
- [ ] Implement DwmRegisterThumbnail
- [ ] Implement DwmUpdateThumbnailProperties
- [ ] Handle WM_SIZE to recompute scale/position
- [ ] Implement DwmUnregisterThumbnail cleanup
- [ ] Test with real window
- [ ] Test on multi-monitor setup
- [ ] Verify live updates work

### Phase 3c: Reparent Mode ✅

- [ ] Create `cropped_window/reparent.rs`
- [ ] Create `cropped_window/child_window.rs`
- [ ] Implement SaveOriginalState()
- [ ] Implement SetParent() reparent
- [ ] Implement RestoreOriginalState()
- [ ] Handle WM_MOUSEACTIVATE to restore focus to target
- [ ] Test window reparent + restore
- [ ] Test with maximized windows
- [ ] Test multi-monitor DPI contexts

### Phase 3d: Screenshot Mode ✅

- [ ] Create `cropped_window/screenshot.rs`
- [ ] Implement PrintWindow() capture
- [ ] Implement BitBlt() crop
- [ ] Implement WM_PAINT rendering
- [ ] Test bitmap capture
- [ ] Test StretchBlt aspect ratio preservation
- [ ] Test cleanup (bitmap deletion)

### Phase 3e-f: Main Loop & Events ✅

- [ ] Implement event thread (MsgWaitForMultipleObjects)
- [ ] Implement DispatcherQueue scheduling
- [ ] Implement main message pump
- [ ] Implement startup (COM, DPI, Compositor)
- [ ] Test hotkey events trigger overlay
- [ ] Test all three modes activate
- [ ] Test window lifecycle (create, close, cleanup)

### Testing ✅

- [ ] All unit tests pass
- [ ] No memory leaks (Valgrind or similar)
- [ ] Manual E2E test each mode
- [ ] Multi-monitor stress test
- [ ] High DPI test (125%, 150%)
- [ ] Dark theme test
- [ ] Close target window while cropped (graceful degradation)

---

## 11. VERIFICATION CHECKLIST (Final)

Before declaring production-ready:

- ✅ **Geometry Math**: All coordinate transforms tested and verified
- ✅ **Overlay Rendering**: Red border appears, moves with mouse, crosshair works
- ✅ **Thumbnail Mode**: Live preview shows, updates on target resize, DWM works
- ✅ **Reparent Mode**: Target reparents successfully, fully restores on close
- ✅ **Screenshot Mode**: Screenshot captures correctly, StretchBlt renders aspect-ratio-aware
- ✅ **Hotkey Integration**: All 3 hotkeys trigger appropriate mode
- ✅ **Multi-Monitor**: Overlay spans all displays, coordinates correct on misaligned monitors
- ✅ **DPI Handling**: Per-monitor DPI awareness enabled, scaling correct at 100%/125%/150%
- ✅ **Theme Support**: Dark/light theme switching updates window
- ✅ **Window Lifecycle**: Windows destroyed cleanly, no lingering handles
- ✅ **Error Handling**: All Win32 errors caught, logged, no crashes
- ✅ **1:1 UX Parity**: All three modes behave identically to C++ original
- ✅ **Performance**: Overlay smooth, no jank, < 50ms selection response
- ✅ **Resource Cleanup**: No memory leaks, DC/bitmap/thumbnail handles released

---

## SUMMARY

**CropAndLock Rust Port: MODERATE Difficulty, HIGH Confidence**

**Port Feasibility**:
- ✅ Pure geometry logic → 100% Rust, easily tested
- ✅ Win32 APIs → all in `windows-sys`, well-documented
- ✅ WinRT Composition → fully available in `windows` crate
- ✅ DWM Thumbnail → bindings exist, straightforward
- ✅ No COM complexity → simple FFI layer already done

**Risk Level**: LOW

**Estimated Timeline**: 10-14 weeks for production-quality port

**Next Steps**:
1. Create workspace structure
2. Implement Phase 1 (core library) with full test coverage
3. Implement Phase 3a (overlay UI)
4. Implement all three display modes in parallel
5. Integration testing
6. E2E verification