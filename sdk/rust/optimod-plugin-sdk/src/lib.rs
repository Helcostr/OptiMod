//! OptiMod content-checker plugin contract (Rust).
//!
//! Plugin authors: implement the C ABI in [`sdk/include/optimod_plugin.h`] or use this
//! crate's types when writing Rust `cdylib` plugins.

#![forbid(unsafe_code)]

/// Must match [`OPTIMOD_PLUGIN_ABI_VERSION`] in the C header and host loader.
pub const ABI_VERSION: u32 = 1;

/// Tiered checker outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Pass,
    Flag,
    Block,
}

impl Action {
    #[inline]
    pub const fn is_good(self) -> bool {
        matches!(self, Action::Pass | Action::Flag)
    }

    /// Encode for the C ABI [`optimod_check`] return value.
    #[inline]
    pub const fn to_check_code(self) -> i32 {
        match self {
            Action::Pass => crate::check_code::PASS,
            Action::Flag => crate::check_code::FLAG,
            Action::Block => crate::check_code::BLOCK,
        }
    }

    /// Decode a C ABI return value from a plugin.
    pub fn from_check_code(code: i32) -> Result<Self, CheckError> {
        match code {
            check_code::PASS => Ok(Action::Pass),
            check_code::FLAG => Ok(Action::Flag),
            check_code::BLOCK => Ok(Action::Block),
            check_code::ERR_NULL => Err(CheckError::NullInput),
            check_code::ERR_UTF8 => Err(CheckError::InvalidUtf8),
            check_code::ERR_ABI => Err(CheckError::AbiMismatch),
            other => Err(CheckError::UnknownCode(other)),
        }
    }
}

/// C ABI return values — mirrors `optimod_plugin.h`.
pub mod check_code {
    pub const PASS: i32 = 1;
    pub const FLAG: i32 = 2;
    pub const BLOCK: i32 = 0;
    pub const ERR_NULL: i32 = -1;
    pub const ERR_UTF8: i32 = -2;
    pub const ERR_ABI: i32 = -3;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckError {
    NullInput,
    InvalidUtf8,
    AbiMismatch,
    UnknownCode(i32),
}

/// In-process checker trait (Rust plugins inside the workspace).
pub trait Checker: Send + Sync {
    fn name(&self) -> &'static str;
    fn decide(&self, input: &str) -> Action;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_codes() {
        for action in [Action::Pass, Action::Flag, Action::Block] {
            assert_eq!(Action::from_check_code(action.to_check_code()), Ok(action));
        }
    }
}
