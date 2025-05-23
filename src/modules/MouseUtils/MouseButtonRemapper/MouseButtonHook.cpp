#include "pch.h"
#include "MouseButtonHook.h"
#include <common/debug_control.h>

// Initialize static members
HHOOK MouseButtonHook::s_mouseHook = nullptr;
MouseButtonHook::MouseButtonCallback MouseButtonHook::s_mouseButtonCallback = nullptr;

MouseButtonHook::MouseButtonHook()
{
}

MouseButtonHook::~MouseButtonHook()
{
    disable();
}

void MouseButtonHook::setMouseButtonCallback(MouseButtonCallback callback)
{
    s_mouseButtonCallback = callback;
}

void MouseButtonHook::enable()
{
#if defined(DISABLE_LOWLEVEL_HOOKS_WHEN_DEBUGGED)
    if (IsDebuggerPresent())
    {
        return;
    }
#endif
    if (!s_mouseHook)
    {
        s_mouseHook = SetWindowsHookEx(WH_MOUSE_LL, mouseHookProc, GetModuleHandle(NULL), 0);
    }
}

void MouseButtonHook::disable()
{
    if (s_mouseHook)
    {
        UnhookWindowsHookEx(s_mouseHook);
        s_mouseHook = nullptr;
    }
}

LRESULT CALLBACK MouseButtonHook::mouseHookProc(int nCode, WPARAM wParam, LPARAM lParam)
{
    if (nCode == HC_ACTION)
    {
        MSLLHOOKSTRUCT* mouseStruct = reinterpret_cast<MSLLHOOKSTRUCT*>(lParam);
        
        // Check if we're handling mouse button events
        switch (wParam)
        {
            case WM_LBUTTONDOWN:
            case WM_LBUTTONUP:
            case WM_RBUTTONDOWN:
            case WM_RBUTTONUP:
            case WM_MBUTTONDOWN:
            case WM_MBUTTONUP:
            case WM_XBUTTONDOWN:
            case WM_XBUTTONUP:
            {
                if (s_mouseButtonCallback)
                {
                    // Call the registered callback
                    bool handled = s_mouseButtonCallback(wParam, mouseStruct->pt, mouseStruct->mouseData);
                    if (handled)
                    {
                        // Return non-zero to prevent the system from passing the message to other hooks or the target window
                        return 1;
                    }
                }
                break;
            }
        }
    }
    
    // Call the next hook in the chain
    return CallNextHookEx(s_mouseHook, nCode, wParam, lParam);
}