pub mod rss;
pub mod submit;
pub mod webauthn;

use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Json, Router,
};

use crate::app::AppState;
use crate::storage;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/feed.rss", get(rss::rss_feed))
        .route("/audio/:filename", get(serve_audio))
        .route("/api/episodes", get(list_episodes))
        .route("/api/submit", post(submit::submit_url))
        .route(
            "/api/webauthn/register/start",
            post(webauthn::register_start),
        )
        .route(
            "/api/webauthn/register/finish",
            post(webauthn::register_finish),
        )
        .route("/api/webauthn/auth/start", post(webauthn::auth_start))
        .route("/api/webauthn/auth/finish", post(webauthn::auth_finish))
        .with_state(state)
}

async fn index() -> Html<&'static str> {
    Html(include_str!("../../static/index.html"))
}

async fn list_episodes(State(state): State<AppState>) -> impl IntoResponse {
    match storage::list_episodes(&state.pool).await {
        Ok(episodes) => Json(episodes).into_response(),
        Err(e) => {
            tracing::error!("list_episodes: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn serve_audio(
    Path(filename): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let path = std::path::PathBuf::from(&state.config.media_dir).join(&filename);
    match tokio::fs::read(&path).await {
        Ok(bytes) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "audio/mpeg")],
            bytes,
        )
            .into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
