use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use axum::Router;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;
use std::str::FromStr;
use webauthn_rs::prelude::*;

use crate::auth::SessionStore;
use crate::config::AppConfig;
use crate::routes;
use crate::storage;

/// Shared application state threaded through every handler.
#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub pool: SqlitePool,
    pub webauthn: Arc<Webauthn>,
    pub sessions: SessionStore,
    /// username → serialised PasskeyRegistration (JSON)
    pub reg_states: Arc<Mutex<HashMap<String, String>>>,
    /// username → serialised PasskeyAuthentication (JSON)
    pub auth_states: Arc<Mutex<HashMap<String, String>>>,
}

pub async fn build_app(config: AppConfig) -> Result<Router> {
    let opts = SqliteConnectOptions::from_str(&config.database_url)?
        .create_if_missing(true);

    // Use a single connection for :memory: so all queries share the same DB.
    let max_conn = if config.database_url.contains(":memory:") { 1 } else { 5 };
    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(max_conn)
        .connect_with(opts)
        .await?;

    storage::run_migrations(&pool).await?;

    let rp_origin = url::Url::parse(&config.passkey_rp_origin)?;
    let webauthn = WebauthnBuilder::new(&config.passkey_rp_id, &rp_origin)?
        .rp_name(&config.podcast_title)
        .build()?;

    let state = AppState {
        config,
        pool,
        webauthn: Arc::new(webauthn),
        sessions: SessionStore::new(),
        reg_states: Arc::new(Mutex::new(HashMap::new())),
        auth_states: Arc::new(Mutex::new(HashMap::new())),
    };

    Ok(routes::build_router(state))
}
