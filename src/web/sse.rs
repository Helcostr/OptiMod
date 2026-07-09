use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::get,
    Router,
};
use std::{convert::Infallible, sync::Arc};
use tokio::sync::broadcast::error::RecvError;
use tracing::warn;

use super::auth::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/events", get(events_handler))
        .with_state(state)
}

async fn events_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let mut rx = state.tx.subscribe();
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(message) => {
                    if let Ok(payload) = serde_json::to_string(&message) {
                        yield Ok(Event::default().event("chat-message").data(payload));
                    }
                }
                Err(RecvError::Lagged(skipped)) => warn!(skipped, "SSE receiver lagged"),
                Err(RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream)
}
