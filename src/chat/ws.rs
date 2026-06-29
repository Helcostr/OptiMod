use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{info, warn, error};

use crate::config::app_config::TwitchConfig;
use crate::chat::http::{fetch_user_id, fetch_token_owner_id, subscribe_to_chat};

use super::models::{TwitchWelcomeMessage, TwitchChatMessage};

pub async fn connect_and_listen(config: TwitchConfig) -> anyhow::Result<()> {
    info!("Fetching Broadcaster User ID for channel: {}", config.channel);
    let broadcaster_id = fetch_user_id(&config.client_id, &config.user_token, &config.channel).await?;
    info!("Broadcaster ID: {}", broadcaster_id);

    info!("Fetching token owner ID...");
    let token_owner_id = fetch_token_owner_id(&config.client_id, &config.user_token).await?;

    let url = "wss://eventsub.wss.twitch.tv/ws";
    
    let (ws_stream, _) = connect_async(url).await?;
    info!("Connected to Twitch EventSub WebSocket");

    let (mut write, mut read) = ws_stream.split();

    while let Some(msg) = read.next().await {
        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                error!("WebSocket error: {}", e);
                break;
            }
        };

        match msg {
            Message::Text(text) => {
                if let Ok(welcome) = serde_json::from_str::<TwitchWelcomeMessage>(&text) {
                    if welcome.metadata.message_type == "session_welcome" {
                        info!("Received session_welcome. Session ID: {}", welcome.payload.session.id);
                        let sub_res = subscribe_to_chat(
                            &config.client_id,
                            &config.user_token,
                            &welcome.payload.session.id,
                            &broadcaster_id,
                            &token_owner_id,
                        ).await;
                        
                        if let Err(e) = sub_res {
                            error!("Failed to subscribe to chat: {}", e);
                        }
                    }
                } else if let Ok(chat) = serde_json::from_str::<TwitchChatMessage>(&text) {
                    if chat.metadata.message_type == "notification" {
                        info!("Chat message from {}: {}", chat.payload.event.chatter_user_name, chat.payload.event.message.text);
                        // TODO: Map to Internal Event and pipe to plugins
                    }
                }
            }
            Message::Ping(p) => {
                let _ = write.send(Message::Pong(p)).await;
            }
            Message::Close(_) => {
                warn!("WebSocket closed by server.");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
