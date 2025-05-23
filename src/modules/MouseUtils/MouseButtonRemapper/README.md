# Mouse Button Remapper

## Overview

The Mouse Button Remapper module allows users to remap mouse buttons to other mouse buttons, keyboard keys, or keyboard shortcuts with modifiers.

## Features

- Remap the five standard mouse buttons (left, right, middle, forward, backward)
- Remap a mouse button to another mouse button
- Remap a mouse button to a keyboard key
- Remap a mouse button to a keyboard key with modifiers (Ctrl, Alt, Shift, Win)
- Disable mouse buttons

## Settings

The module stores its configuration in JSON format. The settings include:

- Enable/disable the module
- Mappings from original mouse buttons to target actions

## Implementation Details

The module uses a low-level mouse hook (WH_MOUSE_LL) to intercept mouse button events and apply remappings.

### Mouse Buttons

| Button Name | Virtual Key Code | Windows Message |
|-------------|-----------------|-----------------|
| Left Button | VK_LBUTTON (0x01) | WM_LBUTTONDOWN/UP |
| Right Button | VK_RBUTTON (0x02) | WM_RBUTTONDOWN/UP |
| Middle Button | VK_MBUTTON (0x04) | WM_MBUTTONDOWN/UP |
| X1 Button (Forward) | VK_XBUTTON1 (0x05) | WM_XBUTTONDOWN/UP |
| X2 Button (Back) | VK_XBUTTON2 (0x06) | WM_XBUTTONDOWN/UP |

### Remapping Types

1. Button to Button: Maps one mouse button to another mouse button
2. Button to Key: Maps a mouse button to a keyboard key
3. Button to Key+Modifiers: Maps a mouse button to a keyboard key with modifier keys
4. Disable Button: Prevents a mouse button from triggering any action