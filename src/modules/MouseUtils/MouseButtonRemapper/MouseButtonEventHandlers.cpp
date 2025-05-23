#include "pch.h"
#include "MouseButtonEventHandlers.h"
#include <common/utils/logger/logger.h>

namespace MouseButtonEventHandlers
{
    DWORD MessageToMouseButton(WPARAM wParam, DWORD mouseData)
    {
        switch (wParam)
        {
        case WM_LBUTTONDOWN:
        case WM_LBUTTONUP:
            return MB_LEFT_BUTTON;
        case WM_RBUTTONDOWN:
        case WM_RBUTTONUP:
            return MB_RIGHT_BUTTON;
        case WM_MBUTTONDOWN:
        case WM_MBUTTONUP:
            return MB_MIDDLE_BUTTON;
        case WM_XBUTTONDOWN:
        case WM_XBUTTONUP:
            // HIWORD(mouseData) contains the button that was pressed
            return (HIWORD(mouseData) == XBUTTON1) ? MB_X1_BUTTON : MB_X2_BUTTON;
        default:
            return 0;
        }
    }

    std::pair<WPARAM, WPARAM> MouseButtonToMessages(DWORD buttonCode)
    {
        switch (buttonCode)
        {
        case MB_LEFT_BUTTON:
            return { WM_LBUTTONDOWN, WM_LBUTTONUP };
        case MB_RIGHT_BUTTON:
            return { WM_RBUTTONDOWN, WM_RBUTTONUP };
        case MB_MIDDLE_BUTTON:
            return { WM_MBUTTONDOWN, WM_MBUTTONUP };
        case MB_X1_BUTTON:
            return { WM_XBUTTONDOWN, WM_XBUTTONUP };
        case MB_X2_BUTTON:
            return { WM_XBUTTONDOWN, WM_XBUTTONUP };
        default:
            return { 0, 0 };
        }
    }

    bool HandleMouseButtonRemapEvent(
        WPARAM wParam, 
        POINT point, 
        DWORD mouseData, 
        MouseButtonMappingConfiguration& mappingConfig)
    {
        // Determine if this is a button down or up message
        bool isButtonDown = (wParam == WM_LBUTTONDOWN || wParam == WM_RBUTTONDOWN || 
                             wParam == WM_MBUTTONDOWN || wParam == WM_XBUTTONDOWN);
        
        // Convert the message to a mouse button code
        DWORD buttonCode = MessageToMouseButton(wParam, mouseData);
        if (buttonCode == 0)
        {
            return false; // Not a mouse button we handle
        }
        
        // Check if there's a remapping for this button
        auto remapping = mappingConfig.GetMouseButtonRemap(buttonCode);
        if (!remapping)
        {
            return false; // No remapping for this button
        }
        
        // Handle the remapping based on target type
        if (std::holds_alternative<DWORD>(*remapping))
        {
            DWORD targetButton = std::get<DWORD>(*remapping);
            return SendMouseButtonInput(targetButton, isButtonDown, point);
        }
        else if (std::holds_alternative<std::pair<DWORD, std::vector<DWORD>>>(*remapping))
        {
            auto [targetKey, modifiers] = std::get<std::pair<DWORD, std::vector<DWORD>>>(*remapping);
            return SendKeyboardInputWithModifiers(targetKey, modifiers, isButtonDown);
        }
        else if (std::holds_alternative<std::wstring>(*remapping))
        {
            std::wstring targetValue = std::get<std::wstring>(*remapping);
            if (targetValue == L"disabled")
            {
                // Button is disabled, consume the input
                return true;
            }
        }
        
        return false;
    }

    bool SendMouseButtonInput(DWORD buttonCode, bool isDown, POINT point)
    {
        INPUT input = {};
        input.type = INPUT_MOUSE;
        
        // Set the mouse position
        input.mi.dx = point.x;
        input.mi.dy = point.y;
        
        // Set the appropriate flags based on the button
        switch (buttonCode)
        {
        case MB_LEFT_BUTTON:
            input.mi.dwFlags = isDown ? MOUSEEVENTF_LEFTDOWN : MOUSEEVENTF_LEFTUP;
            break;
        case MB_RIGHT_BUTTON:
            input.mi.dwFlags = isDown ? MOUSEEVENTF_RIGHTDOWN : MOUSEEVENTF_RIGHTUP;
            break;
        case MB_MIDDLE_BUTTON:
            input.mi.dwFlags = isDown ? MOUSEEVENTF_MIDDLEDOWN : MOUSEEVENTF_MIDDLEUP;
            break;
        case MB_X1_BUTTON:
            input.mi.dwFlags = isDown ? MOUSEEVENTF_XDOWN : MOUSEEVENTF_XUP;
            input.mi.mouseData = XBUTTON1;
            break;
        case MB_X2_BUTTON:
            input.mi.dwFlags = isDown ? MOUSEEVENTF_XDOWN : MOUSEEVENTF_XUP;
            input.mi.mouseData = XBUTTON2;
            break;
        default:
            return false;
        }
        
        // Send the input
        UINT result = SendInput(1, &input, sizeof(INPUT));
        if (result != 1)
        {
            Logger::error(L"Failed to send mouse input. Error code: {}", GetLastError());
            return false;
        }
        
        return true;
    }

    bool SendKeyboardInput(DWORD virtualKey, bool isDown)
    {
        INPUT input = {};
        input.type = INPUT_KEYBOARD;
        input.ki.wVk = static_cast<WORD>(virtualKey);
        input.ki.dwFlags = isDown ? 0 : KEYEVENTF_KEYUP;
        
        // Send the input
        UINT result = SendInput(1, &input, sizeof(INPUT));
        if (result != 1)
        {
            Logger::error(L"Failed to send keyboard input. Error code: {}", GetLastError());
            return false;
        }
        
        return true;
    }

    bool SendKeyboardInputWithModifiers(DWORD virtualKey, const std::vector<DWORD>& modifiers, bool isDown)
    {
        // If pressing down, first press the modifiers then the key
        // If releasing, first release the key then the modifiers
        std::vector<INPUT> inputs;
        
        if (isDown)
        {
            // Press modifiers first
            for (DWORD mod : modifiers)
            {
                INPUT input = {};
                input.type = INPUT_KEYBOARD;
                input.ki.wVk = static_cast<WORD>(mod);
                inputs.push_back(input);
            }
            
            // Then press the key
            INPUT keyInput = {};
            keyInput.type = INPUT_KEYBOARD;
            keyInput.ki.wVk = static_cast<WORD>(virtualKey);
            inputs.push_back(keyInput);
        }
        else
        {
            // Release the key first
            INPUT keyInput = {};
            keyInput.type = INPUT_KEYBOARD;
            keyInput.ki.wVk = static_cast<WORD>(virtualKey);
            keyInput.ki.dwFlags = KEYEVENTF_KEYUP;
            inputs.push_back(keyInput);
            
            // Then release modifiers in reverse order
            for (auto it = modifiers.rbegin(); it != modifiers.rend(); ++it)
            {
                INPUT input = {};
                input.type = INPUT_KEYBOARD;
                input.ki.wVk = static_cast<WORD>(*it);
                input.ki.dwFlags = KEYEVENTF_KEYUP;
                inputs.push_back(input);
            }
        }
        
        // Send all inputs
        UINT result = SendInput(static_cast<UINT>(inputs.size()), inputs.data(), sizeof(INPUT));
        if (result != inputs.size())
        {
            Logger::error(L"Failed to send keyboard inputs with modifiers. Error code: {}", GetLastError());
            return false;
        }
        
        return true;
    }
}