use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::feed::Episode;

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id   TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            created_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS passkey_credentials (
            id   TEXT PRIMARY KEY,
            user_id  TEXT NOT NULL,
            username TEXT NOT NULL,
            credential_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS episodes (
            id   TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            source_url TEXT NOT NULL,
            audio_path TEXT NOT NULL,
            audio_url  TEXT NOT NULL,
            duration_secs INTEGER,
            added_at TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn get_or_create_user(pool: &SqlitePool, username: &str) -> Result<Uuid> {
    if let Some(row) = sqlx::query("SELECT id FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await?
    {
        let id_str: String = row.try_get("id")?;
        return Ok(Uuid::parse_str(&id_str)?);
    }

    let id = Uuid::new_v4();
    let id_str = id.to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query("INSERT INTO users (id, username, created_at) VALUES (?, ?, ?)")
        .bind(&id_str)
        .bind(username)
        .bind(&now)
        .execute(pool)
        .await?;
    Ok(id)
}

pub async fn save_credential(
    pool: &SqlitePool,
    user_id: Uuid,
    username: &str,
    credential_json: &str,
) -> Result<()> {
    let id = Uuid::new_v4().to_string();
    let uid = user_id.to_string();
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO passkey_credentials (id, user_id, username, credential_json, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&uid)
    .bind(username)
    .bind(credential_json)
    .bind(&now)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_credentials_for_user(pool: &SqlitePool, username: &str) -> Result<Vec<String>> {
    let rows =
        sqlx::query("SELECT credential_json FROM passkey_credentials WHERE username = ?")
            .bind(username)
            .fetch_all(pool)
            .await?;
    Ok(rows
        .into_iter()
        .filter_map(|r| r.try_get::<String, _>("credential_json").ok())
        .collect())
}

pub async fn insert_episode(pool: &SqlitePool, episode: &Episode) -> Result<()> {
    let id = episode.id.to_string();
    let added_at = episode.added_at.to_rfc3339();
    let duration = episode.duration_secs.map(|d| d as i64);
    sqlx::query(
        "INSERT INTO episodes
         (id, title, source_url, audio_path, audio_url, duration_secs, added_at)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&episode.title)
    .bind(&episode.source_url)
    .bind(&episode.audio_path)
    .bind(&episode.audio_url)
    .bind(duration)
    .bind(&added_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_credentials_for_user(pool: &SqlitePool, username: &str) -> Result<()> {
    sqlx::query("DELETE FROM passkey_credentials WHERE username = ?")
        .bind(username)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_episodes(pool: &SqlitePool) -> Result<Vec<Episode>> {
    let rows = sqlx::query(
        "SELECT id, title, source_url, audio_path, audio_url, duration_secs, added_at
         FROM episodes ORDER BY added_at DESC",
    )
    .fetch_all(pool)
    .await?;

    let mut episodes = Vec::new();
    for row in rows {
        let id = Uuid::parse_str(&row.try_get::<String, _>("id")?)?;
        let added_at = DateTime::parse_from_rfc3339(&row.try_get::<String, _>("added_at")?)
            .map(|dt| dt.with_timezone(&Utc))?;
        let duration: Option<i64> = row.try_get("duration_secs")?;
        episodes.push(Episode {
            id,
            title: row.try_get("title")?,
            source_url: row.try_get("source_url")?,
            audio_path: row.try_get("audio_path")?,
            audio_url: row.try_get("audio_url")?,
            duration_secs: duration.map(|d| d as u32),
            added_at,
        });
    }
    Ok(episodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;
    use uuid::Uuid;

    use crate::feed::Episode;

    async fn make_pool() -> SqlitePool {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:")
            .unwrap()
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        run_migrations(&pool).await.unwrap();
        pool
    }

    fn make_episode(title: &str) -> Episode {
        Episode {
            id: Uuid::new_v4(),
            title: title.to_string(),
            source_url: "https://youtube.com/watch?v=test".to_string(),
            audio_path: "/media/test.mp3".to_string(),
            audio_url: "http://localhost:3000/audio/test.mp3".to_string(),
            duration_secs: Some(120),
            added_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_migrations_run_successfully() {
        let _pool = make_pool().await;
        // Passes if no panic
    }

    #[tokio::test]
    async fn test_insert_and_list_episode() {
        let pool = make_pool().await;
        let ep = make_episode("Test Episode");
        insert_episode(&pool, &ep).await.unwrap();
        let episodes = list_episodes(&pool).await.unwrap();
        assert_eq!(episodes.len(), 1);
        assert_eq!(episodes[0].title, "Test Episode");
    }

    #[tokio::test]
    async fn test_list_episodes_empty() {
        let pool = make_pool().await;
        let episodes = list_episodes(&pool).await.unwrap();
        assert!(episodes.is_empty());
    }

    #[tokio::test]
    async fn test_insert_multiple_episodes() {
        let pool = make_pool().await;
        for i in 0..3 {
            let mut ep = make_episode(&format!("Episode {}", i));
            ep.source_url = format!("https://youtube.com/watch?v=test{}", i);
            insert_episode(&pool, &ep).await.unwrap();
        }
        let episodes = list_episodes(&pool).await.unwrap();
        assert_eq!(episodes.len(), 3);
    }

    #[tokio::test]
    async fn test_episode_without_duration() {
        let pool = make_pool().await;
        let mut ep = make_episode("No Duration");
        ep.duration_secs = None;
        insert_episode(&pool, &ep).await.unwrap();
        let episodes = list_episodes(&pool).await.unwrap();
        assert_eq!(episodes.len(), 1);
        assert!(episodes[0].duration_secs.is_none());
    }

    #[tokio::test]
    async fn test_get_or_create_user() {
        let pool = make_pool().await;
        let id1 = get_or_create_user(&pool, "alice").await.unwrap();
        let id2 = get_or_create_user(&pool, "alice").await.unwrap();
        assert_eq!(id1, id2, "Same user should get same id");
        let id3 = get_or_create_user(&pool, "bob").await.unwrap();
        assert_ne!(id1, id3, "Different users should get different ids");
    }

    #[tokio::test]
    async fn test_save_and_get_credential() {
        let pool = make_pool().await;
        let user_id = get_or_create_user(&pool, "alice").await.unwrap();
        save_credential(&pool, user_id, "alice", r#"{"fake":"cred"}"#)
            .await
            .unwrap();
        let creds = get_credentials_for_user(&pool, "alice").await.unwrap();
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0], r#"{"fake":"cred"}"#);
    }

    #[tokio::test]
    async fn test_delete_credentials() {
        let pool = make_pool().await;
        let user_id = get_or_create_user(&pool, "alice").await.unwrap();
        save_credential(&pool, user_id, "alice", r#"{"fake":"cred"}"#)
            .await
            .unwrap();
        delete_credentials_for_user(&pool, "alice").await.unwrap();
        let creds = get_credentials_for_user(&pool, "alice").await.unwrap();
        assert!(creds.is_empty());
    }
}
