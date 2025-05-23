#pragma once

#include <functional>

class MouseButtonHook
{
public:
    // Callback signature for mouse button events
    // wParam: the mouse message (WM_LBUTTONDOWN, etc.)
    // point: the mouse coordinates
    // mouseData: additional mouse data (wheel delta, xbutton)
    using MouseButtonCallback = std::function<bool(WPARAM wParam, POINT point, DWORD mouseData)>;

    MouseButtonHook();
    ~MouseButtonHook();

    // Enable or disable the hook
    void enable();
    void disable();

    // Set the callback for mouse button events
    void setMouseButtonCallback(MouseButtonCallback callback);

private:
    static HHOOK s_mouseHook;
    static MouseButtonCallback s_mouseButtonCallback;
    
    // Static mouse hook procedure
    static LRESULT CALLBACK mouseHookProc(int nCode, WPARAM wParam, LPARAM lParam);
};