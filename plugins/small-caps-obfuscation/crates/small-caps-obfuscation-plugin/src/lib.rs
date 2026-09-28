//! OptiMod plugin: small-caps Unicode obfuscation (English dictionary today).
//!
//! Exports the C ABI from `sdk/include/optimod_plugin.h`.
//!
//! Build: `cargo build -p small-caps-obfuscation-plugin --release`
//! → `target/release/optimod_small_caps_obfuscation.dll` (Windows)

use cm_core::Action;
use cm_english::{EnglishChecker, EnglishConfig};
use optimod_plugin_sdk::{self as sdk, check_code, ABI_VERSION};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::slice;
use std::sync::OnceLock;

static CHECKER: OnceLock<EnglishChecker> = OnceLock::new();

fn apply_env_f32(
    mut config: EnglishConfig,
    key: &str,
    apply: impl Fn(EnglishConfig, f32) -> EnglishConfig,
) -> EnglishConfig {
    if let Ok(raw) = std::env::var(key) {
        if let Ok(value) = raw.parse::<f32>() {
            config = apply(config, value);
        }
    }
    config
}

fn checker_config_from_env() -> EnglishConfig {
    let mut config = EnglishConfig::default();

    #[cfg(not(test))]
    {
        config = apply_env_f32(config, "CM_BLOCK_OBFUSCATION_THRESHOLD", |c, v| {
            c.with_block_obfuscation_threshold(v)
        });
        if std::env::var("CM_BLOCK_OBFUSCATION_THRESHOLD").is_err() {
            config = apply_env_f32(config, "CM_BLOCK_THRESHOLD", |c, v| {
                c.with_block_obfuscation_threshold(v)
            });
        }

        config = apply_env_f32(config, "CM_BLOCK_ENGLISH_RATIO", |c, v| {
            c.with_block_english_ratio(v)
        });

        if let Ok(force) = std::env::var("CM_FORCE_IS_GOOD") {
            let is_good = matches!(force.as_str(), "1" | "true" | "TRUE" | "True");
            config = config.with_force_action(if is_good { Action::Pass } else { Action::Block });
        }
    }

    config
}

fn checker() -> &'static EnglishChecker {
    CHECKER.get_or_init(|| EnglishChecker::new(checker_config_from_env()))
}

fn decide_bytes(bytes: &[u8]) -> i32 {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return check_code::ERR_UTF8;
    };
    cm_core_action(checker().decide(text)).to_check_code()
}

fn cm_core_action(action: Action) -> sdk::Action {
    match action {
        Action::Pass => sdk::Action::Pass,
        Action::Flag => sdk::Action::Flag,
        Action::Block => sdk::Action::Block,
    }
}

#[no_mangle]
pub extern "C" fn optimod_abi_version() -> u32 {
    ABI_VERSION
}

#[no_mangle]
pub extern "C" fn optimod_name() -> *const c_char {
    c"small-caps-obfuscation".as_ptr()
}

#[no_mangle]
pub extern "C" fn optimod_check(input: *const u8, len: usize) -> i32 {
    if input.is_null() && len != 0 {
        return check_code::ERR_NULL;
    }
    let bytes = if len == 0 {
        &[][..]
    } else {
        unsafe { slice::from_raw_parts(input, len) }
    };
    decide_bytes(bytes)
}

#[no_mangle]
pub unsafe extern "C" fn optimod_check_cstr(input: *const c_char) -> i32 {
    if input.is_null() {
        return check_code::ERR_NULL;
    }
    let cstr = unsafe { CStr::from_ptr(input) };
    let Ok(text) = cstr.to_str() else {
        return check_code::ERR_UTF8;
    };
    cm_core_action(checker().decide(text)).to_check_code()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cm_core::{Checker, PLUGIN_MODULE_ID};

    #[test]
    fn ffi_good() {
        let msg = b"hello";
        assert_eq!(optimod_check(msg.as_ptr(), msg.len()), check_code::PASS);
    }

    #[test]
    fn ffi_bad() {
        let msg = "ʏᴏ ʙʀᴏ ᴊᴜꜱᴛ ᴡᴀɴᴛᴇᴅ".as_bytes();
        assert_eq!(optimod_check(msg.as_ptr(), msg.len()), check_code::BLOCK);
    }

    #[test]
    fn ffi_name_matches_module() {
        let name = unsafe { CStr::from_ptr(optimod_name()) };
        assert_eq!(name.to_str().unwrap(), PLUGIN_MODULE_ID);
        assert_eq!(checker().name(), PLUGIN_MODULE_ID);
    }
}
