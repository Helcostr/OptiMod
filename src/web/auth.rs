use axum::{
    extract::{Query, State},
    response::{Html, IntoResponse, Redirect},
    routing::get,
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{distributions::Alphanumeric, Rng};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tracing::{error, info};
use zeroize::Zeroize;

use crate::{chat::ws, config::app_config::{AppConfig, TwitchConfig}, core::EventSender, plugins::Plugin};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub tx: EventSender,
    pub token: Arc<RwLock<Option<String>>>,
    pub plugins: Arc<Vec<Arc<dyn Plugin>>>,
    pub oauth_sessions: Arc<RwLock<HashMap<String, OAuthSession>>>,
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

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/status", get(status_handler))
        .route("/auth/login", get(login_handler))
        .route("/auth/callback", get(callback_handler))
        .with_state(state)
}

async fn index_handler() -> impl IntoResponse {
    Html(r#"<!doctype html><html lang="en"><head><meta charset="UTF-8" /><meta name="viewport" content="width=device-width, initial-scale=1.0" /><title>Secure Twitch Chat Monitor</title></head><body><main><h1>Secure Twitch Chat Monitor</h1><a href="/auth/login">Login with Twitch</a></main></body></html>"#)
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub authenticated: bool,
    pub channel: String,
    pub token_loaded: bool,
    pub pending_oauth_sessions: usize,
}

async fn status_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let token_loaded = state.token.read().await.is_some();
    let pending_oauth_sessions = state.oauth_sessions.read().await.len();
    Json(StatusResponse {
        authenticated: token_loaded,
        channel: state.config.twitch.channel.clone(),
        token_loaded,
        pending_oauth_sessions,
    })
}

async fn login_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let client_id = &state.config.twitch.client_id;
    let redirect_uri = "http://localhost:3000/auth/callback";
    let scope = "user:read:chat";
    let state_param: String = rand::thread_rng().sample_iter(&Alphanumeric).take(32).map(char::from).collect();
    let code_verifier: String = rand::thread_rng().sample_iter(&Alphanumeric).take(64).map(char::from).collect();
    let code_challenge = pkce_challenge(&code_verifier);

    state.oauth_sessions.write().await.insert(state_param.clone(), OAuthSession { code_verifier });

    let url = format!(
        "https://id.twitch.tv/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}&state={}&code_challenge={}&code_challenge_method=S256",
        client_id, redirect_uri, scope, state_param, code_challenge
    );
    Redirect::temporary(&url)
}

async fn callback_handler(
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

    let res = client.post("https://id.twitch.tv/oauth2/token").form(&auth_params).send().await;

    match res {
        Ok(response) if response.status().is_success() => {
            if let Ok(mut token_data) = response.json::<TwitchTokenResponse>().await {
                info!("Successfully obtained User Access Token!");
                let mut token = state.token.write().await;
                token_data.refresh_token.zeroize();
                *token = Some(token_data.access_token.clone());
                drop(token);

                let mut twitch_config: TwitchConfig = state.config.twitch.clone();
                twitch_config.user_token = token_data.access_token;
                let tx = state.tx.clone();
                let plugins = state.plugins.clone();
                tokio::spawn(async move {
                    if let Err(e) = ws::connect_and_listen(twitch_config, tx, plugins).await {
                        error!("Chat listener error: {}", e);
                    }
                });

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
