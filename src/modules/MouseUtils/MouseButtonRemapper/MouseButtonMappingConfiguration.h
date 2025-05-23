#pragma once

#include <common/utils/json.h>
#include <variant>
#include <string>
#include <map>
#include <optional>

// Define constants for Mouse Buttons
constexpr DWORD MB_LEFT_BUTTON = VK_LBUTTON;     // 0x01
constexpr DWORD MB_RIGHT_BUTTON = VK_RBUTTON;    // 0x02
constexpr DWORD MB_MIDDLE_BUTTON = VK_MBUTTON;   // 0x04
constexpr DWORD MB_X1_BUTTON = VK_XBUTTON1;      // 0x05
constexpr DWORD MB_X2_BUTTON = VK_XBUTTON2;      // 0x06

// For mapping mouse buttons to keyboard keys or other mouse buttons
using MappedAction = std::variant<DWORD, std::pair<DWORD, std::vector<DWORD>>, std::wstring>;

// Map: Original Mouse Button -> Target Action
using MouseButtonRemapTable = std::map<DWORD, MappedAction>;

class MouseButtonMappingConfiguration
{
public:
    MouseButtonMappingConfiguration() = default;
    ~MouseButtonMappingConfiguration() = default;

    // Load the configuration
    bool LoadSettings();

    // Save the configuration
    bool SaveSettingsToFile();

    // Clear all mappings
    void ClearMappings();

    // Add a new mouse button mapping
    bool AddMouseButtonRemap(const DWORD& originalButton, const MappedAction& action);

    // Remove a mouse button mapping
    bool RemoveMouseButtonRemap(const DWORD& originalButton);

    // Get the mapped action for a mouse button
    std::optional<MappedAction> GetMouseButtonRemap(const DWORD& originalButton) const;

    // The map that stores the remappings
    MouseButtonRemapTable mouseButtonRemapTable;

private:
    bool LoadMouseButtonRemaps(const json::JsonObject& jsonData);
    
    // Current configuration name
    std::wstring currentConfig = L"Default";
};