// Utilities - GUID parsing, console helpers
// Port: types/inc/utils.hpp, types/utils.cpp

use std::fmt;

/// GUID/UUID representation
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Guid {
    data: [u8; 16],
}

impl Guid {
    /// Create from raw bytes (little-endian format)
    pub const fn from_bytes(data: [u8; 16]) -> Self {
        Self { data }
    }

    /// Parse from string (formats: {xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx} or xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx)
    pub fn parse(s: &str) -> Result<Self, &'static str> {
        let s = s.trim();
        let s = if s.starts_with('{') && s.ends_with('}') {
            &s[1..s.len() - 1]
        } else {
            s
        };

        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 5 {
            return Err("Invalid GUID format");
        }

        let mut data = [0u8; 16];

        // Parse each part as hex
        let parse_hex = |s: &str| -> Result<Vec<u8>, &'static str> {
            (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| "Invalid hex"))
                .collect()
        };

        let d1 = parse_hex(parts[0])?;
        let d2 = parse_hex(parts[1])?;
        let d3 = parse_hex(parts[2])?;
        let d4 = parse_hex(parts[3])?;
        let d5 = parse_hex(parts[4])?;

        if d1.len() != 4 || d2.len() != 2 || d3.len() != 2 || d4.len() != 2 || d5.len() != 6 {
            return Err("Invalid GUID part length");
        }

        data[0..4].copy_from_slice(&d1);
        data[4..6].copy_from_slice(&d2);
        data[6..8].copy_from_slice(&d3);
        data[8..10].copy_from_slice(&d4);
        data[10..16].copy_from_slice(&d5);

        Ok(Self { data })
    }

    /// Convert to bytes
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.data
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            self.data[0], self.data[1], self.data[2], self.data[3],
            self.data[4], self.data[5],
            self.data[6], self.data[7],
            self.data[8], self.data[9],
            self.data[10], self.data[11], self.data[12], self.data[13], self.data[14], self.data[15]
        )
    }
}

/// Console utility functions
pub mod console {
    /// Check if a character is printable (for console display)
    pub fn is_printable(ch: char) -> bool {
        !ch.is_control() && ch != '\x7F'
    }

    /// Replace control characters with printable representations
    pub fn sanitize_control_chars(s: &str) -> String {
        s.chars()
            .map(|ch| if is_printable(ch) { ch } else { '�' })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guid_parse_with_braces() {
        let guid = Guid::parse("{12345678-90AB-CDEF-1234-567890ABCDEF}").unwrap();
        assert_eq!(guid.data[0], 0x12);
        assert_eq!(guid.data[1], 0x34);
        assert_eq!(guid.data[15], 0xEF);
    }

    #[test]
    fn guid_parse_without_braces() {
        let guid = Guid::parse("12345678-90AB-CDEF-1234-567890ABCDEF").unwrap();
        assert_eq!(guid.data[0], 0x12);
    }

    #[test]
    fn guid_parse_lowercase() {
        let guid = Guid::parse("abcdef01-2345-6789-abcd-ef0123456789").unwrap();
        assert_eq!(guid.data[0], 0xAB);
        assert_eq!(guid.data[1], 0xCD);
    }

    #[test]
    fn guid_parse_invalid_format() {
        assert!(Guid::parse("invalid").is_err());
        assert!(Guid::parse("12345678-90AB-CDEF").is_err());
    }

    #[test]
    fn guid_display() {
        let data = [
            0x12, 0x34, 0x56, 0x78,
            0x90, 0xAB,
            0xCD, 0xEF,
            0x12, 0x34,
            0x56, 0x78, 0x90, 0xAB, 0xCD, 0xEF,
        ];
        let guid = Guid::from_bytes(data);
        assert_eq!(guid.to_string(), "12345678-90ab-cdef-1234-567890abcdef");
    }

    #[test]
    fn guid_roundtrip() {
        let original = "12345678-90AB-CDEF-1234-567890ABCDEF";
        let guid = Guid::parse(original).unwrap();
        assert_eq!(guid.to_string().to_uppercase(), original);
    }

    #[test]
    fn console_is_printable() {
        assert!(console::is_printable('A'));
        assert!(console::is_printable('世'));
        assert!(!console::is_printable('\x00'));
        assert!(!console::is_printable('\x1B'));
        assert!(!console::is_printable('\x7F'));
    }

    #[test]
    fn console_sanitize() {
        let input = "Hello\x00World\x1B[31m";
        let output = console::sanitize_control_chars(input);
        assert_eq!(output, "Hello�World�[31m");
    }
}
