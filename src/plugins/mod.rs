use optimod_checker::PipelineVerdict;

use crate::events::models::ChatMessage;

pub mod database;
pub mod logger;
pub mod metrics;

/// Side-effect handler. Runs after the parent checker pipeline produces a verdict.
pub trait EventPlugin: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn on_message(&self, message: &ChatMessage, verdict: &PipelineVerdict);
}
