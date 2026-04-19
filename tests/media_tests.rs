// Media module tests - written FIRST (TDD red phase)
use to_podcast::media::{validate_url, AudioFile};
use std::path::PathBuf;

#[test]
fn test_validate_url_valid_youtube() {
    let result = validate_url("https://www.youtube.com/watch?v=dQw4w9WgXcQ");
    assert!(result.is_ok());
}

#[test]
fn test_validate_url_valid_vimeo() {
    let result = validate_url("https://vimeo.com/123456789");
    assert!(result.is_ok());
}

#[test]
fn test_validate_url_invalid_not_url() {
    let result = validate_url("not-a-url");
    assert!(result.is_err());
}

#[test]
fn test_validate_url_empty_string() {
    let result = validate_url("");
    assert!(result.is_err());
}

#[test]
fn test_validate_url_no_scheme() {
    let result = validate_url("www.youtube.com/watch?v=dQw4w9WgXcQ");
    assert!(result.is_err());
}

#[test]
fn test_validate_url_returns_parsed_url() {
    let result = validate_url("https://www.youtube.com/watch?v=test123");
    assert!(result.is_ok());
    let parsed = result.unwrap();
    assert_eq!(parsed.scheme(), "https");
    assert_eq!(parsed.host_str(), Some("www.youtube.com"));
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
    // HTTP URLs should also be valid (yt-dlp handles various sites)
    let result = validate_url("http://example.com/video/123");
    assert!(result.is_ok());
}

#[test]
fn test_validate_url_ftp_rejected() {
    // Only http/https should be valid
    let result = validate_url("ftp://example.com/video.mp4");
    assert!(result.is_err());
}
