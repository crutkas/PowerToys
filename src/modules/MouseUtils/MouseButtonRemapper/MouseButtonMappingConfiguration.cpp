#include "pch.h"
#include "MouseButtonMappingConfiguration.h"
#include <common/utils/logger/logger.h>
#include <common/utils/winapi_error.h>
#include <common/SettingsAPI/settings_helpers.h>

// JSON Keys
namespace MBRConfigConstants
{
    const wchar_t REMAPPINGS_ARRAY_KEY[] = L"remappings";
    const wchar_t ORIGINAL_BUTTON_KEY[] = L"originalButton";
    const wchar_t TARGET_TYPE_KEY[] = L"targetType";
    const wchar_t TARGET_VALUE_KEY[] = L"targetValue";
    const wchar_t TARGET_MODIFIERS_KEY[] = L"targetModifiers";
    
    // Target types
    const wchar_t TARGET_TYPE_BUTTON[] = L"button";
    const wchar_t TARGET_TYPE_KEY[] = L"key";
    const wchar_t TARGET_TYPE_KEY_WITH_MODIFIERS[] = L"keyWithModifiers";
    const wchar_t TARGET_TYPE_DISABLED[] = L"disabled";
}

// File to store configurations
std::wstring GetSettingsFileName()
{
    std::wstring saveFolderPath = PTSettingsHelper::get_module_save_folder_location(L"MouseButtonRemapper");
    return saveFolderPath + L"\\config.json";
}

bool MouseButtonMappingConfiguration::LoadSettings()
{
    try
    {
        std::wstring fileName = GetSettingsFileName();
        if (!std::filesystem::exists(fileName))
        {
            Logger::info(L"Mouse Button Remapper config file doesn't exist. Creating new file.");
            return SaveSettingsToFile();
        }

        auto jsonData = json::from_file(fileName);
        if (jsonData)
        {
            ClearMappings();
            return LoadMouseButtonRemaps(*jsonData);
        }
    }
    catch (const std::exception& e)
    {
        Logger::error(L"Failed to load Mouse Button Remapper settings: {}", WindowsGetStringFromUTF8(e.what()));
    }
    return false;
}

bool MouseButtonMappingConfiguration::SaveSettingsToFile()
{
    try
    {
        std::wstring fileName = GetSettingsFileName();
        std::filesystem::path parent_path = std::filesystem::path(fileName).parent_path();
        std::filesystem::create_directories(parent_path);

        json::JsonObject root{};
        json::JsonArray remappingsArray{};

        // Serialize the mappings
        for (const auto& [originalButton, action] : mouseButtonRemapTable)
        {
            json::JsonObject entry{};
            entry.SetNamedValue(MBRConfigConstants::ORIGINAL_BUTTON_KEY, json::value(static_cast<int>(originalButton)));

            // Handle different types of actions
            if (std::holds_alternative<DWORD>(action))
            {
                DWORD targetButton = std::get<DWORD>(action);
                entry.SetNamedValue(MBRConfigConstants::TARGET_TYPE_KEY, json::value(MBRConfigConstants::TARGET_TYPE_BUTTON));
                entry.SetNamedValue(MBRConfigConstants::TARGET_VALUE_KEY, json::value(static_cast<int>(targetButton)));
            }
            else if (std::holds_alternative<std::pair<DWORD, std::vector<DWORD>>>(action))
            {
                auto [targetKey, modifiers] = std::get<std::pair<DWORD, std::vector<DWORD>>>(action);
                entry.SetNamedValue(MBRConfigConstants::TARGET_TYPE_KEY, json::value(MBRConfigConstants::TARGET_TYPE_KEY_WITH_MODIFIERS));
                entry.SetNamedValue(MBRConfigConstants::TARGET_VALUE_KEY, json::value(static_cast<int>(targetKey)));
                
                json::JsonArray modifiersArray{};
                for (DWORD mod : modifiers)
                {
                    modifiersArray.Append(json::value(static_cast<int>(mod)));
                }
                entry.SetNamedValue(MBRConfigConstants::TARGET_MODIFIERS_KEY, modifiersArray);
            }
            else if (std::holds_alternative<std::wstring>(action))
            {
                // Disabled if value is "disabled"
                std::wstring val = std::get<std::wstring>(action);
                if (val == L"disabled")
                {
                    entry.SetNamedValue(MBRConfigConstants::TARGET_TYPE_KEY, json::value(MBRConfigConstants::TARGET_TYPE_DISABLED));
                }
            }

            remappingsArray.Append(entry);
        }

        root.SetNamedValue(MBRConfigConstants::REMAPPINGS_ARRAY_KEY, remappingsArray);
        return json::to_file(fileName, root);
    }
    catch (const std::exception& e)
    {
        Logger::error(L"Failed to save Mouse Button Remapper settings: {}", WindowsGetStringFromUTF8(e.what()));
    }
    return false;
}

