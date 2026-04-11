use crate::settings::Language;

/// A letter key that may have associated accented characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LetterKey {
    VK0,
    VK1,
    VK2,
    VK3,
    VK4,
    VK5,
    VK6,
    VK7,
    VK8,
    VK9,
    VkA,
    VkB,
    VkC,
    VkD,
    VkE,
    VkF,
    VkG,
    VkH,
    VkI,
    VkJ,
    VkK,
    VkL,
    VkM,
    VkN,
    VkO,
    VkP,
    VkQ,
    VkR,
    VkS,
    VkT,
    VkU,
    VkV,
    VkW,
    VkX,
    VkY,
    VkZ,
    VkComma,
    VkPeriod,
    VkMinus,
    VkPlus,
}

impl LetterKey {
    /// Convert a Windows virtual-key code to a `LetterKey`, if recognized.
    pub fn from_vk(vk: u32) -> Option<Self> {
        match vk {
            0x30 => Some(Self::VK0),
            0x31 => Some(Self::VK1),
            0x32 => Some(Self::VK2),
            0x33 => Some(Self::VK3),
            0x34 => Some(Self::VK4),
            0x35 => Some(Self::VK5),
            0x36 => Some(Self::VK6),
            0x37 => Some(Self::VK7),
            0x38 => Some(Self::VK8),
            0x39 => Some(Self::VK9),
            0x41 => Some(Self::VkA),
            0x42 => Some(Self::VkB),
            0x43 => Some(Self::VkC),
            0x44 => Some(Self::VkD),
            0x45 => Some(Self::VkE),
            0x46 => Some(Self::VkF),
            0x47 => Some(Self::VkG),
            0x48 => Some(Self::VkH),
            0x49 => Some(Self::VkI),
            0x4A => Some(Self::VkJ),
            0x4B => Some(Self::VkK),
            0x4C => Some(Self::VkL),
            0x4D => Some(Self::VkM),
            0x4E => Some(Self::VkN),
            0x4F => Some(Self::VkO),
            0x50 => Some(Self::VkP),
            0x51 => Some(Self::VkQ),
            0x52 => Some(Self::VkR),
            0x53 => Some(Self::VkS),
            0x54 => Some(Self::VkT),
            0x55 => Some(Self::VkU),
            0x56 => Some(Self::VkV),
            0x57 => Some(Self::VkW),
            0x58 => Some(Self::VkX),
            0x59 => Some(Self::VkY),
            0x5A => Some(Self::VkZ),
            0xBC => Some(Self::VkComma),
            0xBE => Some(Self::VkPeriod),
            0xBD => Some(Self::VkMinus),
            0xBB => Some(Self::VkPlus),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Per-language accent tables
// ---------------------------------------------------------------------------

fn french(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['à', 'â', 'á', 'ä', 'ã', 'æ'],
        LetterKey::VkC => &['ç'],
        LetterKey::VkE => &['é', 'è', 'ê', 'ë', '€'],
        LetterKey::VkI => &['î', 'ï', 'í', 'ì'],
        LetterKey::VkO => &['ô', 'ö', 'ó', 'ò', 'õ', 'œ'],
        LetterKey::VkU => &['û', 'ù', 'ü', 'ú'],
        LetterKey::VkY => &['ÿ', 'ý'],
        _ => &[],
    }
}

fn spanish(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['á'],
        LetterKey::VkE => &['é', '€'],
        LetterKey::VkI => &['í'],
        LetterKey::VkN => &['ñ'],
        LetterKey::VkO => &['ó'],
        LetterKey::VkU => &['ú', 'ü'],
        _ => &[],
    }
}

fn german(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['ä'],
        LetterKey::VkE => &['€'],
        LetterKey::VkO => &['ö'],
        LetterKey::VkS => &['ß'],
        LetterKey::VkU => &['ü'],
        _ => &[],
    }
}

fn portuguese(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['á', 'à', 'â', 'ã'],
        LetterKey::VkC => &['ç'],
        LetterKey::VkE => &['é', 'ê', '€'],
        LetterKey::VkI => &['í'],
        LetterKey::VkO => &['ô', 'ó', 'õ'],
        LetterKey::VkU => &['ú'],
        _ => &[],
    }
}

fn italian(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['à'],
        LetterKey::VkE => &['è', 'é', 'ə', '€'],
        LetterKey::VkI => &['ì', 'í'],
        LetterKey::VkO => &['ò', 'ó'],
        LetterKey::VkU => &['ù', 'ú'],
        _ => &[],
    }
}

fn polish(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['ą'],
        LetterKey::VkC => &['ć'],
        LetterKey::VkE => &['ę', '€'],
        LetterKey::VkL => &['ł'],
        LetterKey::VkN => &['ń'],
        LetterKey::VkO => &['ó'],
        LetterKey::VkS => &['ś'],
        LetterKey::VkZ => &['ż', 'ź'],
        _ => &[],
    }
}

