//! NFKC normalization — applied once before alphabet scanning.

#![forbid(unsafe_code)]

use unicode_normalization::UnicodeNormalization;

/// High bit set in each byte lane — non-ASCII if any byte in an 8-byte chunk matches.
const ASCII_HIGH_BIT_MASK: u64 = 0x8080808080808080;

/// Normalize text with Unicode NFKC (compatibility decomposition + canonical composition).
#[inline]
pub fn nfkc(input: &str) -> String {
    input.nfkc().collect()
}

/// Normalize into an existing buffer, reusing capacity when possible.
pub fn nfkc_into(input: &str, buf: &mut String) {
    buf.clear();
    for ch in input.nfkc() {
        buf.push(ch);
    }
}

/// True when every byte is ASCII (`< 0x80`). Small-caps alphabet chars are always non-ASCII.
#[inline]
pub fn is_ascii(input: &str) -> bool {
    let bytes = input.as_bytes();
    let mut i = 0;
    while i + 8 <= bytes.len() {
        let chunk = u64::from_le_bytes(bytes[i..i + 8].try_into().expect("8 bytes"));
        if chunk & ASCII_HIGH_BIT_MASK != 0 {
            return false;
        }
        i += 8;
    }
    while i < bytes.len() {
        if bytes[i] >= 128 {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_unchanged() {
        assert_eq!(nfkc("hello"), "hello");
    }

    #[test]
    fn fullwidth_digits() {
        assert_eq!(nfkc("１２３"), "123");
    }

    #[test]
    fn small_caps_not_collapsed() {
        // Custom alphabet handles these; NFKC alone does not map to ASCII.
        assert_eq!(nfkc("ᴏ"), "ᴏ");
    }

    #[test]
    fn is_ascii_detects_plain_text() {
        assert!(is_ascii("hello stream"));
        assert!(is_ascii(""));
    }

    #[test]
    fn is_ascii_rejects_small_caps() {
        assert!(!is_ascii("hellᴏ"));
        assert!(!is_ascii("ʏᴏ"));
    }
}
