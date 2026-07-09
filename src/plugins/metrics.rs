use std::sync::atomic::{AtomicU64, Ordering};

use crate::events::models::ChatMessage;

use super::Plugin;

pub struct MetricsPlugin {
    pub messages_seen: AtomicU64,
}

impl MetricsPlugin {
    pub fn new() -> Self {
        Self { messages_seen: AtomicU64::new(0) }
    }
}

impl Plugin for MetricsPlugin {
    fn name(&self) -> &'static str {
        "metrics"
    }

    fn on_message(&self, _message: &ChatMessage) {
        self.messages_seen.fetch_add(1, Ordering::Relaxed);
    }
}
