use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub message_id: String,
    pub timestamp_ms: u128,
    pub channel_id: String,
    pub channel_login: String,
    pub user_id: String,
    pub user_login: String,
    pub user_name: String,
    pub badges: Vec<Badge>,
    pub color: String,
    pub raw_message: String,
    pub normalized_message: Option<String>,
    pub security_flags: Vec<String>,
}

impl ChatMessage {
    pub fn new(
        message_id: String,
        channel_id: String,
        channel_login: String,
        user_id: String,
        user_login: String,
        user_name: String,
        badges: Vec<Badge>,
        color: String,
        raw_message: String,
    ) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        Self {
            message_id,
            timestamp_ms,
            channel_id,
            channel_login,
            user_id,
            user_login,
            user_name,
            badges,
            color,
            raw_message,
            normalized_message: None,
            security_flags: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Badge {
    pub set_id: String,
    pub id: String,
}
