use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use uuid::Uuid;

use crate::app::AppState;
use crate::feed::Episode;
use crate::media::process_url;
use crate::storage;

#[derive(Deserialize)]
pub struct SubmitRequest {
    pub url: String,
}

pub async fn submit_url(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<SubmitRequest>,
) -> impl IntoResponse {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::to_string))
        .unwrap_or_default();

    if token.is_empty() || !state.sessions.is_valid(&token) {
        return (StatusCode::UNAUTHORIZED, "Valid session token required").into_response();
    }

    if let Err(e) = crate::media::validate_url(&req.url) {
        return (StatusCode::BAD_REQUEST, e.to_string()).into_response();
    }

    let url = req.url.clone();
    let pool = state.pool.clone();
    let media_dir = state.config.media_dir.clone();
    let podcast_link = state.config.podcast_link.clone();

    tokio::spawn(async move {
        let output = std::path::PathBuf::from(&media_dir);
        match process_url(&url, &output).await {
            Ok(audio) => {
                let filename = audio
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("episode.mp3");
                let audio_url = format!("{}/audio/{}", podcast_link, filename);
                let episode = Episode {
                    id: Uuid::new_v4(),
                    title: audio.title,
                    source_url: url,
                    audio_path: audio.path.to_string_lossy().into_owned(),
                    audio_url,
                    duration_secs: audio.duration_secs,
                    added_at: Utc::now(),
                };
                if let Err(e) = storage::insert_episode(&pool, &episode).await {
                    tracing::error!("insert_episode: {e}");
                }
            }
            Err(e) => tracing::error!("process_url failed: {e}"),
        }
    });

    (StatusCode::ACCEPTED, "Queued for processing").into_response()
}
