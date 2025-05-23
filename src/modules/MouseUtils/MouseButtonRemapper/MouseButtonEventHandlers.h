#pragma once

#include "MouseButtonMappingConfiguration.h"

namespace MouseButtonEventHandlers
{
    // Convert mouse button message (WM_LBUTTONDOWN) to mouse button code (VK_LBUTTON)
    DWORD MessageToMouseButton(WPARAM wParam, DWORD mouseData);
    
    // Convert mouse button code (VK_LBUTTON) to up/down messages (WM_LBUTTONDOWN/WM_LBUTTONUP)
    std::pair<WPARAM, WPARAM> MouseButtonToMessages(DWORD buttonCode);
    
    // Handle a mouse button event and perform remapping if needed
    bool HandleMouseButtonRemapEvent(
        WPARAM wParam, 
        POINT point, 
        DWORD mouseData, 
        MouseButtonMappingConfiguration& mappingConfig);
    
    // Send a simulated mouse button event
    bool SendMouseButtonInput(DWORD buttonCode, bool isDown, POINT point);
    
    // Send a simulated keyboard input (for when a mouse button is remapped to a key)
    bool SendKeyboardInput(DWORD virtualKey, bool isDown);
    
    // Send a keyboard input with modifiers
    bool SendKeyboardInputWithModifiers(DWORD virtualKey, const std::vector<DWORD>& modifiers, bool isDown);
}