fn czech(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['á'],
        LetterKey::VkC => &['č'],
        LetterKey::VkD => &['ď'],
        LetterKey::VkE => &['é', 'ě'],
        LetterKey::VkI => &['í'],
        LetterKey::VkN => &['ň'],
        LetterKey::VkO => &['ó'],
        LetterKey::VkR => &['ř'],
        LetterKey::VkS => &['š'],
        LetterKey::VkT => &['ť'],
        LetterKey::VkU => &['ú', 'ů'],
        LetterKey::VkY => &['ý'],
        LetterKey::VkZ => &['ž'],
        _ => &[],
    }
}

fn romanian(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['ă', 'â'],
        LetterKey::VkI => &['î'],
        LetterKey::VkS => &['ș'],
        LetterKey::VkT => &['ț'],
        _ => &[],
    }
}

fn swedish(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['å', 'ä'],
        LetterKey::VkO => &['ö'],
        _ => &[],
    }
}

fn turkish(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkC => &['ç'],
        LetterKey::VkG => &['ğ'],
        LetterKey::VkI => &['ı', 'İ'],
        LetterKey::VkO => &['ö'],
        LetterKey::VkS => &['ş'],
        LetterKey::VkU => &['ü'],
        _ => &[],
    }
}

fn dutch(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkE => &['é', 'ë', '€'],
        LetterKey::VkI => &['ï'],
        LetterKey::VkO => &['ö'],
        LetterKey::VkU => &['ü'],
        _ => &[],
    }
}

fn hungarian(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['á'],
        LetterKey::VkE => &['é'],
        LetterKey::VkI => &['í'],
        LetterKey::VkO => &['ó', 'ö', 'ő'],
        LetterKey::VkU => &['ú', 'ü', 'ű'],
        _ => &[],
    }
}

fn catalan(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['à'],
        LetterKey::VkC => &['ç'],
        LetterKey::VkE => &['é', 'è', '€'],
        LetterKey::VkI => &['í', 'ï'],
        LetterKey::VkO => &['ó', 'ò'],
        LetterKey::VkU => &['ú', 'ü'],
        _ => &[],
    }
}

fn croatian(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkC => &['č', 'ć'],
        LetterKey::VkD => &['đ'],
        LetterKey::VkS => &['š'],
        LetterKey::VkZ => &['ž'],
        _ => &[],
    }
}

fn danish(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['å', 'æ'],
        LetterKey::VkO => &['ø'],
        _ => &[],
    }
}

fn norwegian(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['å', 'æ'],
        LetterKey::VkO => &['ø'],
        _ => &[],
    }
}

fn welsh(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkA => &['â'],
        LetterKey::VkE => &['ê'],
        LetterKey::VkI => &['î'],
        LetterKey::VkO => &['ô'],
        LetterKey::VkU => &['û'],
        LetterKey::VkW => &['ŵ'],
        LetterKey::VkY => &['ŷ'],
        _ => &[],
    }
}

fn currency(letter: LetterKey) -> &'static [char] {
    match letter {
        LetterKey::VkE => &['€'],
        LetterKey::VkR => &['₹', '₽'],
        LetterKey::VkY => &['¥'],
        LetterKey::VkL => &['£'],
        LetterKey::VkC => &['¢', '₡'],
        _ => &[],
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Return accents for a single language and letter key.
pub fn get_accents(letter: LetterKey, language: Language) -> &'static [char] {
    match language {
        Language::French => french(letter),
        Language::Spanish => spanish(letter),
        Language::German => german(letter),
        Language::Portuguese => portuguese(letter),
        Language::Italian => italian(letter),
        Language::Polish => polish(letter),
        Language::Czech => czech(letter),
        Language::Romanian => romanian(letter),
        Language::Swedish => swedish(letter),
        Language::Turkish => turkish(letter),
        Language::Dutch => dutch(letter),
        Language::Hungarian => hungarian(letter),
        Language::Catalan => catalan(letter),
        Language::Croatian => croatian(letter),
        Language::Danish => danish(letter),
        Language::Norwegian => norwegian(letter),
        Language::Welsh => welsh(letter),
        Language::Currency => currency(letter),
    }
}

/// Merge accents from multiple languages for a given letter, preserving order
/// and removing duplicates.
pub fn get_accents_for_languages(letter: LetterKey, languages: &[Language]) -> Vec<char> {
    let mut result = Vec::new();
    for &lang in languages {
        for &ch in get_accents(letter, lang) {
            if !result.contains(&ch) {
                result.push(ch);
            }
        }
    }
    result
}

