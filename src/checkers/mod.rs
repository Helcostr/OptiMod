mod timing;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use optimod_checker::{
    damsel_in_distress::DamselInDistressModule,
    loader::LoadedPlugin,
    small_caps_obfuscation::SmallCapsObfuscationModule,
    Action,
    CheckerModule,
    ModuleResult,
    PipelineVerdict,
};
use serde::Serialize;

pub use timing::{CheckerTimingRegistry, PluginTimingSnapshot};

use sqlx::{Pool, Postgres};

use crate::events::models::{CheckerTiming, MessageTimings};

#[derive(Debug, Clone, Serialize)]
pub struct CheckerDescriptor {
    pub name: String,
    pub kind: String,
    pub loaded: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckerStepDetail {
    pub name: String,
    pub kind: String,
    pub action: String,
    pub duration_us: u64,
    pub averages: PluginTimingSnapshot,
    pub detail: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct InspectResult {
    pub verdict: PipelineVerdict,
    pub steps: Vec<CheckerStepDetail>,
    pub total_checker_us: u64,
}

struct RegisteredChecker {
    module: Arc<dyn CheckerModule>,
    kind: &'static str,
}

/// Parent-owned checker pipeline. Child modules see UTF-8 text only.
pub struct CheckerPipeline {
    modules: Vec<RegisteredChecker>,
    timing: CheckerTimingRegistry,
}

impl CheckerPipeline {
    pub fn from_env() -> Self {
        let mut modules = Vec::new();

        let enabled = std::env::var("OPTIMOD_CHECKERS")
            .unwrap_or_else(|_| "small-caps-obfuscation,damsel-in-distress".to_string());

        for name in enabled.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match name {
                "small-caps-obfuscation" => {
                    modules.push(RegisteredChecker {
                        module: Arc::new(SmallCapsObfuscationModule::default()),
                        kind: "builtin",
                    });
                }
                "damsel-in-distress" => {
                    modules.push(RegisteredChecker {
                        module: Arc::new(DamselInDistressModule::default()),
                        kind: "builtin",
                    });
                }
                other => tracing::warn!(checker = other, "unknown built-in checker; skipping"),
            }
        }

        if modules.is_empty() {
            tracing::warn!("no built-in checkers enabled; defaulting to small-caps-obfuscation");
            modules.push(RegisteredChecker {
                module: Arc::new(SmallCapsObfuscationModule::default()),
                kind: "builtin",
            });
        }

        for path in std::env::var("OPTIMOD_PLUGIN_PATH")
            .unwrap_or_default()
            .split(';')
            .filter(|p| !p.is_empty())
            .map(PathBuf::from)
        {
            match LoadedPlugin::load(&path) {
                Ok(plugin) => {
                    tracing::info!(
                        path = %path.display(),
                        module = %plugin.name(),
                        "loaded checker plugin"
                    );
                    modules.push(RegisteredChecker {
                        module: Arc::new(plugin),
                        kind: "dll",
                    });
                }
                Err(e) => {
                    tracing::error!(path = %path.display(), error = %e, "failed to load checker plugin");
                }
            }
        }

        Self {
            modules,
            timing: CheckerTimingRegistry::new(),
        }
    }

    pub fn descriptors(&self) -> Vec<CheckerDescriptor> {
        self.modules
            .iter()
            .map(|entry| CheckerDescriptor {
                name: entry.module.name().to_string(),
                kind: entry.kind.to_string(),
                loaded: true,
            })
            .collect()
    }

    pub fn timing_snapshots(&self) -> std::collections::HashMap<String, PluginTimingSnapshot> {
        self.timing.snapshots()
    }

    pub async fn from_env_hydrated(pool: &Pool<Postgres>) -> Self {
        let pipeline = Self::from_env();
        match CheckerTimingRegistry::hydrate_from_db(pool).await {
            Ok(hydrated) => Self {
                modules: pipeline.modules,
                timing: hydrated,
            },
            Err(error) => {
                tracing::warn!(error = %error, "failed to restore checker timings from database");
                pipeline
            }
        }
    }

    pub fn inspect_timed(&self, text: &str, include_debug_detail: bool) -> InspectResult {
        let total_start = Instant::now();
        let mut steps = Vec::with_capacity(self.modules.len());
        let mut module_results = Vec::with_capacity(self.modules.len());

        for entry in &self.modules {
            let name = entry.module.name();
            let start = Instant::now();
            let action = entry.module.decide(text);
            let duration_ns = start.elapsed().as_nanos() as u64;
            let duration_us = duration_ns / 1_000;

            self.timing.record(name, duration_ns, action);

            module_results.push(ModuleResult {
                module: name.to_string(),
                action,
            });

            steps.push(CheckerStepDetail {
                name: name.to_string(),
                kind: entry.kind.to_string(),
                action: action_label(action),
                duration_us,
                averages: self.timing.snapshot(name),
                detail: if include_debug_detail {
                    entry.module.debug_detail(text)
                } else {
                    None
                },
            });
        }

        let verdict = PipelineVerdict {
            action: worst_action(module_results.iter().map(|result| result.action)),
            modules: module_results,
        };

        InspectResult {
            verdict,
            steps,
            total_checker_us: total_start.elapsed().as_micros() as u64,
        }
    }

    pub fn inspect_steps(&self, text: &str) -> Vec<CheckerStepDetail> {
        self.inspect_timed(text, true).steps
    }

    pub fn inspect(&self, text: &str) -> PipelineVerdict {
        self.inspect_timed(text, false).verdict
    }

    pub fn message_timings(
        security_duration_us: u64,
        inspect: &InspectResult,
    ) -> MessageTimings {
        MessageTimings {
            total_duration_us: security_duration_us + inspect.total_checker_us,
            security_duration_us,
            checker_duration_us: inspect.total_checker_us,
            checkers: inspect
                .steps
                .iter()
                .map(|step| CheckerTiming {
                    name: step.name.clone(),
                    duration_us: step.duration_us,
                    action: step.action.clone(),
                })
                .collect(),
        }
    }
}

impl Default for CheckerPipeline {
    fn default() -> Self {
        Self::from_env()
    }
}

fn action_label(action: Action) -> String {
    match action {
        Action::Pass => "pass".to_string(),
        Action::Flag => "flag".to_string(),
        Action::Block => "block".to_string(),
    }
}

fn worst_action(actions: impl IntoIterator<Item = Action>) -> Action {
    actions.into_iter().fold(Action::Pass, |worst, action| {
        if action_rank(action) > action_rank(worst) {
            action
        } else {
            worst
        }
    })
}

const fn action_rank(action: Action) -> u8 {
    match action {
        Action::Pass => 0,
        Action::Flag => 1,
        Action::Block => 2,
    }
}

pub fn drop_blocked_messages() -> bool {
    matches!(
        std::env::var("OPTIMOD_DROP_BLOCKED").ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("True")
    )
}
