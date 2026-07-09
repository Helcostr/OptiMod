use crate::events::models::ChatMessage;

pub mod database;
pub mod logger;
pub mod metrics;

pub trait Plugin: Send + Sync {
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
    fn on_message(&self, message: &ChatMessage);
}
