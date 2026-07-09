use unicode_normalization::UnicodeNormalization;

use super::report::SecurityReport;

pub fn inspect_message(message: &str) -> SecurityReport {
    let normalized: String = message.nfc().collect();
    let mut flags = Vec::new();

    if normalized != message {
        flags.push("unicode_normalized".to_string());
    }

    let skeleton = unicode_security::skeleton(message);
    if skeleton != normalized {
        flags.push("confusable_chars_detected".to_string());
    }

    if normalized.chars().any(|c| c.is_control() && !c.is_whitespace()) {
        flags.push("control_characters_detected".to_string());
    }

    SecurityReport {
        normalized_message: if normalized == message { None } else { Some(normalized) },
        flags,
    }
}
