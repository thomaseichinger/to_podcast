use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};

use crate::app::AppState;

/// GET /api/episodes
///
/// Returns a JSON array of all episodes (no auth required, metadata only).
pub async fn list_episodes(State(state): State<AppState>) -> impl IntoResponse {
    match state.storage.list_episodes().await {
        Ok(episodes) => (StatusCode::OK, Json(episodes)).into_response(),
        Err(e) => {
            tracing::error!("Failed to list episodes: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "Failed to load episodes" })),
            )
                .into_response()
        }
    }
}
