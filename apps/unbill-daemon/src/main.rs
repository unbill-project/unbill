use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use unbill_asymmetric_channel::local::LocalAsymChannel;
use unbill_asymmetric_channel::rpc::{self, DEFAULT_ADDR};
use unbill_store_fs::{FsStore, UNBILL_PATH};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("info".parse().unwrap())
                .add_directive("iroh::socket::remote_map=error".parse().unwrap()),
        )
        .init();

    let data_dir = UNBILL_PATH.ensure_data_dir()?;
    let store = Arc::new(FsStore::new(data_dir));
    let channel = LocalAsymChannel::open(store).await?;

    let addr: SocketAddr = DEFAULT_ADDR.parse()?;
    tracing::info!("unbill-daemon listening on {addr}");

    tokio::select! {
        res = channel.accept_loop() => {
            if let Err(e) = res {
                tracing::error!("accept_loop: {e}");
            }
        }
        res = rpc::serve(Arc::clone(&channel), addr) => {
            if let Err(e) = res {
                tracing::error!("rpc::serve: {e}");
            }
        }
    }

    Ok(())
}
