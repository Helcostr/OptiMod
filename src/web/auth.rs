use axum::{
    extract::{Query, State},
    response::{Redirect, IntoResponse},
    routing::get,
    Router,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, error};
use crate::config::app_config::AppConfig;

pub struct AppState {
    pub config: AppConfig,
}

#[derive(Deserialize)]
pub struct AuthRequest {
    code: String,
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
        .route("/auth/login", get(login_handler))
        .route("/auth/callback", get(callback_handler))
        .with_state(state)
}

async fn login_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let client_id = &state.config.twitch.client_id;
    let redirect_uri = "http://localhost:3000/auth/callback";
    let scope = "user:read:chat";
    let url = format!(
        "https://id.twitch.tv/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}",
        client_id, redirect_uri, scope
    );
    Redirect::temporary(&url)
}

async fn callback_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AuthRequest>,
) -> impl IntoResponse {
    info!("Received OAuth callback with code");
    
    let client = Client::new();
    let client_id = &state.config.twitch.client_id;
    let client_secret = &state.config.twitch.client_secret;
    let redirect_uri = "http://localhost:3000/auth/callback";

    let auth_params = [
        ("client_id", client_id.as_str()),
        ("client_secret", client_secret.as_str()),
        ("code", &params.code),
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
            if let Ok(token_data) = response.json::<TwitchTokenResponse>().await {
                info!("Successfully obtained User Access Token!");
                
                // Update the token in our config clone and spawn the listener
                let mut new_config = state.config.twitch.clone();
                new_config.user_token = token_data.access_token;

                tokio::spawn(async move {
                    if let Err(e) = crate::chat::ws::connect_and_listen(new_config).await {
                        error!("Chat listener error: {}", e);
                    }
                });

                return Redirect::temporary("/");
            } else {
                error!("Failed to parse token response");
            }
        }
        Ok(response) => {
            if let Ok(text) = response.text().await {
                error!("Token exchange failed: {}", text);
            }
        }
        Err(e) => {
            error!("Request failed: {}", e);
        }
    }

    Redirect::temporary("/?error=auth_failed")
}
