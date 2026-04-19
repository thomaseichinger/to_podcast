use chrono::{DateTime, Utc};
use rss::{ChannelBuilder, ItemBuilder, EnclosureBuilder, extension::itunes::{
    ITunesChannelExtensionBuilder, ITunesItemExtensionBuilder,
}};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: Uuid,
    pub title: String,
    pub source_url: String,
    pub audio_path: String,
    pub audio_url: String,
    pub duration_secs: Option<u32>,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedConfig {
    pub title: String,
    pub description: String,
    pub link: String,
    pub author: String,
}

/// Format duration as HH:MM:SS for iTunes
fn format_duration(secs: u32) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

/// Generate RSS XML feed string for given episodes and feed config
pub fn generate_rss(episodes: &[Episode], config: &FeedConfig) -> String {
    let itunes_channel = ITunesChannelExtensionBuilder::default()
        .author(Some(config.author.clone()))
        .build();

    let mut items = Vec::new();

    for episode in episodes {
        let enclosure = EnclosureBuilder::default()
            .url(episode.audio_url.clone())
            .mime_type("audio/mpeg".to_string())
            .length("0".to_string()) // length in bytes; 0 as placeholder
            .build();

        let duration_str = episode.duration_secs.map(format_duration);

        let itunes_item = ITunesItemExtensionBuilder::default()
            .duration(duration_str)
            .author(Some(config.author.clone()))
            .build();

        let pub_date = episode.added_at.format("%a, %d %b %Y %H:%M:%S GMT").to_string();

        let item = ItemBuilder::default()
            .title(Some(episode.title.clone()))
            .link(Some(episode.source_url.clone()))
            .enclosure(Some(enclosure))
            .pub_date(Some(pub_date))
            .guid(Some(rss::GuidBuilder::default()
                .value(episode.id.to_string())
                .permalink(false)
                .build()))
            .itunes_ext(Some(itunes_item))
            .build();

        items.push(item);
    }

    let channel = ChannelBuilder::default()
        .title(config.title.clone())
        .description(config.description.clone())
        .link(config.link.clone())
        .itunes_ext(Some(itunes_channel))
        .items(items)
        .build();

    channel.to_string()
}
