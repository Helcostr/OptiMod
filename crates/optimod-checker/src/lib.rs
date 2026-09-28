//! OptiMod host-side checker pipeline.

pub use optimod_plugin_sdk::{Action, Checker as CheckerTrait, ABI_VERSION};

mod pipeline;

pub use pipeline::{run_checkers, CheckerModule, ModuleResult, PipelineVerdict};

#[cfg(feature = "damsel_in_distress")]
pub mod damsel_in_distress;

#[cfg(feature = "small_caps_obfuscation")]
pub mod small_caps_obfuscation;

#[cfg(feature = "dll")]
pub mod loader;
