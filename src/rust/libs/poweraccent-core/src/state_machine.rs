use crate::accent_map::{self, LetterKey};
use crate::settings::Settings;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Trigger key that activates or navigates the accent picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerKey {
    Space,
    Left,
    Right,
}

impl TriggerKey {
    /// Try to interpret a Windows virtual-key code as a trigger key.
    pub fn from_vk(vk: u32) -> Option<Self> {
        match vk {
            0x20 => Some(Self::Space),
            0x25 => Some(Self::Left),
            0x27 => Some(Self::Right),
            _ => None,
        }
    }
}

/// An action the host should perform in response to a key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Display the accent picker for the given letter.
    ShowAccents {
        letter: LetterKey,
        accents: Vec<char>,
    },
    /// Update the currently highlighted accent character.
    UpdateSelection {
        index: usize,
        ch: char,
    },
    /// Emit the selected accent character (via SendInput).
    EmitAccent {
        ch: char,
    },
    /// Emit the original trigger key instead of an accent (false-start).
    EmitTrigger {
        trigger: TriggerKey,
    },
    /// Hide the accent picker without selecting a character.
    HideAccents,
}

/// Result returned by the state machine for each key event.
#[derive(Debug, Clone)]
pub struct KeyResult {
    /// Whether the key should be suppressed (not forwarded to the application).
    pub suppress: bool,
    /// Actions the host should perform.
    pub actions: Vec<Action>,
}

impl KeyResult {
    fn pass_through() -> Self {
        Self {
            suppress: false,
            actions: Vec::new(),
        }
    }

    fn suppressed(actions: Vec<Action>) -> Self {
        Self {
            suppress: true,
            actions,
        }
    }
}

