// Feed module tests - written FIRST (TDD red phase)
use to_podcast::feed::{Episode, FeedConfig, generate_rss};
use chrono::Utc;
use uuid::Uuid;

fn make_episode(title: &str, audio_url: &str) -> Episode {
    Episode {
        id: Uuid::new_v4(),
        title: title.to_string(),
        source_url: "https://www.youtube.com/watch?v=dQw4w9WgXcQ".to_string(),
        audio_path: format!("/media/{}.mp3", title.replace(' ', "_")),
        audio_url: audio_url.to_string(),
        duration_secs: Some(180),
        added_at: Utc::now(),
    }
}

fn make_config() -> FeedConfig {
    FeedConfig {
        title: "Test Podcast".to_string(),
        description: "A test podcast feed".to_string(),
        link: "http://localhost:3000".to_string(),
        author: "Test Author".to_string(),
    }
}

#[test]
fn test_episode_creation() {
    let ep = make_episode("My Episode", "http://localhost:3000/audio/my_episode.mp3");
    assert_eq!(ep.title, "My Episode");
    assert_eq!(ep.duration_secs, Some(180));
    assert!(!ep.audio_url.is_empty());
}

#[test]
fn test_generate_rss_valid_xml() {
    let episodes = vec![make_episode("Episode One", "http://localhost:3000/audio/ep1.mp3")];
    let config = make_config();
    let rss_str = generate_rss(&episodes, &config);

    // Must be valid XML with RSS root
    assert!(rss_str.contains("<?xml"));
    assert!(rss_str.contains("<rss"));
    assert!(rss_str.contains("</rss>"));
}

#[test]
fn test_generate_rss_contains_channel_info() {
    let episodes = vec![];
    let config = make_config();
    let rss_str = generate_rss(&episodes, &config);

    assert!(rss_str.contains("Test Podcast"));
    assert!(rss_str.contains("A test podcast feed"));
    assert!(rss_str.contains("http://localhost:3000"));
}

#[test]
fn test_generate_rss_contains_episode() {
    let episodes = vec![make_episode("Great Episode", "http://localhost:3000/audio/great.mp3")];
    let config = make_config();
    let rss_str = generate_rss(&episodes, &config);

    assert!(rss_str.contains("Great Episode"));
    assert!(rss_str.contains("http://localhost:3000/audio/great.mp3"));
    assert!(rss_str.contains("<item>") || rss_str.contains("<item "));
}

#[test]
fn test_generate_rss_multiple_episodes() {
    let episodes = vec![
        make_episode("Episode A", "http://localhost:3000/audio/a.mp3"),
        make_episode("Episode B", "http://localhost:3000/audio/b.mp3"),
        make_episode("Episode C", "http://localhost:3000/audio/c.mp3"),
    ];
    let config = make_config();
    let rss_str = generate_rss(&episodes, &config);

    assert!(rss_str.contains("Episode A"));
    assert!(rss_str.contains("Episode B"));
    assert!(rss_str.contains("Episode C"));
}

#[test]
fn test_generate_rss_has_itunes_namespace() {
    let episodes = vec![make_episode("Test", "http://localhost:3000/audio/test.mp3")];
    let config = make_config();
    let rss_str = generate_rss(&episodes, &config);

    assert!(rss_str.contains("itunes"));
}

#[test]
fn test_generate_rss_enclosure_for_audio() {
    let episodes = vec![make_episode("Enc Test", "http://localhost:3000/audio/enc.mp3")];
    let config = make_config();
    let rss_str = generate_rss(&episodes, &config);

    // Should have enclosure tag for podcast players
    assert!(rss_str.contains("enclosure"));
    assert!(rss_str.contains("audio/mpeg"));
}

#[test]
fn test_generate_rss_duration_in_itunes() {
    let episodes = vec![make_episode("Duration Test", "http://localhost:3000/audio/dur.mp3")];
    let config = make_config();
    let rss_str = generate_rss(&episodes, &config);

    assert!(rss_str.contains("itunes:duration") || rss_str.contains("duration"));
}

#[test]
fn test_episode_no_duration() {
    let mut ep = make_episode("No Duration", "http://localhost:3000/audio/nodur.mp3");
    ep.duration_secs = None;
    let config = make_config();
    let rss_str = generate_rss(&[ep], &config);
    // Should still generate valid RSS
    assert!(rss_str.contains("<rss"));
    assert!(rss_str.contains("No Duration"));
}
