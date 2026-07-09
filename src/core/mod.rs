use tokio::sync::broadcast;
use crate::events::models::ChatMessage;

/// Convenience type alias for the event sender.
pub type EventSender = broadcast::Sender<ChatMessage>;

/// Create a new broadcast channel for internal events.
/// Capacity of 256 is more than enough for chat bursts.
pub fn create_channel() -> (broadcast::Sender<ChatMessage>, broadcast::Receiver<ChatMessage>) {
    broadcast::channel(256)
}
