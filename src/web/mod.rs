use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;
use tower_http::services::ServeDir;

pub mod auth;
pub mod debug;
pub mod sse;

use crate::web::auth::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    // API routes first (more specific paths)
    let api_router = Router::new()
        .route("/api/status", get(auth::status_handler))
        .route("/api/plugins", get(auth::plugins_handler))
        .route("/api/debug/message", post(debug::debug_message_handler))
        .route("/api/channel", post(auth::switch_channel_handler))
        .route("/api/flagged", get(auth::flagged_messages_handler))
        .route("/api/messages", get(auth::messages_handler))
        .route("/auth/login", get(auth::login_handler))
        .route("/auth/callback", get(auth::callback_handler))
        .with_state(state.clone());

    // SSE router
    let sse_router = sse::router(state.clone());

    // Static files - serves index.html for "/" and assets for "/assets/*"
    let static_router = Router::new().nest_service(
        "/",
        ServeDir::new("./dist").append_index_html_on_directories(true),
    );

    // Merge: API + SSE routes take precedence over static files
    api_router.merge(sse_router).merge(static_router)
}
