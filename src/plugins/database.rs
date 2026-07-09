use crate::events::models::ChatMessage;
use crate::plugins::Plugin;
use serde_json;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

pub struct DatabasePlugin {
    tx: mpsc::Sender<ChatMessage>,
    _handle: JoinHandle<()>,
}

impl DatabasePlugin {
    pub async fn new(db_url: &str) -> anyhow::Result<(Pool<Postgres>, Self)> {
        // Retry connection to handle PostgreSQL startup time
        let pool = loop {
            match PgPoolOptions::new()
                .max_connections(5)
                .connect(db_url)
                .await
            {
                Ok(pool) => break pool,
                Err(e) => {
                    error!("Failed to connect to database, retrying...: {}", e);
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        };

        // Create table if not exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                message_id TEXT PRIMARY KEY,
                timestamp_ms BIGINT NOT NULL,
                channel_id TEXT NOT NULL,
                channel_login TEXT NOT NULL,
                user_id TEXT NOT NULL,
                user_login TEXT NOT NULL,
                user_name TEXT NOT NULL,
                badges JSONB NOT NULL,
                color TEXT NOT NULL,
                raw_message TEXT NOT NULL,
                normalized_message TEXT,
                security_flags JSONB NOT NULL
            )
            "#,
        )
        .execute(&pool)
        .await?;

        // Create indexes for efficient queries
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp_ms DESC)
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_messages_flags ON messages(security_flags)
            "#,
        )
        .execute(&pool)
        .await?;

        // Create hypertable for TimescaleDB (enables time-series partitioning)
        let hypertable_result = sqlx::query(
            r#"
            SELECT create_hypertable('messages', 'timestamp_ms', if_not_exists => true)
            "#,
        )
        .execute(&pool)
        .await;

        if let Err(e) = hypertable_result {
            // Hypertable may already exist or TimescaleDB may not be installed
            info!("Hypertable note (info): {}", e);
        }

        info!("Database initialized at {}", db_url);

        let (tx, mut rx) = mpsc::channel::<ChatMessage>(1000);

        let pool_clone = pool.clone();
        let handle = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if let Err(e) = Self::store_message(&pool_clone, &msg).await {
                    error!("Failed to store message: {}", e);
                }
            }
        });

        Ok((
            pool,
            Self {
                tx,
                _handle: handle,
            },
        ))
    }

    async fn store_message(pool: &Pool<Postgres>, msg: &ChatMessage) -> anyhow::Result<()> {
        let badges_json = serde_json::to_string(&msg.badges)?;
        let flags_json = serde_json::to_string(&msg.security_flags)?;

        // Cast strings to jsonb for PostgreSQL
        sqlx::query(
            r#"
            INSERT INTO messages (
                message_id, timestamp_ms, channel_id, channel_login,
                user_id, user_login, user_name, badges, color,
                raw_message, normalized_message, security_flags
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9, $10, $11, $12::jsonb)
            ON CONFLICT (message_id) DO NOTHING
            "#,
        )
        .bind(&msg.message_id)
        .bind(msg.timestamp_ms as i64)
        .bind(&msg.channel_id)
        .bind(&msg.channel_login)
        .bind(&msg.user_id)
        .bind(&msg.user_login)
        .bind(&msg.user_name)
        .bind(&badges_json)
        .bind(&msg.color)
        .bind(&msg.raw_message)
        .bind(&msg.normalized_message)
        .bind(&flags_json)
        .execute(pool)
        .await?;

        Ok(())
    }
}

impl Plugin for DatabasePlugin {
    fn name(&self) -> &'static str {
        "database"
    }

    fn on_message(&self, message: &ChatMessage) {
        // Non-blocking send - if channel is full, message is dropped
        if self.tx.try_send(message.clone()).is_err() {
            warn!("Database queue full, dropping message");
        }
    }
}
