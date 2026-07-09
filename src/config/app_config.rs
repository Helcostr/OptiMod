use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StorageConfig {
    /// PostgreSQL connection URL: postgresql://user:pass@host:port/db
    /// Or SQLite path: sqlite:./messages.db
    pub database_url: Option<String>,
}

/// Raw TOML representation — secrets are optional here, supplied via env vars.
#[derive(Debug, Clone, Deserialize)]
struct RawTwitchConfig {
    pub channel: String,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub user_token: Option<String>,
}

/// Resolved config — all fields required after merging with env vars.
#[derive(Debug, Clone)]
pub struct TwitchConfig {
    pub channel: String,
    pub client_id: String,
    pub client_secret: String,
    pub user_token: String,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub twitch: TwitchConfig,
    pub storage: StorageConfig,
}

#[derive(Deserialize)]
struct RawAppConfig {
    server: ServerConfig,
    twitch: RawTwitchConfig,
    #[serde(default)]
    storage: Option<StorageConfig>,
}

impl AppConfig {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)?;
        let raw: RawAppConfig = toml::from_str(&contents)?;

        // Overlay env vars on top of whatever was in the file
        let twitch = TwitchConfig {
            channel: raw.twitch.channel,
            client_id: raw
                .twitch
                .client_id
                .or_else(|| std::env::var("TWITCH_CLIENT_ID").ok())
                .unwrap_or_default(),
            client_secret: raw
                .twitch
                .client_secret
                .or_else(|| std::env::var("TWITCH_CLIENT_SECRET").ok())
                .unwrap_or_default(),
            user_token: raw
                .twitch
                .user_token
                .or_else(|| std::env::var("TWITCH_USER_TOKEN").ok())
                .unwrap_or_default(),
        };

        let storage = raw.storage.unwrap_or(StorageConfig { database_url: None });

        Ok(AppConfig {
            server: raw.server,
            twitch,
            storage,
        })
    }

    /// Fallback when no config file is found — read everything from env.
    pub fn from_env() -> Self {
        AppConfig {
            server: ServerConfig { port: 3000 },
            twitch: TwitchConfig {
                channel: std::env::var("TWITCH_CHANNEL")
                    .unwrap_or_else(|_| "twitchpresents".into()),
                client_id: std::env::var("TWITCH_CLIENT_ID").unwrap_or_default(),
                client_secret: std::env::var("TWITCH_CLIENT_SECRET").unwrap_or_default(),
                user_token: std::env::var("TWITCH_USER_TOKEN").unwrap_or_default(),
            },
            storage: StorageConfig {
                database_url: std::env::var("DATABASE_URL").ok(),
            },
        }
    }
}
