use std::sync::Arc;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing::info;

use unbill_asymmetric_channel::local::LocalAsymChannel;
use unbill_console::service::UnbillConsole;
use unbill_server::{AppState, build_router};
use unbill_store_fs::FsStore;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let api_key = std::env::var("API_KEY").context("API_KEY must be set")?;
    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_owned())
        .parse()
        .context("PORT must be a valid port number")?;

    let data_dir = unbill_store_fs::UNBILL_PATH
        .ensure_data_dir()
        .context("failed to resolve data directory")?;
    let store = Arc::new(FsStore::new(data_dir));
    let channel = LocalAsymChannel::open(store)
        .await
        .context("failed to open channel")?;

    info!("device node_id={}", channel.service().device_id());

    let accept_channel = Arc::clone(&channel);
    tokio::spawn(async move {
        if let Err(e) = accept_channel.accept_loop().await {
            tracing::error!("accept loop exited: {e}");
        }
    });

    let service = UnbillConsole::open(channel);
    let state = Arc::new(AppState { service, api_key });
    let router = build_router(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&addr)
        .await
        .with_context(|| format!("failed to bind to {addr}"))?;

    info!("unbill-server listening on {addr}");
    axum::serve(listener, router)
        .await
        .context("server error")?;

    Ok(())
}
