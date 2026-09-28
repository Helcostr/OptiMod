//! Module checker contract: **one UTF-8 message in → structured result out**.
//!
//! [`Action`] is the tiered outcome (`Pass`, `Flag`, `Block`). [`CheckVerdict`] is the
//! boolean host view (`is_good`). Use `Action::is_good()` or `CheckVerdict::from_bool()` to
//! convert between them.

#![forbid(unsafe_code)]

/// Stable ABI version for plugin hosts. Bump when C exports change.
pub const ABI_VERSION: u32 = 1;

/// OptiMod module id returned by `optimod_name()` for this plugin.
pub const PLUGIN_MODULE_ID: &str = "small-caps-obfuscation";

/// Shared fixture id under `fixtures/` (used by cm-english tests, fixture-runner, and CI).
pub const TWITCH_SMALL_CAPS_SPAM: &str = "twitch_small_caps_spam";

/// Outcome tier for checkers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// Message is clearly acceptable.
    Pass,
    /// Suspicious but still treated as good for hosts.
    Flag,
    /// Message should be blocked.
    Block,
}

impl Action {
    /// `true` when the message is acceptable (`Pass` or `Flag`).
    #[inline]
    pub const fn is_good(self) -> bool {
        matches!(self, Action::Pass | Action::Flag)
    }
}

/// Public checker outcome for hosts that only need a bool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CheckVerdict {
    /// `true` = acceptable message, `false` = block.
    pub is_good: bool,
}

impl CheckVerdict {
    #[inline]
    pub const fn good() -> Self {
        Self { is_good: true }
    }

    #[inline]
    pub const fn bad() -> Self {
        Self { is_good: false }
    }

    #[inline]
    pub const fn from_bool(is_good: bool) -> Self {
        Self { is_good }
    }
}

impl From<bool> for CheckVerdict {
    fn from(is_good: bool) -> Self {
        Self { is_good }
    }
}

impl From<CheckVerdict> for bool {
    fn from(v: CheckVerdict) -> bool {
        v.is_good
    }
}

/// One module: UTF-8 text in, structured verdict out. No chat metadata.
pub trait Checker: Send + Sync {
    /// Stable module id (e.g. `"english"`).
    fn name(&self) -> &'static str;

    /// Fast good/bad path for hot callers.
    fn is_good(&self, input: &str) -> bool;

    /// Full verdict; default delegates to [`Checker::is_good`].
    fn check(&self, input: &str) -> CheckVerdict {
        CheckVerdict::from_bool(self.is_good(input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_is_good() {
        assert!(Action::Pass.is_good());
        assert!(Action::Flag.is_good());
        assert!(!Action::Block.is_good());
    }

    #[test]
    fn check_verdict_helpers() {
        assert!(CheckVerdict::good().is_good);
        assert!(!CheckVerdict::bad().is_good);
        assert!(CheckVerdict::from_bool(true).is_good);
        assert!(!CheckVerdict::from_bool(false).is_good);
        assert_eq!(bool::from(CheckVerdict::good()), true);
        assert_eq!(bool::from(CheckVerdict::bad()), false);
    }
}
