use futures_util::{SinkExt, StreamExt};
use std::time::Instant;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use tracing::{error, info, warn};

use crate::{
    chat::http::{fetch_token_owner_id, fetch_user_id, subscribe_to_chat},
    checkers::{drop_blocked_messages, CheckerPipeline},
    config::app_config::TwitchConfig,
    core::EventSender,
    events::models::{Badge, ChatMessage},
    plugins::EventPlugin,
    security::pipeline::inspect_message,
};

use super::models::{TwitchChatMessage, TwitchWelcomeMessage};

pub async fn connect_and_listen(
    config: TwitchConfig,
    tx: EventSender,
    checkers: std::sync::Arc<CheckerPipeline>,
    plugins: std::sync::Arc<Vec<std::sync::Arc<dyn EventPlugin>>>,
) -> anyhow::Result<()> {
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
                        if let Err(e) = subscribe_to_chat(
                            &config.client_id,
                            &config.user_token,
                            &welcome.payload.session.id,
                            &broadcaster_id,
                            &token_owner_id,
                        )
                        .await
                        {
                            error!("Failed to subscribe to chat: {}", e);
                        }
                    }
                } else if let Ok(chat) = serde_json::from_str::<TwitchChatMessage>(&text) {
                    if chat.metadata.message_type == "notification" {
                        let ev = chat.payload.event;
                        let raw_message = ev.message.text;
                        let security_start = Instant::now();
                        let security = inspect_message(&raw_message);
                        let security_duration_us = security_start.elapsed().as_micros() as u64;
                        let inspect = checkers.inspect_timed(&raw_message, false);
                        let timings =
                            CheckerPipeline::message_timings(security_duration_us, &inspect);
                        let verdict = inspect.verdict;

                        let mut flags = security.flags;
                        flags.extend(verdict.security_flags());

                        let badges = ev
                            .badges
                            .into_iter()
                            .map(|b| Badge { set_id: b.set_id, id: b.id })
                            .collect();
                        let msg_event = ChatMessage::new(
                            ev.message_id,
                            ev.broadcaster_user_id,
                            ev.broadcaster_user_login,
                            ev.chatter_user_id,
                            ev.chatter_user_login,
                            ev.chatter_user_name,
                            badges,
                            ev.color,
                            raw_message,
                            security.normalized_message,
                            flags,
                            Some(timings),
                        );

                        for plugin in plugins.iter() {
                            plugin.on_message(&msg_event, &verdict);
                        }

                        if verdict.is_good() || !drop_blocked_messages() {
                            let _ = tx.send(msg_event);
                        }
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
