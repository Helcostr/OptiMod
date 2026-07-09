use tracing::info;

use crate::events::models::ChatMessage;

use super::Plugin;

pub struct LoggerPlugin;

impl Plugin for LoggerPlugin {
    fn name(&self) -> &'static str {
        "logger"
    }

    fn on_message(&self, message: &ChatMessage) {
        info!(
            channel = %message.channel_login,
            user = %message.user_login,
            message = %message.raw_message,
            flags = ?message.security_flags,
            "chat message received"
        );
    }
}
