use tracing::info;
use tracing_subscriber::EnvFilter;

mod config;
mod chat;
mod events;
mod web;

use crate::config::app_config::AppConfig;
use std::sync::Arc;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize configuration
    dotenvy::dotenv().ok();
    
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("secure_twitch_monitor=info".parse()?))
        .init();

    info!("Starting Secure Twitch Chat Monitor...");

    let config = AppConfig::load("config.toml").unwrap_or_else(|_| AppConfig::from_env());

    info!("Loaded config for channel: {}", config.twitch.channel);

    let state = Arc::new(web::auth::AppState {
        config: config.clone(),
    });

    let app = web::auth::router(state);

    let addr = format!("0.0.0.0:{}", config.server.port);
    info!("Starting web server on {}", addr);
    let listener = TcpListener::bind(addr).await?;
    
    axum::serve(listener, app).await?;

    Ok(())
}
