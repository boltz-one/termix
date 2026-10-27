// Unicode character width detection
// Port: types/inc/CodepointWidthDetector.hpp, types/CodepointWidthDetector.cpp

/// Get display width of a Unicode codepoint (columns)
pub fn codepoint_width(codepoint: u32) -> u8 {
    // Control characters: 0 width
    if codepoint < 0x20 || (0x7F..=0x9F).contains(&codepoint) {
        return 0;
    }

    // ASCII printable: 1 width
    if (0x20..=0x7E).contains(&codepoint) {
        return 1;
    }

    // Check combining marks (zero-width)
    if is_combining(codepoint) {
        return 0;
    }

    // Check East Asian Width
    if is_east_asian_fullwidth(codepoint) || is_east_asian_wide(codepoint) {
        return 2;
    }

    // Check emoji presentation
    if is_emoji_presentation(codepoint) {
        return 2;
    }

    // Default: narrow (1 width)
    1
}

/// Check if codepoint is a combining mark (zero-width)
fn is_combining(cp: u32) -> bool {
    matches!(cp,
        // Combining Diacritical Marks
        0x0300..=0x036F |
        // Combining Diacritical Marks Extended
        0x1AB0..=0x1AFF |
        // Combining Diacritical Marks Supplement
        0x1DC0..=0x1DFF |
        // Combining Diacritical Marks for Symbols
        0x20D0..=0x20FF |
        // Combining Half Marks
        0xFE20..=0xFE2F |
        // Zero Width Joiner/Non-Joiner
        0x200B..=0x200F |
        // Variation Selectors
        0xFE00..=0xFE0F |
        0xE0100..=0xE01EF
    )
}

/// Check if codepoint is East Asian Fullwidth
fn is_east_asian_fullwidth(cp: u32) -> bool {
    matches!(cp,
        // Fullwidth ASCII variants
        0xFF01..=0xFF60 |
        // Fullwidth symbol variants
        0xFFE0..=0xFFE6
    )
}

/// Check if codepoint is East Asian Wide
fn is_east_asian_wide(cp: u32) -> bool {
    matches!(cp,
        // CJK Unified Ideographs Extension A
        0x3400..=0x4DBF |
        // CJK Unified Ideographs
        0x4E00..=0x9FFF |
        // Hangul Syllables
        0xAC00..=0xD7AF |
        // CJK Compatibility Ideographs
        0xF900..=0xFAFF |
        // CJK Unified Ideographs Extension B-F
        0x20000..=0x2FFFF |
        // CJK Compatibility Ideographs Supplement
        0x2F800..=0x2FA1F |
        // Hiragana
        0x3040..=0x309F |
        // Katakana
        0x30A0..=0x30FF |
        // Katakana Phonetic Extensions
        0x31F0..=0x31FF |
        // Enclosed CJK Letters and Months
        0x3200..=0x32FF |
        // CJK Compatibility
        0x3300..=0x33FF |
        // CJK Unified Ideographs Extension A
        0x3400..=0x4DBF
    )
}

/// Check if codepoint has emoji presentation (default wide)
fn is_emoji_presentation(cp: u32) -> bool {
    matches!(cp,
        // Emoticons
        0x1F600..=0x1F64F |
        // Miscellaneous Symbols and Pictographs
        0x1F300..=0x1F5FF |
        // Transport and Map Symbols
        0x1F680..=0x1F6FF |
        // Supplemental Symbols and Pictographs
        0x1F900..=0x1F9FF |
        // Symbols and Pictographs Extended-A
        0x1FA70..=0x1FAFF |
        // Some common emoji
        0x2600..=0x26FF | // Miscellaneous Symbols
        0x2700..=0x27BF   // Dingbats
    )
}

