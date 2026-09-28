use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};
use optimod_checker::Action;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::{
    checkers::{drop_blocked_messages, CheckerPipeline},
    events::models::ChatMessage,
    security::pipeline::inspect_message,
    web::auth::AppState,
};

#[derive(Deserialize)]
pub struct DebugMessageRequest {
    pub message: String,
    #[serde(default = "default_user_name")]
    pub user_name: String,
    #[serde(default = "default_true")]
    pub inject_sse: bool,
}

fn default_user_name() -> String {
    "debug_user".to_string()
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
pub struct SecurityStepDetail {
    pub name: String,
    pub action: String,
    pub duration_us: u64,
    pub flags: Vec<String>,
    pub normalized_message: Option<String>,
}

#[derive(Serialize)]
pub struct DebugMessageResponse {
    pub emitted_to_sse: bool,
    pub dropped: bool,
    pub verdict_action: String,
    pub total_duration_us: u64,
    pub checker_duration_us: u64,
    pub security: SecurityStepDetail,
    pub checkers: Vec<crate::checkers::CheckerStepDetail>,
    pub event_plugins_run: Vec<String>,
    pub message: ChatMessage,
}

fn verdict_action_label(action: Action) -> String {
    match action {
        Action::Pass => "pass".to_string(),
        Action::Flag => "flag".to_string(),
        Action::Block => "block".to_string(),
    }
}

pub async fn debug_message_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DebugMessageRequest>,
) -> impl IntoResponse {
    let pipeline_start = Instant::now();
    let raw_message = req.message;
    let channel = state.channel.read().await.clone();

    let security_start = Instant::now();
    let security = inspect_message(&raw_message);
    let security_duration_us = security_start.elapsed().as_micros() as u64;

    let inspect = state.checkers.inspect_timed(&raw_message, true);
    let timings = CheckerPipeline::message_timings(security_duration_us, &inspect);
    let verdict = inspect.verdict;
    let checker_steps = inspect.steps;

    let mut flags = security.flags.clone();
    flags.extend(verdict.security_flags());

    let debug_id = format!(
        "debug-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    let msg_event = ChatMessage::new(
        debug_id,
        "debug-channel".to_string(),
        channel.clone(),
        "debug-user".to_string(),
        req.user_name.to_lowercase(),
        req.user_name.clone(),
        vec![],
        "#9146FF".to_string(),
        raw_message.clone(),
        security.normalized_message.clone(),
        flags,
        Some(timings),
    );

    for plugin in state.plugins.iter() {
        plugin.on_message(&msg_event, &verdict);
    }

    let dropped = !verdict.is_good() && drop_blocked_messages();
    let emitted_to_sse = req.inject_sse && !dropped;
    if emitted_to_sse {
        let _ = state.tx.send(msg_event.clone());
    }

    let total_duration_us = pipeline_start.elapsed().as_micros() as u64;

    Json(DebugMessageResponse {
        emitted_to_sse,
        dropped,
        verdict_action: verdict_action_label(verdict.action),
        total_duration_us,
        checker_duration_us: inspect.total_checker_us,
        security: SecurityStepDetail {
            name: "security-normalization".to_string(),
            action: if security.flags.is_empty() {
                "pass".to_string()
            } else {
                "flag".to_string()
            },
            duration_us: security_duration_us,
            flags: security.flags,
            normalized_message: security.normalized_message,
        },
        checkers: checker_steps,
        event_plugins_run: state
            .plugins
            .iter()
            .map(|plugin| plugin.name().to_string())
            .collect(),
        message: msg_event,
    })
}
