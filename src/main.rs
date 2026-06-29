use tracing::info;
use tracing_subscriber::EnvFilter;

mod config;

use crate::config::app_config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize configuration
    dotenvy::dotenv().ok();
    
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("secure_twitch_monitor=info".parse()?))
        .init();

    info!("Starting Secure Twitch Chat Monitor...");

    let config = AppConfig::load("config.toml").unwrap_or_else(|_| {
        AppConfig {
            server: config::app_config::ServerConfig { port: 3000 },
            twitch: config::app_config::TwitchConfig { channel: "twitchpresents".into() },
        }
    });

    info!("Loaded config for channel: {}", config.twitch.channel);

    // TODO: Initialize web server, EventSub connection, etc.
    
    // Keep the main thread alive for now
    tokio::signal::ctrl_c().await?;
    info!("Shutting down");

    Ok(())
}
