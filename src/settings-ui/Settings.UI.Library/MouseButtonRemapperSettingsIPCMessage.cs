// Copyright (c) Microsoft Corporation
// The Microsoft Corporation licenses this file to you under the MIT license.
// See the LICENSE file in the project root for more information.

using System.Text.Json.Serialization;

namespace Microsoft.PowerToys.Settings.UI.Library
{
    public class SndMouseButtonRemapperSettings
    {
        [JsonPropertyName("activation_shortcut")]
        public HotkeySettings ActivationShortcut { get; set; }

        [JsonPropertyName("left_button_mapping")]
        public string LeftButtonMapping { get; set; }

        [JsonPropertyName("right_button_mapping")]
        public string RightButtonMapping { get; set; }

        [JsonPropertyName("middle_button_mapping")]
        public string MiddleButtonMapping { get; set; }

        [JsonPropertyName("x1_button_mapping")]
        public string X1ButtonMapping { get; set; }

        [JsonPropertyName("x2_button_mapping")]
        public string X2ButtonMapping { get; set; }

        [JsonPropertyName("excluded_apps")]
        public string ExcludedApps { get; set; }

        [JsonPropertyName("auto_activate")]
        public bool AutoActivate { get; set; }

        public SndMouseButtonRemapperSettings()
        {
        }

        public SndMouseButtonRemapperSettings(MouseButtonRemapperSettings settings)
        {
            ActivationShortcut = settings.Properties.ActivationShortcut;
            LeftButtonMapping = settings.Properties.LeftButtonMapping.Value;
            RightButtonMapping = settings.Properties.RightButtonMapping.Value;
            MiddleButtonMapping = settings.Properties.MiddleButtonMapping.Value;
            X1ButtonMapping = settings.Properties.X1ButtonMapping.Value;
            X2ButtonMapping = settings.Properties.X2ButtonMapping.Value;
            ExcludedApps = settings.Properties.ExcludedApps.Value;
            AutoActivate = settings.Properties.AutoActivate.Value;
        }
    }
}