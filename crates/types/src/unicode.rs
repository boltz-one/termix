// Unicode utilities and UTF-8/16 conversion
// Port: til/ut_til/u8u16convertTests.cpp, types/convert.cpp

use std::char;

/// Convert UTF-8 bytes to UTF-16 code units
pub fn utf8_to_utf16(utf8: &[u8]) -> Vec<u16> {
    String::from_utf8_lossy(utf8)
        .encode_utf16()
        .collect()
}

/// Convert UTF-16 code units to UTF-8 bytes
pub fn utf16_to_utf8(utf16: &[u16]) -> Vec<u8> {
    String::from_utf16_lossy(utf16)
        .into_bytes()
}

/// Decode next UTF-8 character, return (char, bytes_consumed)
pub fn decode_utf8(bytes: &[u8]) -> Option<(char, usize)> {
    if bytes.is_empty() {
        return None;
    }

    let first = bytes[0];

    // ASCII fast path
    if first < 0x80 {
        return Some((first as char, 1));
    }

    // Multi-byte UTF-8
    let len = if first & 0xE0 == 0xC0 {
        2
    } else if first & 0xF0 == 0xE0 {
        3
    } else if first & 0xF8 == 0xF0 {
        4
    } else {
        return None; // Invalid
    };

    if bytes.len() < len {
        return None; // Incomplete
    }

    std::str::from_utf8(&bytes[..len])
        .ok()
        .and_then(|s| s.chars().next())
        .map(|c| (c, len))
}

/// Check if byte is UTF-8 continuation byte
pub fn is_utf8_continuation(byte: u8) -> bool {
    byte & 0xC0 == 0x80
}

/// Count UTF-8 characters in byte slice
pub fn utf8_char_count(bytes: &[u8]) -> usize {
    String::from_utf8_lossy(bytes).chars().count()
}

/// Split string at grapheme cluster boundaries using unicode-segmentation
pub fn grapheme_clusters(s: &str) -> Vec<&str> {
    use unicode_segmentation::UnicodeSegmentation;
    s.graphemes(true).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_to_utf16_ascii() {
        let utf8 = b"Hello";
        let utf16 = utf8_to_utf16(utf8);
        assert_eq!(utf16, vec![b'H' as u16, b'e' as u16, b'l' as u16, b'l' as u16, b'o' as u16]);
    }

    #[test]
    fn utf8_to_utf16_multibyte() {
        let utf8 = "こんにちは".as_bytes();
        let utf16 = utf8_to_utf16(utf8);
        assert_eq!(utf16.len(), 5);
    }

    #[test]
    fn utf16_to_utf8_ascii() {
        let utf16 = vec![b'H' as u16, b'i' as u16];
        let utf8 = utf16_to_utf8(&utf16);
        assert_eq!(utf8, b"Hi");
    }

    #[test]
    fn utf16_to_utf8_roundtrip() {
        let original = "Hello 世界 🚀";
        let utf16 = original.encode_utf16().collect::<Vec<_>>();
        let utf8 = utf16_to_utf8(&utf16);
        assert_eq!(String::from_utf8_lossy(&utf8), original);
    }

    #[test]
    fn decode_utf8_ascii() {
        let bytes = b"A";
        let (ch, len) = decode_utf8(bytes).unwrap();
        assert_eq!(ch, 'A');
        assert_eq!(len, 1);
    }

    #[test]
    fn decode_utf8_multibyte() {
        let bytes = "世".as_bytes();
        let (ch, len) = decode_utf8(bytes).unwrap();
        assert_eq!(ch, '世');
        assert_eq!(len, 3);
    }

    #[test]
    fn decode_utf8_incomplete() {
        let bytes = &[0xE4, 0xB8]; // Incomplete "世"
        assert_eq!(decode_utf8(bytes), None);
    }

    #[test]
    fn is_continuation_byte() {
        assert!(!is_utf8_continuation(b'A'));
        assert!(!is_utf8_continuation(0xE4));
        assert!(is_utf8_continuation(0xB8));
        assert!(is_utf8_continuation(0x96));
    }

    #[test]
    fn utf8_char_count_ascii() {
        assert_eq!(utf8_char_count(b"Hello"), 5);
    }

    #[test]
    fn utf8_char_count_mixed() {
        assert_eq!(utf8_char_count("Hello世界".as_bytes()), 7);
    }

    #[test]
    fn grapheme_clusters_ascii() {
        let clusters = grapheme_clusters("Hello");
        assert_eq!(clusters, vec!["H", "e", "l", "l", "o"]);
    }

    #[test]
    fn grapheme_clusters_combining() {
        let clusters = grapheme_clusters("é"); // e + combining acute
        assert_eq!(clusters.len(), 1); // Single grapheme cluster
    }

    #[test]
    fn grapheme_clusters_emoji() {
        let clusters = grapheme_clusters("👨‍👩‍👧‍👦"); // Family emoji
        assert_eq!(clusters.len(), 1); // Single grapheme cluster (ZWJ sequence)
    }
}
