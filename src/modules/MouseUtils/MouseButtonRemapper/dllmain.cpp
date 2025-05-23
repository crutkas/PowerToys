#include "pch.h"
#include <interface/powertoy_module_interface.h>
#include <common/SettingsAPI/settings_objects.h>
#include <common/utils/logger/logger.h>
#include "trace.h"
#include "MouseButtonHook.h"
#include "MouseButtonMappingConfiguration.h"
#include "MouseButtonEventHandlers.h"

extern "C" IMAGE_DOS_HEADER __ImageBase;

HMODULE m_hModule;
HHOOK g_hook = nullptr;

// The PowerToy name that will be shown in the settings.
const static wchar_t* MODULE_NAME = L"MouseButtonRemapper";
// Add a description that will we shown in the module settings page.
const static wchar_t* MODULE_DESC = L"Remap mouse buttons to keyboard keys or other mouse buttons";

namespace
{
    const wchar_t JSON_KEY_PROPERTIES[] = L"properties";
    const wchar_t JSON_KEY_VALUE[] = L"value";
    const wchar_t JSON_KEY_ENABLED[] = L"enabled";
}

// Implement the PowerToy Module Interface and all the required methods.
class MouseButtonRemapper : public PowertoyModuleIface
{
private:
    // The PowerToy state.
    bool m_enabled = false;
    
    // Mouse button hook
    MouseButtonHook m_mouseButtonHook;
    
    // Configuration
    MouseButtonMappingConfiguration m_mappingConfig;
    
    // Load initial settings from the persisted values.
    void init_settings();

public:
    // Constructor
    MouseButtonRemapper()
    {
        LoggerHelpers::init_logger(MODULE_NAME, L"ModuleInterface", LogSettings::mouseButtonRemapperLoggerName);
        init_settings();
    };

    // Destroy the powertoy and free memory
    virtual void destroy() override
    {
        delete this;
    }

    // Return the localized display name of the powertoy
    virtual const wchar_t* get_name() override
    {
        return MODULE_NAME;
    }

    // Return the non localized key of the powertoy, this will be cached by the runner
    virtual const wchar_t* get_key() override
    {
        return MODULE_NAME;
    }

    // Return the configured status for the gpo policy for the module
    virtual powertoys_gpo::gpo_rule_configured_t gpo_policy_enabled_configuration() override
    {
        return powertoys_gpo::gpo_rule_configured_t::not_configured;
    }

    // Return JSON with the configuration options.
    virtual bool get_config(wchar_t* buffer, int* buffer_size) override
    {
        HINSTANCE hinstance = reinterpret_cast<HINSTANCE>(&__ImageBase);

        // Create a Settings object.
        PowerToysSettings::Settings settings(hinstance, get_name());
        settings.set_description(MODULE_DESC);

        // Add bool for enabled
        settings.add_bool_toggle(
            JSON_KEY_ENABLED,
            L"Enable Mouse Button Remapper",
            m_enabled,
            L"Enable or disable remapping of mouse buttons");

        return settings.serialize_to_buffer(buffer, buffer_size);
    }

    // Called by the runner to pass the updated settings values as a serialized JSON.
    virtual void set_config(const wchar_t* config) override
    {
        try
        {
            // Parse the input JSON string.
            PowerToysSettings::PowerToyValues values =
                PowerToysSettings::PowerToyValues::from_json_string(config, get_key());

            // Update the enabled state.
            if (values.is_bool_value(JSON_KEY_ENABLED))
            {
                m_enabled = values.get_bool_value(JSON_KEY_ENABLED);
                
                if (m_enabled)
                {
                    enable();
                }
                else
                {
                    disable();
                }
            }
        }
        catch (std::exception& ex)
        {
            Logger::error(L"Failed to parse Mouse Button Remapper settings: {}", ex.what());
        }
    }

    // Signal from the Settings editor to call a custom action.
    virtual void call_custom_action(const wchar_t* action) override
    {
    }

    // Enable the powertoy
    virtual void enable()
    {
        if (!m_enabled)
        {
            Trace::EnableMouseButtonRemapper(true);
            m_enabled = true;
            
            // Load mappings from settings
            m_mappingConfig.LoadSettings();
            
            // Set the callback for mouse button events
            m_mouseButtonHook.setMouseButtonCallback(
                [&](WPARAM wParam, POINT point, DWORD mouseData) -> bool {
                    return MouseButtonEventHandlers::HandleMouseButtonRemapEvent(
                        wParam, 
                        point, 
                        mouseData, 
                        m_mappingConfig);
                });
                
            // Enable the mouse hook
            m_mouseButtonHook.enable();
            
            Logger::info(L"Mouse Button Remapper enabled");
        }
    }

    // Disable the powertoy
    virtual void disable()
    {
        if (m_enabled)
        {
            Trace::EnableMouseButtonRemapper(false);
            m_enabled = false;
            
            // Disable the mouse hook
            m_mouseButtonHook.disable();
            
            Logger::info(L"Mouse Button Remapper disabled");
        }
    }

    // Returns if the powertoys is enabled
    virtual bool is_enabled() override
    {
        return m_enabled;
    }
};

// Load the settings file.
void MouseButtonRemapper::init_settings()
{
    m_mappingConfig.LoadSettings();
}

extern "C" __declspec(dllexport) PowertoyModuleIface* __cdecl powertoy_create()
{
    return new MouseButtonRemapper();
}

BOOL APIENTRY DllMain(HMODULE hModule, DWORD ul_reason_for_call, LPVOID lpReserved)
{
    m_hModule = hModule;
    switch (ul_reason_for_call)
    {
    case DLL_PROCESS_ATTACH:
        Trace::RegisterProvider();
        break;
    case DLL_THREAD_ATTACH:
    case DLL_THREAD_DETACH:
        break;
    case DLL_PROCESS_DETACH:
        Trace::UnregisterProvider();
        break;
    }
    return TRUE;
}