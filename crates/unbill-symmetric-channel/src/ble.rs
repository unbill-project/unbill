//! The upstream transport spawns persistent actors without a public shutdown API.
//! Own their runtime so closing a device also tears down its Bluetooth actors.

use std::sync::{Arc, Mutex};

use iroh_ble_transport::{BleTransport, ConnectPath};
use tokio::sync::oneshot;
use unbill_model::{BleConnectPath, BlePeer, BlePeerPhase, BleSnapshot, BleState};

use crate::EndpointIdExt;

// sirno:witness:ble-transport:begin
pub(crate) struct BleDriver {
    pub(crate) transport: Arc<BleTransport>,
    shutdown: Mutex<Option<oneshot::Sender<()>>>,
    stopped: tokio::sync::Mutex<Option<oneshot::Receiver<()>>>,
}

impl BleDriver {
    pub(crate) async fn start(id: iroh::EndpointId) -> Result<Self, String> {
        let (ready_tx, ready_rx) = oneshot::channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (stopped_tx, stopped_rx) = oneshot::channel();
        std::thread::Builder::new()
            .name("unbill-ble".into())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error.to_string()));
                        return;
                    }
                };
                runtime.block_on(async move {
                    // Do not timeout/cancel build: upstream performs bounded rollback.
                    match BleTransport::builder().build(id).await {
                        Ok(transport) => {
                            if ready_tx.send(Ok(transport)).is_ok() {
                                let _ = shutdown_rx.await;
                            }
                        }
                        Err(error) => {
                            let _ = ready_tx.send(Err(error.to_string()));
                        }
                    }
                });
                // Cancels every BLE task, including actors holding cyclic senders.
                drop(runtime);
                let _ = stopped_tx.send(());
            })
            .map_err(|error| error.to_string())?;
        let transport = ready_rx
            .await
            .map_err(|_| "BLE runtime stopped during initialization".to_string())??;
        Ok(Self {
            transport,
            shutdown: Mutex::new(Some(shutdown_tx)),
            stopped: tokio::sync::Mutex::new(Some(stopped_rx)),
        })
    }

    fn stop(&self) {
        if let Some(sender) = self
            .shutdown
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
            .take()
        {
            let _ = sender.send(());
        }
    }

    pub(crate) async fn close(&self) {
        self.stop();
        if let Some(stopped) = self.stopped.lock().await.take() {
            let _ = stopped.await;
        }
    }

    pub(crate) fn snapshot(&self) -> BleSnapshot {
        let mut peers: Vec<_> = self
            .transport
            .snapshot_peers()
            .into_iter()
            .map(map_peer)
            .collect();
        peers.sort_by(|left, right| left.device_id.cmp(&right.device_id));
        BleSnapshot {
            state: BleState::Active,
            peers,
        }
    }
}

impl Drop for BleDriver {
    fn drop(&mut self) {
        self.stop();
    }
}

fn map_peer(peer: iroh_ble_transport::BlePeerInfo) -> BlePeer {
    use iroh_ble_transport::BlePeerPhase as Phase;
    BlePeer {
        device_id: peer.device_id.to_string(),
        phase: match peer.phase {
            Phase::Discovered => BlePeerPhase::Discovered,
            Phase::PendingDial => BlePeerPhase::PendingDial,
            Phase::Connecting => BlePeerPhase::Connecting,
            Phase::Handshaking => BlePeerPhase::Handshaking,
            Phase::Connected => BlePeerPhase::Connected,
            Phase::Draining => BlePeerPhase::Draining,
            Phase::Reconnecting => BlePeerPhase::Reconnecting,
            Phase::Restoring => BlePeerPhase::Restoring,
            _ => BlePeerPhase::Unknown,
        },
        consecutive_failures: peer.consecutive_failures,
        connect_path: peer.connect_path.map(|path| match path {
            ConnectPath::Gatt => BleConnectPath::Gatt,
            ConnectPath::L2cap => BleConnectPath::L2cap,
        }),
        verified_node_id: peer.verified_endpoint.map(|id| id.to_node_id()),
    }
}
// sirno:witness:ble-transport:end

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advertisements_do_not_become_authenticated_node_ids() {
        let peer = map_peer(iroh_ble_transport::BlePeerInfo::new(
            "radio-device".into(),
            iroh_ble_transport::BlePeerPhase::Discovered,
            0,
            None,
            None,
        ));
        assert_eq!(peer.phase, BlePeerPhase::Discovered);
        assert_eq!(peer.verified_node_id, None);
    }

    #[test]
    fn verified_identity_and_transport_path_survive_snapshot_mapping() {
        let id = iroh::SecretKey::from_bytes(&[42; 32]).public();
        let peer = map_peer(iroh_ble_transport::BlePeerInfo::new(
            "verified-device".into(),
            iroh_ble_transport::BlePeerPhase::Connected,
            2,
            Some(ConnectPath::L2cap),
            Some(id),
        ));
        assert_eq!(peer.verified_node_id, Some(id.to_node_id()));
        assert_eq!(peer.connect_path, Some(BleConnectPath::L2cap));
        assert_eq!(peer.consecutive_failures, 2);
    }
}
