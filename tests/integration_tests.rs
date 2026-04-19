// Integration tests - requires running server
// Most tests here use axum's test utilities
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use tower::ServiceExt;
use to_podcast::config::AppConfig;

async fn build_test_app() -> axum::Router {
    let config = AppConfig {
        host: "127.0.0.1:0".to_string(),
        podcast_title: "Integration Test Podcast".to_string(),
        podcast_description: "Integration test feed".to_string(),
        podcast_link: "http://localhost:3000".to_string(),
        podcast_author: "Test Author".to_string(),
        rss_username: "testuser".to_string(),
        rss_password: "testpass".to_string(),
        database_url: "sqlite::memory:".to_string(),
        media_dir: "/tmp/test_media".to_string(),
        passkey_rp_id: "localhost".to_string(),
        passkey_rp_origin: "http://localhost:3000".to_string(),
    };

    to_podcast::app::build_app(config).await.expect("Failed to build test app")
}

#[tokio::test]
async fn test_index_returns_200() {
    let app = build_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_rss_feed_requires_auth() {
    let app = build_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/feed.rss")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_rss_feed_with_wrong_auth_rejected() {
    let app = build_test_app().await;

    // Base64 encode "wronguser:wrongpass"
    let creds = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        "wronguser:wrongpass",
    );
    let auth_header = format!("Basic {}", creds);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/feed.rss")
                .header(header::AUTHORIZATION, auth_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_rss_feed_with_correct_auth() {
    let app = build_test_app().await;

    let creds = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        "testuser:testpass",
    );
    let auth_header = format!("Basic {}", creds);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/feed.rss")
                .header(header::AUTHORIZATION, auth_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let content_type = response.headers().get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.contains("xml") || content_type.contains("rss"));
}

#[tokio::test]
async fn test_submit_requires_session_token() {
    let app = build_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/submit")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"url": "https://www.youtube.com/watch?v=test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should reject without auth token
    assert!(
        response.status() == StatusCode::UNAUTHORIZED
            || response.status() == StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn test_webauthn_register_start_endpoint_exists() {
    let app = build_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/webauthn/register/start")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username": "testuser"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return 200 or 400 (bad request for invalid data), not 404
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_webauthn_auth_start_endpoint_exists() {
    let app = build_test_app().await;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/webauthn/auth/start")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"username": "testuser"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return something other than 404
    assert_ne!(response.status(), StatusCode::NOT_FOUND);
}

#[cfg(feature = "integration")]
#[tokio::test]
async fn test_full_submit_flow() {
    // This test requires actual passkey auth which is hard to automate
    // Run only with --features integration
    todo!("Implement full submit flow test with mock passkey");
}