/// Returns `true` if the given letter has at least one accent character in any
/// of the specified languages.
pub fn has_accents(letter: LetterKey, languages: &[Language]) -> bool {
    languages
        .iter()
        .any(|&lang| !get_accents(letter, lang).is_empty())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn french_a_accents() {
        let accents = get_accents(LetterKey::VkA, Language::French);
        assert!(accents.contains(&'à'));
        assert!(accents.contains(&'â'));
        assert!(accents.contains(&'á'));
        assert!(accents.contains(&'ä'));
        assert_eq!(accents.len(), 6);
    }

    #[test]
    fn french_c_cedilla() {
        let accents = get_accents(LetterKey::VkC, Language::French);
        assert_eq!(accents, &['ç']);
    }

    #[test]
    fn french_e_accents() {
        let accents = get_accents(LetterKey::VkE, Language::French);
        assert!(accents.contains(&'é'));
        assert!(accents.contains(&'è'));
        assert!(accents.contains(&'€'));
    }

    #[test]
    fn spanish_n_tilde() {
        let accents = get_accents(LetterKey::VkN, Language::Spanish);
        assert_eq!(accents, &['ñ']);
    }

    #[test]
    fn german_s_sharp() {
        let accents = get_accents(LetterKey::VkS, Language::German);
        assert_eq!(accents, &['ß']);
    }

    #[test]
    fn german_u_umlaut() {
        let accents = get_accents(LetterKey::VkU, Language::German);
        assert_eq!(accents, &['ü']);
    }

    #[test]
    fn polish_z_accents() {
        let accents = get_accents(LetterKey::VkZ, Language::Polish);
        assert!(accents.contains(&'ż'));
        assert!(accents.contains(&'ź'));
    }

    #[test]
    fn italian_a_grave() {
        let accents = get_accents(LetterKey::VkA, Language::Italian);
        assert_eq!(accents, &['à']);
    }

    #[test]
    fn portuguese_a_accents() {
        let accents = get_accents(LetterKey::VkA, Language::Portuguese);
        assert!(accents.contains(&'á'));
        assert!(accents.contains(&'ã'));
    }

    #[test]
    fn unknown_letter_returns_empty() {
        let accents = get_accents(LetterKey::VkX, Language::French);
        assert!(accents.is_empty());
    }

    #[test]
    fn merged_languages_dedup() {
        // French 'e': é è ê ë €
        // Spanish 'e': é €
        // German 'e': €
        let merged =
            get_accents_for_languages(LetterKey::VkE, &[Language::French, Language::Spanish, Language::German]);
        // 'é' and '€' should not be duplicated
        assert_eq!(
            merged.iter().filter(|&&c| c == 'é').count(),
            1,
            "é should appear exactly once"
        );
        assert_eq!(
            merged.iter().filter(|&&c| c == '€').count(),
            1,
            "€ should appear exactly once"
        );
        // Order follows French first
        assert_eq!(merged[0], 'é');
    }

    #[test]
    fn has_accents_true_for_french_a() {
        assert!(has_accents(LetterKey::VkA, &[Language::French]));
    }

    #[test]
    fn has_accents_false_for_unknown() {
        assert!(!has_accents(LetterKey::VkX, &[Language::French]));
    }

    #[test]
    fn disabled_language_excluded() {
        // Only Spanish enabled — German ß should not appear for S
        let accents = get_accents_for_languages(LetterKey::VkS, &[Language::Spanish]);
        assert!(!accents.contains(&'ß'));
        assert!(accents.is_empty());
    }

    #[test]
    fn from_vk_letter_keys() {
        assert_eq!(LetterKey::from_vk(0x41), Some(LetterKey::VkA));
        assert_eq!(LetterKey::from_vk(0x5A), Some(LetterKey::VkZ));
        assert_eq!(LetterKey::from_vk(0x30), Some(LetterKey::VK0));
        assert_eq!(LetterKey::from_vk(0xFF), None);
    }

    #[test]
    fn from_vk_punctuation_keys() {
        assert_eq!(LetterKey::from_vk(0xBC), Some(LetterKey::VkComma));
        assert_eq!(LetterKey::from_vk(0xBE), Some(LetterKey::VkPeriod));
        assert_eq!(LetterKey::from_vk(0xBD), Some(LetterKey::VkMinus));
        assert_eq!(LetterKey::from_vk(0xBB), Some(LetterKey::VkPlus));
    }

    #[test]
    fn currency_euro() {
        let accents = get_accents(LetterKey::VkE, Language::Currency);
        assert_eq!(accents, &['€']);
    }

    #[test]
    fn czech_c_caron() {
        let accents = get_accents(LetterKey::VkC, Language::Czech);
        assert_eq!(accents, &['č']);
    }

    #[test]
    fn merged_empty_for_no_languages() {
        let merged = get_accents_for_languages(LetterKey::VkA, &[]);
        assert!(merged.is_empty());
    }
}
