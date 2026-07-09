use std::sync::Arc;

use tokio::{net::TcpListener, sync::RwLock};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod chat;
mod config;
mod core;
mod events;
mod plugins;
mod security;
mod web;

use crate::{
    config::app_config::AppConfig,
    plugins::{database::DatabasePlugin, logger::LoggerPlugin, metrics::MetricsPlugin, Plugin},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("secure_twitch_monitor=info".parse()?),
        )
        .init();

    info!("Starting Secure Twitch Chat Monitor...");

    let config = AppConfig::load("config.toml").unwrap_or_else(|_| AppConfig::from_env());
    info!("Loaded config for channel: {}", config.twitch.channel);

    let (tx, _rx) = core::create_channel();

    // Initialize database plugin (async)
    // Supports both PostgreSQL and SQLite URLs
    let db_url =
        config.storage.database_url.clone().unwrap_or_else(|| {
            "postgresql://twitch:twitchpass@postgres/twitch_messages".to_string()
        });
    info!("Using database URL: {}", db_url);
    let (db_pool, db_plugin) = DatabasePlugin::new(&db_url).await?;

    let plugins: Arc<Vec<Arc<dyn Plugin>>> = Arc::new(vec![
        Arc::new(db_plugin),
        Arc::new(LoggerPlugin),
        Arc::new(MetricsPlugin::new()),
    ]);

    let state = Arc::new(web::auth::AppState {
        config: config.clone(),
        tx,
        token: Arc::new(RwLock::new(None)),
        plugins,
        oauth_sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        channel: Arc::new(RwLock::new(config.twitch.channel.clone())),
        ws_handle: Arc::new(RwLock::new(None)),
        db_pool: Arc::new(db_pool),
    });

    let app = web::router(state);

    let addr = format!("0.0.0.0:{}", config.server.port);
    info!("Starting web server on {}", addr);
    let listener = TcpListener::bind(addr).await?;

    axum::serve(listener, app).await?;
    Ok(())
}
