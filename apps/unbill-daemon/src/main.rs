use std::sync::Arc;

use anyhow::Result;
use unbill_asymmetric_channel::local::LocalAsymChannel;
use unbill_asymmetric_channel::rpc;
use unbill_store_fs::{FsStore, UNBILL_PATH};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap())
                .add_directive("iroh::socket::remote_map=error".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    let data_dir = UNBILL_PATH.ensure_data_dir()?;
    let socket = UNBILL_PATH.socket_path()?;
    let store = Arc::new(FsStore::open(data_dir)?);
    // sirno:witness:unbill-daemon:begin
    let channel = LocalAsymChannel::open(store).await?;

    tracing::info!("unbill-daemon listening on {}", socket.display());

    tokio::select! {
        res = channel.accept_loop() => {
            if let Err(e) = res {
                tracing::error!("accept_loop: {e}");
            }
        }
        res = rpc::serve(Arc::clone(&channel), &socket) => {
            if let Err(e) = res {
                tracing::error!("rpc::serve: {e}");
            }
        }
    }
    // sirno:witness:unbill-daemon:end

    Ok(())
}
