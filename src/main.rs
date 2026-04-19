use to_podcast::{app, config::AppConfig};
use tracing_subscriber::{EnvFilter, fmt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    fmt().with_env_filter(EnvFilter::from_default_env()).init();

    let config = AppConfig::from_env();
    let addr = config.host.clone();

    tracing::info!("Starting server on {}", addr);

    let router = app::build_app(config).await?;
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, router).await?;
    Ok(())
}
