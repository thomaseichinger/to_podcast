// Media module tests – written FIRST (TDD red phase)
use to_podcast::media::{
    extract_vimeo_id_from_page, parse_vimeo_config, validate_url, AudioFile,
};
use std::path::PathBuf;

// ── existing URL-validation tests ────────────────────────────────────────────

#[test]
fn test_validate_url_valid_youtube() {
    assert!(validate_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ").is_ok());
}

#[test]
fn test_validate_url_valid_vimeo() {
    assert!(validate_url("https://vimeo.com/123456789").is_ok());
}

#[test]
fn test_validate_url_invalid_not_url() {
    assert!(validate_url("not-a-url").is_err());
}

#[test]
fn test_validate_url_empty_string() {
    assert!(validate_url("").is_err());
}

#[test]
fn test_validate_url_no_scheme() {
    assert!(validate_url("www.youtube.com/watch?v=dQw4w9WgXcQ").is_err());
}

#[test]
fn test_validate_url_returns_parsed_url() {
    let u = validate_url("https://www.youtube.com/watch?v=test123").unwrap();
    assert_eq!(u.scheme(), "https");
    assert_eq!(u.host_str(), Some("www.youtube.com"));
}

#[test]
fn test_audio_file_struct() {
    let af = AudioFile {
        path: PathBuf::from("/tmp/test.mp3"),
        duration_secs: Some(300),
        title: "Test Video Title".to_string(),
    };
    assert_eq!(af.title, "Test Video Title");
    assert_eq!(af.duration_secs, Some(300));
    assert_eq!(af.path.extension().and_then(|e| e.to_str()), Some("mp3"));
}

#[test]
fn test_audio_file_no_duration() {
    let af = AudioFile {
        path: PathBuf::from("/tmp/unknown.mp3"),
        duration_secs: None,
        title: "Unknown Duration".to_string(),
    };
    assert!(af.duration_secs.is_none());
}

#[test]
fn test_validate_url_http_accepted() {
    assert!(validate_url("http://example.com/video/123").is_ok());
}

#[test]
fn test_validate_url_ftp_rejected() {
    assert!(validate_url("ftp://example.com/video.mp4").is_err());
}

// ── Vimeo page-parsing tests (new, TDD red) ──────────────────────────────────

#[test]
fn test_extract_vimeo_id_from_https_iframe() {
    let html = r#"<html><body>
        <iframe src="https://player.vimeo.com/video/123456789" frameborder="0"></iframe>
    </body></html>"#;
    assert_eq!(
        extract_vimeo_id_from_page(html),
        Some("123456789".to_string())
    );
}

#[test]
fn test_extract_vimeo_id_from_protocol_relative_iframe() {
    let html = r#"<iframe src="//player.vimeo.com/video/987654321?autoplay=0"></iframe>"#;
    assert_eq!(
        extract_vimeo_id_from_page(html),
        Some("987654321".to_string())
    );
}

#[test]
fn test_extract_vimeo_id_with_query_params() {
    let html = r#"<iframe src="https://player.vimeo.com/video/111222333?h=abc&autopause=0"></iframe>"#;
    assert_eq!(
        extract_vimeo_id_from_page(html),
        Some("111222333".to_string())
    );
}

#[test]
fn test_extract_vimeo_id_from_data_src() {
    // Some sites lazy-load iframes via data-src
    let html = r#"<iframe data-src="https://player.vimeo.com/video/555666777"></iframe>"#;
    assert_eq!(
        extract_vimeo_id_from_page(html),
        Some("555666777".to_string())
    );
}

#[test]
fn test_extract_vimeo_id_no_embed_returns_none() {
    let html = r#"<html><body><p>No video here at all.</p></body></html>"#;
    assert!(extract_vimeo_id_from_page(html).is_none());
}

#[test]
fn test_extract_vimeo_id_non_vimeo_iframe_returns_none() {
    let html = r#"<iframe src="https://www.youtube.com/embed/dQw4w9WgXcQ"></iframe>"#;
    assert!(extract_vimeo_id_from_page(html).is_none());
}

#[test]
fn test_extract_vimeo_id_prefers_first_embed() {
    let html = r#"
        <iframe src="https://player.vimeo.com/video/111"></iframe>
        <iframe src="https://player.vimeo.com/video/222"></iframe>"#;
    assert_eq!(
        extract_vimeo_id_from_page(html),
        Some("111".to_string())
    );
}

// ── Vimeo config JSON parsing tests ──────────────────────────────────────────

fn vimeo_config_json(title: &str, progressive: &[(&str, u32)]) -> String {
    let prog: Vec<String> = progressive
        .iter()
        .map(|(url, h)| format!(r#"{{"url":"{url}","quality":"{h}p","height":{h}}}"#))
        .collect();
    format!(
        r#"{{"video":{{"title":"{title}","duration":300}},"request":{{"files":{{"progressive":[{}]}}}}}}"#,
        prog.join(",")
    )
}

#[test]
fn test_parse_vimeo_config_picks_highest_quality() {
    let json = vimeo_config_json(
        "My Video",
        &[
            ("https://example.com/720p.mp4", 720),
            ("https://example.com/1080p.mp4", 1080),
            ("https://example.com/360p.mp4", 360),
        ],
    );
    let stream = parse_vimeo_config(&json).unwrap();
    assert_eq!(stream.url, "https://example.com/1080p.mp4");
    assert_eq!(stream.title, "My Video");
}

#[test]
fn test_parse_vimeo_config_single_quality() {
    let json = vimeo_config_json("Solo Video", &[("https://example.com/sd.mp4", 480)]);
    let stream = parse_vimeo_config(&json).unwrap();
    assert_eq!(stream.url, "https://example.com/sd.mp4");
    assert_eq!(stream.title, "Solo Video");
}

#[test]
fn test_parse_vimeo_config_no_progressive_returns_error() {
    let json = r#"{"video":{"title":"HLS Only"},"request":{"files":{"progressive":[]}}}"#;
    assert!(parse_vimeo_config(json).is_err());
}

#[test]
fn test_parse_vimeo_config_invalid_json_returns_error() {
    assert!(parse_vimeo_config("not json at all").is_err());
}
