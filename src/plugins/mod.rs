use crate::events::models::ChatMessage;

pub mod logger;
pub mod metrics;

pub trait Plugin: Send + Sync {
    fn name(&self) -> &'static str;
    fn on_message(&self, message: &ChatMessage);
}
