// reqwest HTTP client implementing AsymChannel by calling the HTTP server.
//
// Connects to the SSE /events endpoint in a background task and feeds
// received events into an internal broadcast channel so that
// subscribe_to_server() can hand out receivers.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::StreamExt as _;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use unbill_model::error::{Result, UnbillError};
use unbill_model::{LedgerId, NodeId};

use crate::{AsymChannel, AsymChannelEvent};

#[derive(serde::Deserialize)]
struct InvitationResponse {
    url: String,
}

#[derive(serde::Serialize)]
struct JoinBody {
    url: String,
}

pub struct HttpAsymChannel {
    client: Client,
    base_url: String,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl HttpAsymChannel {
    /// Connect to an HTTP device daemon at `base_url`.
    /// Immediately spawns a background task that maintains the SSE connection.
    pub fn new(base_url: impl Into<String>) -> Arc<Self> {
        let base_url = base_url.into().trim_end_matches('/').to_owned();
        let (tx, _) = broadcast::channel(256);
        let client = Client::new();
        let device = Arc::new(Self {
            client: client.clone(),
            base_url: base_url.clone(),
            events: tx,
        });
        // Spawn SSE listener.
        let tx2 = device.events.clone();
        let events_url = format!("{}/events", base_url);
        tokio::spawn(async move {
            sse_loop(client, events_url, tx2).await;
        });
        device
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn check(resp: reqwest::Response) -> Result<reqwest::Response> {
        if resp.status().is_success() {
            return Ok(resp);
        }
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        Err(UnbillError::Network(format!("HTTP {status}: {body}")))
    }
}

#[async_trait]
impl AsymChannel for HttpAsymChannel {
    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        let resp = self
            .client
            .post(self.url(&format!("/invitations/{ledger_id}")))
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        let resp = Self::check(resp).await?;
        let body: InvitationResponse = resp
            .json()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Ok(body.url)
    }

    async fn join_ledger(&self, url: String) -> Result<()> {
        let resp = self
            .client
            .post(self.url("/join"))
            .json(&JoinBody { url })
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Self::check(resp).await?;
        Ok(())
    }

    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        let resp = self
            .client
            .post(self.url(&format!("/sync/{peer}")))
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Self::check(resp).await?;
        Ok(())
    }

    async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        let resp = self
            .client
            .post(self.url(&format!("/ledgers/{ledger_id}/sync")))
            .header("content-type", "application/octet-stream")
            .body(bytes)
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        if resp.status() == StatusCode::NO_CONTENT {
            return Ok(None);
        }
        let resp = Self::check(resp).await?;
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Ok(Some(bytes.to_vec()))
    }

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        self.events.subscribe()
    }
}

// ---------------------------------------------------------------------------
// SSE listener
// ---------------------------------------------------------------------------

#[derive(Deserialize, Serialize)]
#[serde(tag = "type")]
enum WireEvent {
    LedgerUpdated { ledger_id: LedgerId },
}

/// Connects to the SSE endpoint and forwards events into `tx`, reconnecting
/// on any error after a short delay.
async fn sse_loop(client: Client, url: String, tx: broadcast::Sender<AsymChannelEvent>) {
    loop {
        match client.get(&url).send().await {
            Ok(resp) => {
                let mut stream = resp.bytes_stream();
                let mut buf = String::new();
                while let Some(Ok(chunk)) = stream.next().await {
                    if let Ok(text) = std::str::from_utf8(&chunk) {
                        buf.push_str(text);
                    }
                    // SSE events are separated by a blank line.
                    while let Some(pos) = buf.find("\n\n") {
                        let block = buf[..pos].to_owned();
                        buf = buf[pos + 2..].to_owned();
                        for line in block.lines() {
                            if let Some(data) = line.strip_prefix("data:") {
                                if let Ok(wire) = serde_json::from_str::<WireEvent>(data.trim()) {
                                    let evt = match wire {
                                        WireEvent::LedgerUpdated { ledger_id } => {
                                            AsymChannelEvent::LedgerUpdated { ledger_id }
                                        }
                                    };
                                    let _ = tx.send(evt);
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }
        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}
