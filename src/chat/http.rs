use reqwest::Client;
use serde_json::json;
use serde::Deserialize;
use tracing::{info, error};

#[derive(Deserialize)]
struct UsersResponse {
    data: Vec<User>,
}

#[derive(Deserialize)]
struct User {
    id: String,
}

/// Fetch the Twitch user ID for a given login name (e.g. the channel to monitor).
pub async fn fetch_user_id(client_id: &str, user_token: &str, login: &str) -> anyhow::Result<String> {
    let client = Client::new();
    let url = format!("https://api.twitch.tv/helix/users?login={}", login);
    
    let res = client
        .get(&url)
        .header("Client-Id", client_id)
        .header("Authorization", format!("Bearer {}", user_token))
        .send()
        .await?;

    if !res.status().is_success() {
        let err_text = res.text().await?;
        error!("Twitch API error: {}", err_text);
        return Err(anyhow::anyhow!("Twitch API returned an error"));
    }

    let json: UsersResponse = res.json().await?;
    if let Some(user) = json.data.into_iter().next() {
        Ok(user.id)
    } else {
        Err(anyhow::anyhow!("User not found"))
    }
}

/// Fetch the Twitch user ID for the owner of the supplied token (no login param).
/// Twitch EventSub subscriptions require this as `user_id` in the condition.
pub async fn fetch_token_owner_id(client_id: &str, user_token: &str) -> anyhow::Result<String> {
    let client = Client::new();

    let res = client
        .get("https://api.twitch.tv/helix/users")
        .header("Client-Id", client_id)
        .header("Authorization", format!("Bearer {}", user_token))
        .send()
        .await?;

    if !res.status().is_success() {
        let err_text = res.text().await?;
        error!("Twitch token owner lookup failed: {}", err_text);
        return Err(anyhow::anyhow!("Failed to fetch token owner"));
    }

    let json: UsersResponse = res.json().await?;
    if let Some(user) = json.data.into_iter().next() {
        info!("Token owner ID: {}", user.id);
        Ok(user.id)
    } else {
        Err(anyhow::anyhow!("Token owner not found"))
    }
}

pub async fn subscribe_to_chat(
    client_id: &str,
    user_token: &str,
    session_id: &str,
    broadcaster_user_id: &str,
    token_owner_id: &str,
) -> anyhow::Result<()> {
    let client = Client::new();
    let url = "https://api.twitch.tv/helix/eventsub/subscriptions";

    let payload = json!({
        "type": "channel.chat.message",
        "version": "1",
        "condition": {
            "broadcaster_user_id": broadcaster_user_id,
            "user_id": token_owner_id  // must be the token owner, not necessarily the broadcaster
        },
        "transport": {
            "method": "websocket",
            "session_id": session_id
        }
    });

    let res = client
        .post(url)
        .header("Client-Id", client_id)
        .header("Authorization", format!("Bearer {}", user_token))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    if res.status().is_success() {
        info!("Successfully subscribed to channel.chat.message via EventSub");
    } else {
        let err_text = res.text().await?;
        error!("Failed to subscribe: {}", err_text);
    }

    Ok(())
}
