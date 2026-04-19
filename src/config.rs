use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub podcast_title: String,
    pub podcast_description: String,
    pub podcast_link: String,
    pub podcast_author: String,
    pub rss_username: String,
    pub rss_password: String,
    pub database_url: String,
    pub media_dir: String,
    pub passkey_rp_id: String,
    pub passkey_rp_origin: String,
}

impl AppConfig {
    pub fn from_env() -> Self {
        let host = env::var("PODCAST_HOST").unwrap_or_else(|_| "0.0.0.0:3000".to_string());
        let podcast_title =
            env::var("PODCAST_TITLE").unwrap_or_else(|_| "My Podcast".to_string());
        let podcast_description = env::var("PODCAST_DESCRIPTION")
            .unwrap_or_else(|_| "My personal podcast feed".to_string());
        let podcast_link = env::var("PODCAST_LINK")
            .unwrap_or_else(|_| "http://localhost:3000".to_string());
        let podcast_author =
            env::var("PODCAST_AUTHOR").unwrap_or_else(|_| "Podcast Author".to_string());
        let rss_username =
            env::var("RSS_USERNAME").unwrap_or_else(|_| "podcast".to_string());
        let rss_password =
            env::var("RSS_PASSWORD").unwrap_or_else(|_| "changeme".to_string());
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://podcast.db".to_string());
        let media_dir =
            env::var("MEDIA_DIR").unwrap_or_else(|_| "./media".to_string());
        let passkey_rp_id =
            env::var("PASSKEY_RP_ID").unwrap_or_else(|_| "localhost".to_string());
        let passkey_rp_origin =
            env::var("PASSKEY_RP_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_string());

        AppConfig {
            host,
            podcast_title,
            podcast_description,
            podcast_link,
            podcast_author,
            rss_username,
            rss_password,
            database_url,
            media_dir,
            passkey_rp_id,
            passkey_rp_origin,
        }
    }
}
