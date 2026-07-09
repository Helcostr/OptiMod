use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{distributions::Alphanumeric, Rng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use sha2::{Digest, Sha256};
use sqlx::Row;
use std::{collections::HashMap, sync::Arc};
use tokio::{sync::RwLock, task::JoinHandle};
use tracing::{error, info, warn};
use zeroize::Zeroize;

use crate::{
    chat::ws,
    config::app_config::{AppConfig, TwitchConfig},
    core::EventSender,
    events::models::ChatMessage,
    plugins::Plugin,
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub tx: EventSender,
    pub token: Arc<RwLock<Option<String>>>,
    pub plugins: Arc<Vec<Arc<dyn Plugin>>>,
    pub oauth_sessions: Arc<RwLock<HashMap<String, OAuthSession>>>,
    /// Live monitored channel. Mutable at runtime via POST /api/channel.
    /// Source of truth — `config.twitch.channel` is only the startup default.
    pub channel: Arc<RwLock<String>>,
    /// Abort handle for the running WebSocket listener so we can stop it
    /// and respawn when the monitored channel changes.
    pub ws_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    /// PostgreSQL connection pool for message history.
    pub db_pool: Arc<sqlx::Pool<sqlx::Postgres>>,
}

#[derive(Clone)]
pub struct OAuthSession {
    pub code_verifier: String,
}

#[derive(Deserialize)]
pub struct AuthRequest {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct TwitchTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
    scope: Vec<String>,
    token_type: String,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub authenticated: bool,
    pub channel: String,
    pub token_loaded: bool,
    pub pending_oauth_sessions: usize,
    pub ws_running: bool,
}

#[derive(Serialize)]
pub struct FlaggedMessagesResponse {
    pub messages: Vec<ChatMessage>,
}

#[derive(Deserialize)]
pub struct ChannelRequest {
    pub channel: String,
}

#[derive(Serialize)]
pub struct ChannelResponse {
    pub ok: bool,
    pub channel: String,
    pub error: Option<String>,
}

/// Normalize a channel login: trim, strip leading `#`, lowercase.
fn normalize_channel(input: &str) -> String {
    input
        .trim()
        .trim_start_matches('#')
        .trim_start_matches('@')
        .to_lowercase()
}

/// Convert a database row to ChatMessage
fn row_to_message(row: &sqlx::postgres::PgRow) -> ChatMessage {
    ChatMessage {
        message_id: row.try_get("message_id").unwrap_or_default(),
        timestamp_ms: row.try_get::<i64, _>("timestamp_ms").unwrap_or(0) as u128,
        channel_id: row.try_get("channel_id").unwrap_or_default(),
        channel_login: row.try_get("channel_login").unwrap_or_default(),
        user_id: row.try_get("user_id").unwrap_or_default(),
        user_login: row.try_get("user_login").unwrap_or_default(),
        user_name: row.try_get("user_name").unwrap_or_default(),
        badges: serde_json::from_str(&row.try_get::<String, _>("badges").unwrap_or_default())
            .unwrap_or_default(),
        color: row.try_get("color").unwrap_or_default(),
        raw_message: row.try_get("raw_message").unwrap_or_default(),
        normalized_message: row.try_get("normalized_message").ok(),
        security_flags: serde_json::from_str(
            &row.try_get::<String, _>("security_flags")
                .unwrap_or_default(),
        )
        .unwrap_or_default(),
    }
}

/// Start (or restart) the WebSocket listener using the token + channel
/// currently stored on `state`. Aborts any previously running task first.
pub async fn spawn_ws_listener(state: Arc<AppState>) {
    // Abort previous WS task if any.
    if let Some(handle) = state.ws_handle.write().await.take() {
        handle.abort();
        warn!("Aborted previous WebSocket listener");
    }

    // Snapshot token + channel under short-lived locks; release before spawn.
    let user_token = match state.token.read().await.clone() {
        Some(t) => t,
        None => {
            warn!("No OAuth token yet; deferring WebSocket listener");
            return;
        }
    };
    let twitch_config = TwitchConfig {
        channel: state.channel.read().await.clone(),
        client_id: state.config.twitch.client_id.clone(),
        client_secret: state.config.twitch.client_secret.clone(),
        user_token,
    };
    let tx = state.tx.clone();
    let plugins = state.plugins.clone();

    let handle = tokio::spawn(async move {
        if let Err(e) = ws::connect_and_listen(twitch_config, tx, plugins).await {
            error!("Chat listener error: {}", e);
        }
    });

    *state.ws_handle.write().await = Some(handle);
    info!(
        "Spawned WebSocket listener for channel: {}",
        state.channel.read().await
    );
}

/// Switch the monitored channel and restart the WS listener.
pub async fn switch_channel_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChannelRequest>,
) -> impl IntoResponse {
    let new_channel = normalize_channel(&req.channel);
    if new_channel.is_empty() {
        return Json(ChannelResponse {
            ok: false,
            channel: state.channel.read().await.clone(),
            error: Some("channel name is empty".into()),
        });
    }

    let token = match state.token.read().await.clone() {
        Some(t) => t,
        None => {
            return Json(ChannelResponse {
                ok: false,
                channel: state.channel.read().await.clone(),
                error: Some("not authenticated; login with Twitch first".into()),
            });
        }
    };

    // Validate the channel exists via Twitch API before swapping config.
    match crate::chat::http::fetch_user_id(&state.config.twitch.client_id, &token, &new_channel)
        .await
    {
        Ok(_) => {}
        Err(e) => {
            warn!("Channel '{}' not found: {}", new_channel, e);
            return Json(ChannelResponse {
                ok: false,
                channel: state.channel.read().await.clone(),
                error: Some(format!("channel '{}' not found on Twitch", new_channel)),
            });
        }
    }

    {
        let mut guard = state.channel.write().await;
        info!("Switching monitored channel: {} -> {}", guard, new_channel);
        *guard = new_channel.clone();
    }
    spawn_ws_listener(state.clone()).await;

    Json(ChannelResponse {
        ok: true,
        channel: new_channel,
        error: None,
    })
}

