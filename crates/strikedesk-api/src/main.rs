use std::path::PathBuf;

use strikedesk_api::{load_state, router};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let config_path =
        std::env::var("STRIKEDESK_CONFIG").unwrap_or_else(|_| "config/screener.toml".to_string());
    let state = load_state(PathBuf::from(&config_path).as_path())?;
    let host = state.config.server.host.clone();
    let port = state.config.server.port;
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}")).await?;
    tracing::info!("Strikedesk listening on http://{host}:{port} — badges never place orders");
    axum::serve(listener, app).await?;
    Ok(())
}
