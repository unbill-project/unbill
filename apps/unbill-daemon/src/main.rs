use std::sync::Arc;
use std::time::Duration;

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

    let sync_interval = parse_sync_interval();

    let data_dir = UNBILL_PATH.ensure_data_dir()?;
    let socket = UNBILL_PATH.socket_path()?;
    let store = Arc::new(FsStore::open(data_dir)?);
    // sirno:witness:unbill-daemon:begin
    let channel = LocalAsymChannel::open(store).await?;

    tracing::info!("unbill-daemon listening on {}", socket.display());

    let service = Arc::clone(channel.service());

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
        _ = periodic_sync(&service, sync_interval) => {}
    }
    // sirno:witness:unbill-daemon:end

    Ok(())
}

/// Read `UNBILL_SYNC_INTERVAL_SECS` from the environment.
/// Returns `None` if unset, meaning periodic sync is disabled.
fn parse_sync_interval() -> Option<Duration> {
    let val = std::env::var("UNBILL_SYNC_INTERVAL_SECS").ok()?;
    let secs: u64 = val.parse().ok()?;
    if secs == 0 {
        return None;
    }
    Some(Duration::from_secs(secs))
}

async fn periodic_sync(service: &Arc<unbill_device::UnbillDevice>, interval: Option<Duration>) {
    let duration = match interval {
        Some(d) => d,
        None => {
            // No interval configured — sleep forever so tokio::select! ignores this branch.
            std::future::pending::<()>().await;
            return;
        }
    };

    tracing::info!("periodic sync enabled every {}s", duration.as_secs());
    let mut tick = tokio::time::interval(duration);
    // The first tick fires immediately — skip it to let the endpoint establish.
    tick.tick().await;

    loop {
        tick.tick().await;
        let peers = match service.collect_peers().await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("periodic sync: failed to collect peers: {e}");
                continue;
            }
        };
        for peer in peers {
            if let Err(e) = service.trigger_peer_sync(peer.clone()).await {
                tracing::warn!("periodic sync with {peer}: {e}");
            }
        }
    }
}
