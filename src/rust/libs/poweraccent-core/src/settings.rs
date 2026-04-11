use serde::{Deserialize, Serialize};

/// Activation key mode — which trigger keys open the accent picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActivationKey {
    Space,
    LeftRightArrow,
    Both,
}

impl Default for ActivationKey {
    fn default() -> Self {
        Self::Both
    }
}

/// Supported accent languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    French,
    Spanish,
    German,
    Portuguese,
    Italian,
    Polish,
    Czech,
    Romanian,
    Swedish,
    Turkish,
    Dutch,
    Hungarian,
    Catalan,
    Croatian,
    Danish,
    Norwegian,
    Welsh,
    Currency,
}

/// PowerAccent settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Which trigger key(s) activate accent selection.
    #[serde(default)]
    pub activation_key: ActivationKey,

    /// Minimum hold time in milliseconds before accent selection is committed.
    /// Below this threshold a key release is treated as a "false start" and the
    /// trigger key is emitted instead.
    #[serde(default = "default_input_time")]
    pub input_time_ms: u64,

    /// Active languages whose accent sets are merged.
    #[serde(default = "default_languages")]
    pub selected_languages: Vec<Language>,

    /// Newline-separated list of excluded application executable names.
    #[serde(default)]
    pub excluded_apps: Vec<String>,

    /// When true, do not activate when Windows Game Mode is on.
    #[serde(default = "default_true")]
    pub do_not_activate_on_game_mode: bool,

    /// Start accent selection from the leftmost character.
    #[serde(default)]
    pub start_selection_from_left: bool,

    /// Sort accents by recent usage frequency.
    #[serde(default)]
    pub sort_by_usage_frequency: bool,
}

fn default_input_time() -> u64 {
    300
}

fn default_languages() -> Vec<Language> {
    vec![Language::French]
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            activation_key: ActivationKey::default(),
            input_time_ms: default_input_time(),
            selected_languages: default_languages(),
            excluded_apps: Vec::new(),
            do_not_activate_on_game_mode: true,
            start_selection_from_left: false,
            sort_by_usage_frequency: false,
        }
    }
}

impl Settings {
    /// Returns true if the given trigger key is allowed by the current activation mode.
    pub fn is_trigger_allowed(&self, trigger: crate::state_machine::TriggerKey) -> bool {
        use crate::state_machine::TriggerKey;
        match self.activation_key {
            ActivationKey::Both => true,
            ActivationKey::Space => trigger == TriggerKey::Space,
            ActivationKey::LeftRightArrow => {
                matches!(trigger, TriggerKey::Left | TriggerKey::Right)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_machine::TriggerKey;

    #[test]
    fn default_settings() {
        let s = Settings::default();
        assert_eq!(s.activation_key, ActivationKey::Both);
        assert_eq!(s.input_time_ms, 300);
        assert!(!s.selected_languages.is_empty());
    }

    #[test]
    fn trigger_allowed_both() {
        let s = Settings::default(); // Both
        assert!(s.is_trigger_allowed(TriggerKey::Space));
        assert!(s.is_trigger_allowed(TriggerKey::Left));
        assert!(s.is_trigger_allowed(TriggerKey::Right));
    }

    #[test]
    fn trigger_allowed_space_only() {
        let s = Settings {
            activation_key: ActivationKey::Space,
            ..Default::default()
        };
        assert!(s.is_trigger_allowed(TriggerKey::Space));
        assert!(!s.is_trigger_allowed(TriggerKey::Left));
        assert!(!s.is_trigger_allowed(TriggerKey::Right));
    }

    #[test]
    fn trigger_allowed_arrows_only() {
        let s = Settings {
            activation_key: ActivationKey::LeftRightArrow,
            ..Default::default()
        };
        assert!(!s.is_trigger_allowed(TriggerKey::Space));
        assert!(s.is_trigger_allowed(TriggerKey::Left));
        assert!(s.is_trigger_allowed(TriggerKey::Right));
    }

    #[test]
    fn serde_roundtrip() {
        let s = Settings::default();
        let json = serde_json::to_string(&s).unwrap();
        let s2: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s.input_time_ms, s2.input_time_ms);
        assert_eq!(s.activation_key, s2.activation_key);
    }
}
