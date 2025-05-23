// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

using System;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Microsoft.PowerToys.Settings.UI.Library
{
    public class MouseButtonRemapperSettings
    {
        public const string ModuleName = "MouseButtonRemapper";

        [JsonPropertyName("properties")]
        public MouseButtonRemapperProperties Properties { get; set; }

        public MouseButtonRemapperSettings()
        {
            Properties = new MouseButtonRemapperProperties();
        }

        public MouseButtonRemapperSettings(string settingsJson)
        {
            try
            {
                if (string.IsNullOrWhiteSpace(settingsJson))
                {
                    Properties = new MouseButtonRemapperProperties();
                }
                else
                {
                    Properties = JsonSerializer.Deserialize<MouseButtonRemapperProperties>(settingsJson);
                }
            }
            catch (Exception ex)
            {
                Properties = new MouseButtonRemapperProperties();
            }
        }

        public string ToJsonString()
        {
            return JsonSerializer.Serialize(this);
        }
    }

    public class MouseButtonRemapperProperties
    {
        [JsonPropertyName("activation_shortcut")]
        public HotkeySettings ActivationShortcut { get; set; }

        [JsonPropertyName("left_button_mapping")]
        public StringProperty LeftButtonMapping { get; set; }

        [JsonPropertyName("right_button_mapping")]
        public StringProperty RightButtonMapping { get; set; }

        [JsonPropertyName("middle_button_mapping")]
        public StringProperty MiddleButtonMapping { get; set; }

        [JsonPropertyName("x1_button_mapping")]
        public StringProperty X1ButtonMapping { get; set; }

        [JsonPropertyName("x2_button_mapping")]
        public StringProperty X2ButtonMapping { get; set; }

        [JsonPropertyName("excluded_apps")]
        public StringProperty ExcludedApps { get; set; }

        [JsonPropertyName("auto_activate")]
        public BoolProperty AutoActivate { get; set; }

        [JsonPropertyName("default_activation_shortcut")]
        public HotkeySettings DefaultActivationShortcut { get; }

        public MouseButtonRemapperProperties()
        {
            ActivationShortcut = new HotkeySettings(true, false, false, false, 0x4D); // Win + M
            DefaultActivationShortcut = new HotkeySettings(true, false, false, false, 0x4D); // Win + M
            
            LeftButtonMapping = new StringProperty { Value = string.Empty };
            RightButtonMapping = new StringProperty { Value = string.Empty };
            MiddleButtonMapping = new StringProperty { Value = string.Empty };
            X1ButtonMapping = new StringProperty { Value = string.Empty };
            X2ButtonMapping = new StringProperty { Value = string.Empty };
            
            ExcludedApps = new StringProperty { Value = string.Empty };
            AutoActivate = new BoolProperty { Value = false };
        }
    }
}