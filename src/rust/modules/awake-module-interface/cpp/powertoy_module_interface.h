#pragma once
// Minimal reproduction of PowertoyModuleIface for the Rust adapter.
// This matches the real header at src/modules/interface/powertoy_module_interface.h
// in the microsoft/PowerToys repository.

#include <cstddef>
#include <cstdint>

// Minimal GPO enum matching powertoys_gpo::gpo_rule_configured_t
namespace powertoys_gpo {
    enum gpo_rule_configured_t {
        gpo_rule_configured_wrong_value = -3,
        gpo_rule_configured_unavailable = -2,
        gpo_rule_configured_not_configured = -1,
        gpo_rule_configured_disabled = 0,
        gpo_rule_configured_enabled = 1,
    };
}

class PowertoyModuleIface {
public:
    struct Hotkey {
        bool win = false;
        bool ctrl = false;
        bool shift = false;
        bool alt = false;
        unsigned char key = 0;
        int id = 0;
        bool isShown = true;
    };

    struct HotkeyEx {
        unsigned short modifiersMask = 0;
        unsigned short vkCode = 0;
        int id = 0;
    };

    virtual const wchar_t* get_name() = 0;
    virtual const wchar_t* get_key() = 0;
    virtual bool get_config(wchar_t* buffer, int* buffer_size) = 0;
    virtual void set_config(const wchar_t* config) = 0;
    virtual void call_custom_action(const wchar_t*) {}
    virtual void enable() = 0;
    virtual void disable() = 0;
    virtual bool is_enabled() = 0;
    virtual void destroy() = 0;
    virtual size_t get_hotkeys(Hotkey*, size_t) { return 0; }

    // These two MUST be present to keep vtable layout correct
    virtual int GetHotkeyEx() { return 0; }  // returns std::optional<HotkeyEx> in real code
    virtual void OnHotkeyEx() {}

    virtual bool on_hotkey(size_t) { return false; }
    virtual bool keep_track_of_pressed_win_key() { return false; }
    virtual unsigned int milliseconds_win_key_must_be_pressed() { return 0; }
    virtual void send_settings_telemetry() {}
    virtual bool is_enabled_by_default() const { return true; }

    virtual powertoys_gpo::gpo_rule_configured_t gpo_policy_enabled_configuration() {
        return powertoys_gpo::gpo_rule_configured_not_configured;
    }
};

typedef PowertoyModuleIface*(__cdecl* powertoy_create_func)();