pub async fn status_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let token_loaded = state.token.read().await.is_some();
    let pending_oauth_sessions = state.oauth_sessions.read().await.len();
    let ws_running = state.ws_handle.read().await.is_some();
    let channel = state.channel.read().await.clone();
    Json(StatusResponse {
        authenticated: token_loaded,
        channel,
        token_loaded,
        pending_oauth_sessions,
        ws_running,
    })
}

/// Query flagged messages for moderation/debugging.
pub async fn flagged_messages_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let limit: i64 = 100;

    let rows = match sqlx::query(
        r#"
        SELECT * FROM messages
        WHERE security_flags != '[]'::jsonb
        ORDER BY timestamp_ms DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(&*state.db_pool) // &Arc<Pool> dereferenced to &Pool
    .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Failed to query flagged messages: {}", e);
            return Json(FlaggedMessagesResponse { messages: vec![] });
        }
    };

    let messages = rows.into_iter().map(|row| row_to_message(&row)).collect();

    Json(FlaggedMessagesResponse { messages })
}

/// Query all messages for debugging (with optional flag filter).
#[derive(Deserialize)]
pub struct MessagesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub flagged_only: Option<bool>,
}

pub async fn messages_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MessagesQuery>,
) -> impl IntoResponse {
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);
    let flagged_only = params.flagged_only.unwrap_or(false);

    let rows = if flagged_only {
        sqlx::query(
                "SELECT * FROM messages WHERE security_flags != '[]'::jsonb ORDER BY timestamp_ms DESC LIMIT $1 OFFSET $2",
            )
            .bind(limit)
            .bind(offset)
            .fetch_all(&*state.db_pool)
            .await
    } else {
        sqlx::query("SELECT * FROM messages ORDER BY timestamp_ms DESC LIMIT $1 OFFSET $2")
            .bind(limit)
            .bind(offset)
            .fetch_all(&*state.db_pool)
            .await
    };

    let messages = match rows {
        Ok(r) => r.into_iter().map(|row| row_to_message(&row)).collect(),
        Err(e) => {
            error!("Failed to query messages: {}", e);
            vec![]
        }
    };

    Json(FlaggedMessagesResponse { messages })
}

pub async fn login_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let client_id = &state.config.twitch.client_id;
    let redirect_uri = "http://localhost:3000/auth/callback";
    let scope = "user:read:chat";
    let state_param: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    let code_verifier: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();
    let code_challenge = pkce_challenge(&code_verifier);

    state
        .oauth_sessions
        .write()
        .await
        .insert(state_param.clone(), OAuthSession { code_verifier });

    let url = format!(
        "https://id.twitch.tv/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        client_id, redirect_uri, scope, state_param, code_challenge
    );
    Redirect::temporary(&url)
}

pub async fn callback_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AuthRequest>,
) -> impl IntoResponse {
    let Some(state_value) = params.state.as_deref() else {
        return Redirect::temporary("/?error=missing_state");
    };

    let Some(session) = state.oauth_sessions.write().await.remove(state_value) else {
        return Redirect::temporary("/?error=invalid_state");
    };

    info!("Received OAuth callback with code");

    let client = Client::new();
    let client_id = &state.config.twitch.client_id;
    let client_secret = &state.config.twitch.client_secret;
    let redirect_uri = "http://localhost:3000/auth/callback";

    let auth_params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("code", &params.code),
        ("code_verifier", session.code_verifier.as_str()),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
    ];

    let res = client
        .post("https://id.twitch.tv/oauth2/token")
        .form(&auth_params)
        .send()
        .await;

    match res {
        Ok(response) if response.status().is_success() => {
            if let Ok(mut token_data) = response.json::<TwitchTokenResponse>().await {
                info!("Successfully obtained User Access Token!");
                let mut token = state.token.write().await;
                token_data.refresh_token.zeroize();
                *token = Some(token_data.access_token.clone());
                drop(token);

                spawn_ws_listener(state.clone()).await;

                return Redirect::temporary("/");
            }
            error!("Failed to parse token response");
        }
        Ok(response) => {
            if let Ok(text) = response.text().await {
                error!("Token exchange failed: {}", text);
            }
        }
        Err(e) => error!("Request failed: {}", e),
    }

    Redirect::temporary("/?error=auth_failed")
}

fn pkce_challenge(verifier: &str) -> String {
    let digest = Sha256::digest(verifier.as_bytes());
    URL_SAFE_NO_PAD.encode(digest)
}
