use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
};

use crate::app::AppState;
use crate::auth::verify_basic_auth;
use crate::feed::{generate_rss, FeedConfig};
use crate::storage;

pub async fn rss_feed(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if auth.is_empty() {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"Podcast Feed\"")],
            String::new(),
        )
            .into_response();
    }

    if !verify_basic_auth(auth, &state.config.rss_username, &state.config.rss_password) {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Basic realm=\"Podcast Feed\"")],
            String::new(),
        )
            .into_response();
    }

    let episodes = match storage::list_episodes(&state.pool).await {
        Ok(e) => e,
        Err(e) => {
            tracing::error!("list_episodes: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let cfg = FeedConfig {
        title: state.config.podcast_title.clone(),
        description: state.config.podcast_description.clone(),
        link: state.config.podcast_link.clone(),
        author: state.config.podcast_author.clone(),
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/rss+xml; charset=utf-8")],
        generate_rss(&episodes, &cfg),
    )
        .into_response()
}