void MouseButtonMappingConfiguration::ClearMappings()
{
    mouseButtonRemapTable.clear();
}

bool MouseButtonMappingConfiguration::AddMouseButtonRemap(const DWORD& originalButton, const MappedAction& action)
{
    // Check if the key is already remapped
    auto it = mouseButtonRemapTable.find(originalButton);
    if (it != mouseButtonRemapTable.end())
    {
        return false;
    }

    mouseButtonRemapTable[originalButton] = action;
    return true;
}

bool MouseButtonMappingConfiguration::RemoveMouseButtonRemap(const DWORD& originalButton)
{
    auto it = mouseButtonRemapTable.find(originalButton);
    if (it != mouseButtonRemapTable.end())
    {
        mouseButtonRemapTable.erase(it);
        return true;
    }
    return false;
}

std::optional<MappedAction> MouseButtonMappingConfiguration::GetMouseButtonRemap(const DWORD& originalButton) const
{
    auto it = mouseButtonRemapTable.find(originalButton);
    if (it != mouseButtonRemapTable.end())
    {
        return it->second;
    }
    return std::nullopt;
}

bool MouseButtonMappingConfiguration::LoadMouseButtonRemaps(const json::JsonObject& jsonData)
{
    try
    {
        if (jsonData.HasKey(MBRConfigConstants::REMAPPINGS_ARRAY_KEY))
        {
            auto remappingsArray = jsonData.GetNamedArray(MBRConfigConstants::REMAPPINGS_ARRAY_KEY);
            
            for (uint32_t i = 0; i < remappingsArray.Size(); i++)
            {
                auto entry = remappingsArray.GetObjectAt(i);
                if (entry.HasKey(MBRConfigConstants::ORIGINAL_BUTTON_KEY) && entry.HasKey(MBRConfigConstants::TARGET_TYPE_KEY))
                {
                    DWORD originalButton = static_cast<DWORD>(entry.GetNamedNumber(MBRConfigConstants::ORIGINAL_BUTTON_KEY));
                    std::wstring targetType = entry.GetNamedString(MBRConfigConstants::TARGET_TYPE_KEY);
                    
                    if (targetType == MBRConfigConstants::TARGET_TYPE_BUTTON)
                    {
                        if (entry.HasKey(MBRConfigConstants::TARGET_VALUE_KEY))
                        {
                            DWORD targetButton = static_cast<DWORD>(entry.GetNamedNumber(MBRConfigConstants::TARGET_VALUE_KEY));
                            AddMouseButtonRemap(originalButton, targetButton);
                        }
                    }
                    else if (targetType == MBRConfigConstants::TARGET_TYPE_KEY_WITH_MODIFIERS)
                    {
                        if (entry.HasKey(MBRConfigConstants::TARGET_VALUE_KEY) && entry.HasKey(MBRConfigConstants::TARGET_MODIFIERS_KEY))
                        {
                            DWORD targetKey = static_cast<DWORD>(entry.GetNamedNumber(MBRConfigConstants::TARGET_VALUE_KEY));
                            auto modifiersArray = entry.GetNamedArray(MBRConfigConstants::TARGET_MODIFIERS_KEY);
                            
                            std::vector<DWORD> modifiers;
                            for (uint32_t j = 0; j < modifiersArray.Size(); j++)
                            {
                                modifiers.push_back(static_cast<DWORD>(modifiersArray.GetNumberAt(j)));
                            }
                            
                            AddMouseButtonRemap(originalButton, std::make_pair(targetKey, modifiers));
                        }
                    }
                    else if (targetType == MBRConfigConstants::TARGET_TYPE_DISABLED)
                    {
                        AddMouseButtonRemap(originalButton, std::wstring(L"disabled"));
                    }
                }
            }
            return true;
        }
    }
    catch (const std::exception& e)
    {
        Logger::error(L"Failed to parse Mouse Button Remapper remappings: {}", WindowsGetStringFromUTF8(e.what()));
    }
    return false;
}