// ---------------------------------------------------------------------------
// Internal state
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum State {
    /// No letter is being held.
    Idle,
    /// A letter key is held but the accent picker is not yet visible.
    LetterHeld {
        letter: LetterKey,
        press_time_ms: u64,
    },
    /// The accent picker is visible and the user is cycling through characters.
    AccentActive {
        letter: LetterKey,
        accents: Vec<char>,
        selected_index: usize,
        press_time_ms: u64,
        trigger: TriggerKey,
    },
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

/// Pure-logic accent selection state machine.
///
/// Feed key-down / key-up events and receive [`KeyResult`]s describing whether
/// the key should be suppressed and what actions the host should take.
pub struct AccentStateMachine {
    state: State,
    settings: Settings,
}

impl AccentStateMachine {
    pub fn new(settings: Settings) -> Self {
        Self {
            state: State::Idle,
            settings,
        }
    }

    /// Replace the settings (e.g. after the user changes them).
    pub fn update_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// The currently selected accent character, if any.
    pub fn selected_char(&self) -> Option<char> {
        match &self.state {
            State::AccentActive {
                accents,
                selected_index,
                ..
            } => accents.get(*selected_index).copied(),
            _ => None,
        }
    }

    // -- events -------------------------------------------------------------

    /// Process a key-down event.
    pub fn on_key_down(&mut self, vk_code: u32, time_ms: u64, shift_held: bool) -> KeyResult {
        // Check if this is a trigger key
        if let Some(trigger) = TriggerKey::from_vk(vk_code) {
            return self.handle_trigger_down(trigger, shift_held);
        }

        // Check if this is a letter key
        if let Some(letter) = LetterKey::from_vk(vk_code) {
            return self.handle_letter_down(letter, time_ms);
        }

        KeyResult::pass_through()
    }

    /// Process a key-up event.
    pub fn on_key_up(&mut self, vk_code: u32, time_ms: u64) -> KeyResult {
        if let Some(letter) = LetterKey::from_vk(vk_code) {
            return self.handle_letter_up(letter, time_ms);
        }

        KeyResult::pass_through()
    }

    // -- internal -----------------------------------------------------------

    fn handle_letter_down(&mut self, letter: LetterKey, time_ms: u64) -> KeyResult {
        match &self.state {
            State::Idle => {
                if accent_map::has_accents(letter, &self.settings.selected_languages) {
                    self.state = State::LetterHeld {
                        letter,
                        press_time_ms: time_ms,
                    };
                }
                KeyResult::pass_through()
            }
            // Suppress SAME letter repeat while accent picker is active
            // (on-screen keyboard sends WM_KEYDOWN continuously while key held)
            // https://github.com/microsoft/PowerToys/issues/36853
            State::AccentActive {
                letter: active_letter,
                ..
            } => {
                if letter == *active_letter {
                    KeyResult::suppressed(vec![])
                } else {
                    KeyResult::pass_through()
                }
            }
            // Ignore repeat while tracking but not yet active
            State::LetterHeld { .. } => KeyResult::pass_through(),
        }
    }

    fn handle_trigger_down(&mut self, trigger: TriggerKey, shift_held: bool) -> KeyResult {
        if !self.settings.is_trigger_allowed(trigger) {
            return KeyResult::pass_through();
        }

        match self.state.clone() {
            State::LetterHeld {
                letter,
                press_time_ms,
            } => {
                let accents =
                    accent_map::get_accents_for_languages(letter, &self.settings.selected_languages);
                if accents.is_empty() {
                    return KeyResult::pass_through();
                }
                let idx = if self.settings.start_selection_from_left {
                    0
                } else {
                    0
                };
                self.state = State::AccentActive {
                    letter,
                    accents: accents.clone(),
                    selected_index: idx,
                    press_time_ms,
                    trigger,
                };
                KeyResult::suppressed(vec![
                    Action::ShowAccents { letter, accents },
                    Action::UpdateSelection {
                        index: idx,
                        ch: self.selected_char().unwrap(),
                    },
                ])
            }
            State::AccentActive {
                letter,
                accents,
                selected_index,
                press_time_ms,
                trigger: original_trigger,
            } => {
                let len = accents.len();
                let new_index = match trigger {
                    TriggerKey::Right => (selected_index + 1) % len,
                    TriggerKey::Left => (selected_index + len - 1) % len,
                    TriggerKey::Space => {
                        if shift_held {
                            (selected_index + len - 1) % len
                        } else {
                            (selected_index + 1) % len
                        }
                    }
                };
                self.state = State::AccentActive {
                    letter,
                    accents: accents.clone(),
                    selected_index: new_index,
                    press_time_ms,
                    trigger: original_trigger,
                };
                KeyResult::suppressed(vec![Action::UpdateSelection {
                    index: new_index,
                    ch: accents[new_index],
                }])
            }
            State::Idle => KeyResult::pass_through(),
        }
    }

    fn handle_letter_up(&mut self, letter: LetterKey, time_ms: u64) -> KeyResult {
        match self.state.clone() {
            State::LetterHeld {
                letter: held,
                ..
            } if held == letter => {
                self.state = State::Idle;
                KeyResult::pass_through()
            }
            State::AccentActive {
                letter: held,
                accents,
                selected_index,
                press_time_ms,
                trigger,
            } if held == letter => {
                self.state = State::Idle;
                let elapsed = time_ms.saturating_sub(press_time_ms);
                if elapsed < self.settings.input_time_ms {
                    // False start — emit the trigger key instead
                    KeyResult::suppressed(vec![
                        Action::HideAccents,
                        Action::EmitTrigger { trigger },
                    ])
                } else {
                    // Commit the selected accent
                    KeyResult::suppressed(vec![
                        Action::HideAccents,
                        Action::EmitAccent {
                            ch: accents[selected_index],
                        },
                    ])
                }
            }
            _ => KeyResult::pass_through(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{ActivationKey, Language};

    fn make_sm() -> AccentStateMachine {
        let settings = Settings {
            activation_key: ActivationKey::Both,
            input_time_ms: 300,
            selected_languages: vec![Language::French],
            ..Default::default()
        };
        AccentStateMachine::new(settings)
    }

    const VK_A: u32 = 0x41;
    const VK_E: u32 = 0x45;
    const VK_X: u32 = 0x58;
    const VK_SPACE: u32 = 0x20;
    const VK_LEFT: u32 = 0x25;
    const VK_RIGHT: u32 = 0x27;

    // --- hold 'a' + space → first accent ---
    #[test]
    fn hold_a_space_returns_first_accent() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        let r = sm.on_key_down(VK_SPACE, 50, false);
        assert!(r.suppress);
        // ShowAccents should be in actions
        assert!(r.actions.iter().any(|a| matches!(a, Action::ShowAccents { .. })));
        // Release after enough time → accent
        let r = sm.on_key_up(VK_A, 500);
        assert!(r.suppress);
        let emit = r.actions.iter().find_map(|a| match a {
            Action::EmitAccent { ch } => Some(*ch),
            _ => None,
        });
        assert_eq!(emit, Some('à'));
    }

    // --- hold 'a' + space + space → cycles to second accent ---
    #[test]
    fn hold_a_space_space_cycles() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        sm.on_key_down(VK_SPACE, 50, false);
        let r = sm.on_key_down(VK_SPACE, 100, false);
        assert!(r.suppress);
        let update = r.actions.iter().find_map(|a| match a {
            Action::UpdateSelection { ch, .. } => Some(*ch),
            _ => None,
        });
        assert_eq!(update, Some('â')); // second French 'a' accent
    }

    // --- release without trigger → no accent ---
    #[test]
    fn release_without_trigger_no_accent() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        let r = sm.on_key_up(VK_A, 500);
        assert!(!r.suppress);
        assert!(r.actions.is_empty());
    }

    // --- arrow right cycles forward ---
    #[test]
    fn arrow_right_cycles_forward() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        sm.on_key_down(VK_SPACE, 50, false);
        let r = sm.on_key_down(VK_RIGHT, 100, false);
        assert!(r.suppress);
        let update = r.actions.iter().find_map(|a| match a {
            Action::UpdateSelection { ch, .. } => Some(*ch),
            _ => None,
        });
        assert_eq!(update, Some('â'));
    }

    // --- arrow left cycles backward (wraps) ---
    #[test]
    fn arrow_left_wraps_to_end() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        sm.on_key_down(VK_SPACE, 50, false);
        // Currently at index 0 ('à'), left should wrap to last ('æ')
        let r = sm.on_key_down(VK_LEFT, 100, false);
        let update = r.actions.iter().find_map(|a| match a {
            Action::UpdateSelection { ch, .. } => Some(*ch),
            _ => None,
        });
        assert_eq!(update, Some('æ'));
    }

    // --- false start: release within input_time emits trigger ---
    #[test]
    fn false_start_emits_trigger() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        sm.on_key_down(VK_SPACE, 50, false);
        // Release quickly (< 300ms)
        let r = sm.on_key_up(VK_A, 100);
        assert!(r.suppress);
        let trigger = r.actions.iter().find_map(|a| match a {
            Action::EmitTrigger { trigger } => Some(*trigger),
            _ => None,
        });
        assert_eq!(trigger, Some(TriggerKey::Space));
    }

    // --- release after input_time commits accent ---
    #[test]
    fn release_after_input_time_commits_accent() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        sm.on_key_down(VK_SPACE, 50, false);
        let r = sm.on_key_up(VK_A, 500);
        assert!(r.suppress);
        let emit = r.actions.iter().find_map(|a| match a {
            Action::EmitAccent { ch } => Some(*ch),
            _ => None,
        });
        assert_eq!(emit, Some('à'));
    }

    // --- shift+space cycles backward ---
    #[test]
    fn shift_space_cycles_backward() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        sm.on_key_down(VK_SPACE, 50, false); // index 0 = 'à'
        // Forward once
        sm.on_key_down(VK_SPACE, 100, false); // index 1 = 'â'
        // Now shift+space should go back
        let r = sm.on_key_down(VK_SPACE, 150, true);
        let update = r.actions.iter().find_map(|a| match a {
            Action::UpdateSelection { ch, .. } => Some(*ch),
            _ => None,
        });
        assert_eq!(update, Some('à'));
    }

    // --- wraps around at end of accent list ---
    #[test]
    fn wraps_around_at_end() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        sm.on_key_down(VK_SPACE, 50, false); // index 0
        // French 'a' has 6 accents: à â á ä ã æ
        for _ in 0..6 {
            sm.on_key_down(VK_SPACE, 100, false);
        }
        // After 6 advances from index 0 we should wrap back to 0
        let ch = sm.selected_char();
        assert_eq!(ch, Some('à'));
    }

    // --- space-only activation ignores arrows ---
    #[test]
    fn space_only_ignores_arrows() {
        let settings = Settings {
            activation_key: ActivationKey::Space,
            input_time_ms: 300,
            selected_languages: vec![Language::French],
            ..Default::default()
        };
        let mut sm = AccentStateMachine::new(settings);
        sm.on_key_down(VK_A, 0, false);
        let r = sm.on_key_down(VK_RIGHT, 50, false);
        assert!(!r.suppress);
        assert!(r.actions.is_empty());
    }

    // --- arrow-only activation ignores space ---
    #[test]
    fn arrow_only_ignores_space() {
        let settings = Settings {
            activation_key: ActivationKey::LeftRightArrow,
            input_time_ms: 300,
            selected_languages: vec![Language::French],
            ..Default::default()
        };
        let mut sm = AccentStateMachine::new(settings);
        sm.on_key_down(VK_A, 0, false);
        let r = sm.on_key_down(VK_SPACE, 50, false);
        assert!(!r.suppress);
        assert!(r.actions.is_empty());
    }

    // --- key without accents does nothing ---
    #[test]
    fn key_without_accents_passthrough() {
        let mut sm = make_sm();
        sm.on_key_down(VK_X, 0, false);
        let r = sm.on_key_down(VK_SPACE, 50, false);
        assert!(!r.suppress);
    }

    // --- 'e' in French has accents ---
    #[test]
    fn e_french_accents() {
        let mut sm = make_sm();
        sm.on_key_down(VK_E, 0, false);
        let r = sm.on_key_down(VK_SPACE, 50, false);
        assert!(r.suppress);
        let accents = r.actions.iter().find_map(|a| match a {
            Action::ShowAccents { accents, .. } => Some(accents.clone()),
            _ => None,
        });
        let accents = accents.unwrap();
        assert!(accents.contains(&'é'));
        assert!(accents.contains(&'è'));
    }

    // --- idle state: trigger key passes through ---
    #[test]
    fn idle_trigger_passes_through() {
        let mut sm = make_sm();
        let r = sm.on_key_down(VK_SPACE, 0, false);
        assert!(!r.suppress);
    }

    // --- update_settings changes behavior ---
    #[test]
    fn update_settings_changes_behavior() {
        let mut sm = make_sm();
        sm.update_settings(Settings {
            input_time_ms: 50,
            selected_languages: vec![Language::French],
            ..Default::default()
        });
        sm.on_key_down(VK_A, 0, false);
        sm.on_key_down(VK_SPACE, 10, false);
        // Release at 60ms > 50ms threshold → should commit accent (not false start)
        let r = sm.on_key_up(VK_A, 60);
        assert!(r.actions.iter().any(|a| matches!(a, Action::EmitAccent { .. })));
    }

    // --- release different key while accent active → pass through ---
    #[test]
    fn release_different_key_pass_through() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        sm.on_key_down(VK_SPACE, 50, false);
        let r = sm.on_key_up(VK_E, 500);
        assert!(!r.suppress);
        // State should still be AccentActive
        assert!(sm.selected_char().is_some());
    }

    // --- OSK repeat suppression: repeated letter while accent picker visible ---
    // https://github.com/microsoft/PowerToys/issues/36853
    #[test]
    fn osk_repeat_letter_suppressed_while_accent_active() {
        let mut sm = make_sm();
        sm.on_key_down(VK_A, 0, false);
        sm.on_key_down(VK_SPACE, 50, false);
        assert!(sm.selected_char().is_some()); // accent picker visible

        // Repeated letter key (as OSK does continuously) must be suppressed
        let r = sm.on_key_down(VK_A, 100, false);
        assert!(r.suppress, "Repeated letter while accent active must be suppressed");
        assert!(r.actions.is_empty(), "No actions needed for repeat suppression");
    }

    // --- Different letter while accent active passes through (C++ behavior) ---
    #[test]
    fn different_letter_while_accent_active_passes_through() {
        let mut sm = make_sm();
        sm.on_key_down(VK_E, 0, false);
        sm.on_key_down(VK_SPACE, 50, false);
        assert!(sm.selected_char().is_some());

        // Different letter passes through — only SAME letter is suppressed
        let r = sm.on_key_down(VK_A, 100, false);
        assert!(!r.suppress, "Different letter should pass through per C++ behavior");
    }
}
