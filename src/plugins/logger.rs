use optimod_checker::PipelineVerdict;
use tracing::info;

use crate::events::models::ChatMessage;

use super::EventPlugin;

pub struct LoggerPlugin;

impl EventPlugin for LoggerPlugin {
    fn name(&self) -> &'static str {
        "logger"
    }

    fn on_message(&self, message: &ChatMessage, verdict: &PipelineVerdict) {
        info!(
            channel = %message.channel_login,
            user = %message.user_login,
            message = %message.raw_message,
            flags = ?message.security_flags,
            checker_action = ?verdict.action,
            checker_modules = ?verdict.modules,
            "chat message received"
        );
    }
}
