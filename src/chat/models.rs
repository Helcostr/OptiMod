use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TwitchWelcomeMessage {
    pub metadata: WelcomeMetadata,
    pub payload: WelcomePayload,
}

#[derive(Debug, Deserialize)]
pub struct WelcomeMetadata {
    pub message_type: String,
}

#[derive(Debug, Deserialize)]
pub struct WelcomePayload {
    pub session: SessionData,
}

#[derive(Debug, Deserialize)]
pub struct SessionData {
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub struct TwitchChatMessage {
    pub metadata: WelcomeMetadata,
    pub payload: ChatPayload,
}

#[derive(Debug, Deserialize)]
pub struct ChatPayload {
    pub event: ChatEvent,
}

#[derive(Debug, Deserialize)]
pub struct ChatEvent {
    pub message_id: String,
    pub broadcaster_user_id: String,
    pub broadcaster_user_login: String,
    pub chatter_user_id: String,
    pub chatter_user_login: String,
    pub chatter_user_name: String,
    pub message: MessageContent,
    pub color: String,
    pub badges: Vec<Badge>,
}

#[derive(Debug, Deserialize)]
pub struct MessageContent {
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct Badge {
    pub set_id: String,
    pub id: String,
}
