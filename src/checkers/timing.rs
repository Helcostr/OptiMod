use std::collections::HashMap;
use std::sync::Mutex;

use optimod_checker::Action;
use serde::Serialize;
use sqlx::{Pool, Postgres, Row};
use tracing::info;

#[derive(Default)]
struct Bucket {
    count: u64,
    total_ns: u64,
}

impl Bucket {
    fn record(&mut self, duration_ns: u64) {
        self.count += 1;
        self.total_ns += duration_ns;
    }

    fn merge(&mut self, count: u64, total_ns: u64) {
        self.count += count;
        self.total_ns += total_ns;
    }

    fn snapshot(&self) -> TimingSnapshot {
        TimingSnapshot {
            count: self.count,
            avg_us: if self.count == 0 {
                0.0
            } else {
                self.total_ns as f64 / self.count as f64 / 1_000.0
            },
        }
    }
}

#[derive(Default)]
struct PluginTiming {
    all: Bucket,
    blocked: Bucket,
    passed: Bucket,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimingSnapshot {
    pub count: u64,
    pub avg_us: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PluginTimingSnapshot {
    pub all: TimingSnapshot,
    pub blocked: TimingSnapshot,
    pub passed: TimingSnapshot,
}

impl PluginTiming {
    fn record(&mut self, duration_ns: u64, action: Action) {
        self.all.record(duration_ns);
        match action {
            Action::Block => self.blocked.record(duration_ns),
            Action::Pass | Action::Flag => self.passed.record(duration_ns),
        }
    }

    fn merge(&mut self, action: Action, count: u64, total_ns: u64) {
        self.all.merge(count, total_ns);
        match action {
            Action::Block => self.blocked.merge(count, total_ns),
            Action::Pass | Action::Flag => self.passed.merge(count, total_ns),
        }
    }

    fn snapshot(&self) -> PluginTimingSnapshot {
        PluginTimingSnapshot {
            all: self.all.snapshot(),
            blocked: self.blocked.snapshot(),
            passed: self.passed.snapshot(),
        }
    }
}

/// Per-checker timing aggregates (all messages, blocked-only, passed-only).
pub struct CheckerTimingRegistry {
    plugins: Mutex<HashMap<String, PluginTiming>>,
}

impl CheckerTimingRegistry {
    pub fn new() -> Self {
        Self {
            plugins: Mutex::new(HashMap::new()),
        }
    }

    pub fn record(&self, name: &str, duration_ns: u64, action: Action) {
        let mut guard = self.plugins.lock().expect("timing registry lock");
        guard
            .entry(name.to_string())
            .or_default()
            .record(duration_ns, action);
    }

    pub fn snapshot(&self, name: &str) -> PluginTimingSnapshot {
        let guard = self.plugins.lock().expect("timing registry lock");
        guard
            .get(name)
            .map(PluginTiming::snapshot)
            .unwrap_or_default()
    }

    pub fn snapshots(&self) -> HashMap<String, PluginTimingSnapshot> {
        let guard = self.plugins.lock().expect("timing registry lock");
        guard
            .iter()
            .map(|(name, stats)| (name.clone(), stats.snapshot()))
            .collect()
    }

    pub async fn hydrate_from_db(pool: &Pool<Postgres>) -> anyhow::Result<Self> {
        let registry = Self::new();
        let rows = sqlx::query(
            r#"
            SELECT
                checker->>'name' AS name,
                checker->>'action' AS action,
                COUNT(*)::bigint AS count,
                COALESCE(SUM((checker->>'duration_us')::bigint), 0)::bigint AS total_us
            FROM messages,
                jsonb_array_elements(timings->'checkers') AS checker
            WHERE timings IS NOT NULL
            GROUP BY name, action
            "#,
        )
        .fetch_all(pool)
        .await?;

        let mut guard = registry.plugins.lock().expect("timing registry lock");
        let mut restored = 0u64;
        for row in rows {
            let name: String = row.try_get("name")?;
            let action = parse_action(&row.try_get::<String, _>("action")?);
            let count: i64 = row.try_get("count")?;
            let total_us: i64 = row.try_get("total_us")?;
            if count <= 0 {
                continue;
            }

            let count = count as u64;
            let total_ns = (total_us as u64) * 1_000;
            guard
                .entry(name)
                .or_default()
                .merge(action, count, total_ns);
            restored += count;
        }
        drop(guard);

        info!(
            "Restored checker timing averages from {} stored message samples",
            restored
        );
        Ok(registry)
    }
}

fn parse_action(action: &str) -> Action {
    match action {
        "block" => Action::Block,
        "flag" => Action::Flag,
        _ => Action::Pass,
    }
}

impl Default for CheckerTimingRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginTimingSnapshot {
    fn default_snapshot() -> Self {
        PluginTimingSnapshot {
            all: TimingSnapshot { count: 0, avg_us: 0.0 },
            blocked: TimingSnapshot { count: 0, avg_us: 0.0 },
            passed: TimingSnapshot { count: 0, avg_us: 0.0 },
        }
    }
}

impl Default for PluginTimingSnapshot {
    fn default() -> Self {
        Self::default_snapshot()
    }
}
