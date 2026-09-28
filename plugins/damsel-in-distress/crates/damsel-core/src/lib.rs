//! Detects "damsel in distress" spam: flattery + Discord invite patterns.

#![forbid(unsafe_code)]

use cm_core::Action;
use cm_normalize::nfkc;

/// Stable module id for hosts and debug output.
pub const PLUGIN_MODULE_ID: &str = "damsel-in-distress";

/// Shared fixture id under `fixtures/`.
pub const DAMSEL_SPAM_FIXTURE: &str = "damsel_spam";

const FLATTERY_PHRASES: &[&str] = &[
    "love your stream",
    "love your content",
    "love the stream",
    "really love",
    "big fan",
    "fan community",
    "enjoy being part",
    "being part of your",
    "enjoy your stream",
    "great stream",
    "amazing stream",
    "awesome stream",
    "support your stream",
    "love what you do",
];

const DISCORD_INVITE_PHRASES: &[&str] = &[
    "add me on discord",
    "add me up on discord",
    "adding me on discord",
    "adding me up on discord",
    "hit me up on discord",
    "join my discord",
    "join me on discord",
    "my discord",
    "on discord",
];

/// Outcome of evaluating one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub action: Action,
    pub has_flattery: bool,
    pub has_discord_invite: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DamselChecker;

impl DamselChecker {
    pub fn evaluate(&self, text: &str) -> Evaluation {
        let normalized = normalize_for_matching(text);
        let has_flattery = has_flattery_signal(&normalized);
        let has_discord_invite = has_discord_signal(&normalized);
        let pattern_match = has_flattery && has_discord_invite;

        Evaluation {
            action: if pattern_match {
                Action::Block
            } else {
                Action::Pass
            },
            has_flattery,
            has_discord_invite,
        }
    }

    pub fn decide(&self, text: &str) -> Action {
        self.evaluate(text).action
    }
}

fn normalize_for_matching(text: &str) -> String {
    let nfkc = nfkc(text);
    let leet = leet_to_ascii(&nfkc);
    leet.to_ascii_lowercase()
}

fn leet_to_ascii(input: &str) -> String {
    input
        .chars()
        .map(|c| match c {
            '0' | '○' | 'ø' => 'o',
            '1' | '!' | '|' | '¡' | 'ℓ' => 'i',
            '3' => 'e',
            '4' | '@' => 'a',
            '5' | '$' => 's',
            '7' => 't',
            c => c,
        })
        .collect()
}

fn has_flattery_signal(normalized: &str) -> bool {
    FLATTERY_PHRASES
        .iter()
        .any(|phrase| normalized.contains(phrase))
}

fn has_discord_signal(normalized: &str) -> bool {
    if normalized.contains("discord.gg") || normalized.contains("discord.com/invite") {
        return true;
    }

    if normalized.contains("discord") {
        return true;
    }

    DISCORD_INVITE_PHRASES
        .iter()
        .any(|phrase| normalized.contains(phrase))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};

    fn fixture_text(name: &str) -> String {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name)
            .join("input.txt");
        fs::read_to_string(path).expect("fixture should exist")
    }

    #[test]
    fn blocks_damsel_spam() {
        let text = fixture_text(DAMSEL_SPAM_FIXTURE);
        let result = DamselChecker.evaluate(&text);
        assert_eq!(result.action, Action::Block);
        assert!(result.has_flattery);
        assert!(result.has_discord_invite);
    }

    #[test]
    fn passes_regular_discord_mention_without_flattery() {
        let text = "anyone know the discord link for this game?";
        let result = DamselChecker.evaluate(text);
        assert_eq!(result.action, Action::Pass);
        assert!(!result.has_flattery);
        assert!(result.has_discord_invite);
    }

    #[test]
    fn passes_regular_compliment_without_discord() {
        let text = "hey, really love your stream, keep it up!";
        let result = DamselChecker.evaluate(text);
        assert_eq!(result.action, Action::Pass);
        assert!(result.has_flattery);
        assert!(!result.has_discord_invite);
    }

    #[test]
    fn detects_leet_discord_spelling() {
        let normalized = normalize_for_matching("add me up on d1scord");
        assert!(has_discord_signal(&normalized));
    }
}
