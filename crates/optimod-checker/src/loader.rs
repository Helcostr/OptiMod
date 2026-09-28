//! Runtime loader for OptiMod plugin DLLs implementing `optimod_plugin.h`.

use std::ffi::CStr;
use std::path::{Path, PathBuf};

use libloading::Library;
use optimod_plugin_sdk::{Action, CheckError, ABI_VERSION};
use tracing::{error, warn};

use super::CheckerModule;

type AbiVersionFn = unsafe extern "C" fn() -> u32;
type NameFn = unsafe extern "C" fn() -> *const i8;
type CheckFn = unsafe extern "C" fn(*const u8, usize) -> i32;

pub struct LoadedPlugin {
    _lib: Library,
    name: String,
    check_fn: CheckFn,
}

impl LoadedPlugin {
    pub fn load(path: &Path) -> Result<Self, String> {
        let lib = unsafe { Library::new(path) }.map_err(|e| format!("load {}: {}", path.display(), e))?;

        let abi_version: libloading::Symbol<AbiVersionFn> = unsafe {
            lib.get(b"optimod_abi_version\0")
                .map_err(|e| format!("missing optimod_abi_version in {}: {}", path.display(), e))?
        };
        let plugin_abi = unsafe { abi_version() };
        if plugin_abi != ABI_VERSION {
            return Err(format!(
                "ABI mismatch in {}: plugin={} host={}",
                path.display(),
                plugin_abi,
                ABI_VERSION
            ));
        }

        let name_fn: libloading::Symbol<NameFn> = unsafe {
            lib.get(b"optimod_name\0")
                .map_err(|e| format!("missing optimod_name in {}: {}", path.display(), e))?
        };
        let name_ptr = unsafe { name_fn() };
        if name_ptr.is_null() {
            return Err(format!("optimod_name returned null in {}", path.display()));
        }
        let name = unsafe { CStr::from_ptr(name_ptr) }
            .to_str()
            .map_err(|e| format!("invalid optimod_name UTF-8 in {}: {}", path.display(), e))?
            .to_string();

        let check_fn: libloading::Symbol<CheckFn> = unsafe {
            lib.get(b"optimod_check\0")
                .map_err(|e| format!("missing optimod_check in {}: {}", path.display(), e))?
        };
        let check_fn = *check_fn;

        Ok(Self {
            _lib: lib,
            name,
            check_fn,
        })
    }

    pub fn from_env() -> Vec<Self> {
        let paths = std::env::var("OPTIMOD_PLUGIN_PATH")
            .unwrap_or_default()
            .split(';')
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
            .collect::<Vec<_>>();

        paths
            .iter()
            .filter_map(|path| match Self::load(path) {
                Ok(plugin) => {
                    tracing::info!(path = %path.display(), module = %plugin.name, "loaded checker plugin");
                    Some(plugin)
                }
                Err(e) => {
                    error!(path = %path.display(), error = %e, "failed to load checker plugin");
                    None
                }
            })
            .collect()
    }
}

impl CheckerModule for LoadedPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn decide(&self, text: &str) -> Action {
        let code = unsafe { (self.check_fn)(text.as_ptr(), text.len()) };
        match Action::from_check_code(code) {
            Ok(action) => action,
            Err(CheckError::InvalidUtf8) => {
                warn!(module = %self.name, "plugin reported invalid UTF-8; treating as pass");
                Action::Pass
            }
            Err(e) => {
                warn!(module = %self.name, error = ?e, "plugin check error; treating as pass");
                Action::Pass
            }
        }
    }
}