/// Get width of a string in terminal columns
pub fn string_width(s: &str) -> usize {
    s.chars().map(|c| codepoint_width(c as u32) as usize).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_printable_width_1() {
        assert_eq!(codepoint_width(b'A' as u32), 1);
        assert_eq!(codepoint_width(b'z' as u32), 1);
        assert_eq!(codepoint_width(b'0' as u32), 1);
        assert_eq!(codepoint_width(b' ' as u32), 1);
    }

    #[test]
    fn control_chars_width_0() {
        assert_eq!(codepoint_width(0x00), 0); // NUL
        assert_eq!(codepoint_width(0x1B), 0); // ESC
        assert_eq!(codepoint_width(0x7F), 0); // DEL
        assert_eq!(codepoint_width(0x9F), 0); // Control
    }

    #[test]
    fn combining_marks_width_0() {
        assert_eq!(codepoint_width(0x0301), 0); // Combining acute accent
        assert_eq!(codepoint_width(0x0308), 0); // Combining diaeresis
        assert_eq!(codepoint_width(0x20DD), 0); // Combining enclosing circle
    }

    #[test]
    fn cjk_ideographs_width_2() {
        assert_eq!(codepoint_width(0x4E00), 2); // CJK Unified Ideograph
        assert_eq!(codepoint_width(0x4E01), 2);
        assert_eq!(codepoint_width(0x9FFF), 2);
    }

    #[test]
    fn hangul_width_2() {
        assert_eq!(codepoint_width(0xAC00), 2); // Hangul syllable GA
        assert_eq!(codepoint_width(0xD7AF), 2); // Last Hangul syllable
    }

    #[test]
    fn hiragana_katakana_width_2() {
        assert_eq!(codepoint_width(0x3042), 2); // Hiragana A
        assert_eq!(codepoint_width(0x30A2), 2); // Katakana A
    }

    #[test]
    fn fullwidth_ascii_width_2() {
        assert_eq!(codepoint_width(0xFF21), 2); // Fullwidth Latin A
        assert_eq!(codepoint_width(0xFF41), 2); // Fullwidth Latin a
    }

    #[test]
    fn emoji_width_2() {
        assert_eq!(codepoint_width(0x1F600), 2); // Grinning face
        assert_eq!(codepoint_width(0x1F680), 2); // Rocket
        assert_eq!(codepoint_width(0x2764), 2);  // Heavy black heart
    }

    #[test]
    fn string_width_ascii() {
        assert_eq!(string_width("Hello"), 5);
        assert_eq!(string_width("Test 123"), 8);
    }

    #[test]
    fn string_width_mixed() {
        // "Hello" (5) + "世" (2) + "界" (2) = 9
        assert_eq!(string_width("Hello世界"), 9);
    }

    #[test]
    fn string_width_combining() {
        // "e" + combining acute = 1 column
        assert_eq!(string_width("é"), 1); // e + U+0301
    }

    #[test]
    fn string_width_emoji() {
        // Note: Emoji width may vary by terminal/font
        assert_eq!(string_width("🚀"), 2);
        assert_eq!(string_width("Hello 🚀"), 8); // "Hello " (6) + "🚀" (2)
    }

    #[test]
    fn cjk_extension_b_width_2() {
        assert_eq!(codepoint_width(0x20000), 2); // CJK Extension B
    }

    #[test]
    fn latin_extended_width_1() {
        assert_eq!(codepoint_width(0x00E9), 1); // é (precomposed)
        assert_eq!(codepoint_width(0x00F1), 1); // ñ
    }

    #[test]
    fn zero_width_joiner() {
        assert_eq!(codepoint_width(0x200D), 0); // Zero-width joiner
    }

    #[test]
    fn cyrillic_width_1() {
        assert_eq!(codepoint_width(0x0410), 1); // Cyrillic A
        assert_eq!(codepoint_width(0x044F), 1); // Cyrillic ya
    }

    #[test]
    fn arabic_width_1() {
        assert_eq!(codepoint_width(0x0627), 1); // Arabic alef
        assert_eq!(codepoint_width(0x0628), 1); // Arabic beh
    }
}
