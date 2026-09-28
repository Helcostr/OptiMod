use std::sync::atomic::{AtomicU64, Ordering};

use optimod_checker::PipelineVerdict;

use crate::events::models::ChatMessage;

use super::EventPlugin;

pub struct MetricsPlugin {
    pub messages_seen: AtomicU64,
    pub messages_blocked: AtomicU64,
}

impl MetricsPlugin {
    pub fn new() -> Self {
        Self {
            messages_seen: AtomicU64::new(0),
            messages_blocked: AtomicU64::new(0),
        }
    }
}

impl EventPlugin for MetricsPlugin {
    fn name(&self) -> &'static str {
        "metrics"
    }

    fn on_message(&self, _message: &ChatMessage, verdict: &PipelineVerdict) {
        self.messages_seen.fetch_add(1, Ordering::Relaxed);
        if !verdict.is_good() {
            self.messages_blocked.fetch_add(1, Ordering::Relaxed);
        }
    }
}
