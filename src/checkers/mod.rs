use std::sync::Arc;

use optimod_checker::{
    loader::LoadedPlugin,
    run_checkers,
    small_caps_obfuscation::SmallCapsObfuscationModule,
    CheckerModule,
    PipelineVerdict,
};

/// Parent-owned checker pipeline. Child modules see UTF-8 text only.
pub struct CheckerPipeline {
    modules: Vec<Arc<dyn CheckerModule>>,
}

impl CheckerPipeline {
    pub fn from_env() -> Self {
        let mut modules: Vec<Arc<dyn CheckerModule>> = Vec::new();

        let enabled = std::env::var("OPTIMOD_CHECKERS")
            .unwrap_or_else(|_| "small-caps-obfuscation".to_string());

        for name in enabled.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match name {
                "small-caps-obfuscation" => {
                    modules.push(Arc::new(SmallCapsObfuscationModule::default()));
                }
                other => tracing::warn!(checker = other, "unknown built-in checker; skipping"),
            }
        }

        if modules.is_empty() {
            tracing::warn!("no built-in checkers enabled; defaulting to small-caps-obfuscation");
            modules.push(Arc::new(SmallCapsObfuscationModule::default()));
        }

        for plugin in LoadedPlugin::from_env() {
            modules.push(Arc::new(plugin));
        }

        Self { modules }
    }

    pub fn inspect(&self, text: &str) -> PipelineVerdict {
        let refs = self
            .modules
            .iter()
            .map(|module| module.as_ref())
            .collect::<Vec<_>>();
        run_checkers(&refs, text)
    }
}

impl Default for CheckerPipeline {
    fn default() -> Self {
        Self::from_env()
    }
}

pub fn drop_blocked_messages() -> bool {
    matches!(
        std::env::var("OPTIMOD_DROP_BLOCKED").ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("True")
    )
}
