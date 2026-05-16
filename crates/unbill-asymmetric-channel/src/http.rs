// HTTP client implementing AsymChannel against unbill-server.
//
// Control plane:  POST /api/v1/ledgers/{id}/invitations
//                 POST /api/v1/ledgers/join
//                 POST /api/v1/peers/{node_id}/sync
// Data plane:     POST /api/v1/ledgers/{id}/sync  (Automerge binary, single round)
// Events:         GET  /api/v1/events              (SSE stream, reqwest streaming)

use async_trait::async_trait;
use futures_util::StreamExt as _;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use unbill_model::error::{Result, UnbillError};
use unbill_model::{Currency, LedgerId, LedgerMeta, NodeId, Timestamp};

use crate::{AsymChannel, AsymChannelEvent};

#[derive(Deserialize)]
struct InvitationJson {
    url: String,
}

#[derive(Serialize)]
struct JoinBody {
    url: String,
    label: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct MetaJson {
    ledger_id: String,
    name: String,
    currency: String,
    created_at_ms: i64,
    updated_at_ms: i64,
}

pub struct HttpAsymChannel {
    client: Client,
    base_url: String,
    api_key: String,
    device_node_id: NodeId,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl HttpAsymChannel {
    /// Connect to an unbill-server and fetch the remote device identity.
    pub async fn open(base_url: impl Into<String>, api_key: impl Into<String>) -> Result<Self> {
        let (events, _) = broadcast::channel(256);
        let api_key = api_key.into();
        let base_url = format!("{}/api/v1", base_url.into().trim_end_matches('/'));
        let client = Client::new();

        let resp = client
            .get(format!("{base_url}/device/id"))
            .bearer_auth(&api_key)
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(UnbillError::Network(format!("HTTP {status}: {body}")));
        }
        let id_str = resp
            .text()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        let device_node_id = NodeId::new(id_str.trim().to_owned());

        let channel = Self {
            client: client.clone(),
            base_url: base_url.clone(),
            api_key: api_key.clone(),
            device_node_id,
            events: events.clone(),
        };

        let sse_url = format!("{base_url}/events");
        let task = sse_task(client, sse_url, api_key, events);

        #[cfg(not(target_arch = "wasm32"))]
        tokio::spawn(task);
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(task);

        Ok(channel)
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.bearer_auth(&self.api_key)
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

#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
impl AsymChannel for HttpAsymChannel {
    fn device_id(&self) -> NodeId {
        self.device_node_id.clone()
    }

    async fn create_invitation(&self, ledger_id: LedgerId) -> Result<String> {
        let resp = self
            .auth(
                self.client
                    .post(self.url(&format!("/ledgers/{ledger_id}/invitations"))),
            )
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        let resp = Self::check(resp).await?;
        let body: InvitationJson = resp
            .json()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Ok(body.url)
    }

    async fn join_ledger(&self, url: String, label: Option<String>) -> Result<()> {
        let resp = self
            .auth(
                self.client
                    .post(self.url("/ledgers/join"))
                    .json(&JoinBody { url, label }),
            )
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Self::check(resp).await?;
        Ok(())
    }

    async fn trigger_peer_sync(&self, peer: NodeId) -> Result<()> {
        let resp = self
            .auth(self.client.post(self.url(&format!("/peers/{peer}/sync"))))
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Self::check(resp).await?;
        Ok(())
    }

    async fn asym_sync(&self, ledger_id: LedgerId, bytes: Vec<u8>) -> Result<Option<Vec<u8>>> {
        let resp = self
            .auth(
                self.client
                    .post(self.url(&format!("/ledgers/{ledger_id}/sync")))
                    .header("content-type", "application/octet-stream")
                    .body(bytes),
            )
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

    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        let resp = self
            .auth(self.client.get(self.url("/ledgers")))
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        let resp = Self::check(resp).await?;
        let items: Vec<MetaJson> = resp
            .json()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        items
            .into_iter()
            .map(|m| {
                let ledger_id = LedgerId::from_string(&m.ledger_id)
                    .map_err(|e| UnbillError::Network(e.to_string()))?;
                let currency = Currency::from_code(&m.currency).ok_or_else(|| {
                    UnbillError::Network(format!("unknown currency {:?}", m.currency))
                })?;
                Ok(LedgerMeta {
                    ledger_id,
                    name: m.name,
                    currency,
                    created_at: Timestamp::from_millis(m.created_at_ms),
                    updated_at: Timestamp::from_millis(m.updated_at_ms),
                })
            })
            .collect()
    }

    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        let id = meta.ledger_id.to_string();
        let body = MetaJson {
            ledger_id: id.clone(),
            name: meta.name.clone(),
            currency: meta.currency.code().to_owned(),
            created_at_ms: meta.created_at.as_millis(),
            updated_at_ms: meta.updated_at.as_millis(),
        };
        let resp = self
            .auth(
                self.client
                    .put(self.url(&format!("/ledgers/{id}/meta")))
                    .json(&body),
            )
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Self::check(resp).await?;
        Ok(())
    }

    async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let resp = self
            .auth(self.client.get(self.url(&format!("/device/{key}"))))
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        if resp.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let resp = Self::check(resp).await?;
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Ok(Some(bytes.to_vec()))
    }

    async fn save_device_meta(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        let resp = self
            .auth(
                self.client
                    .put(self.url(&format!("/device/{key}")))
                    .header("content-type", "application/octet-stream")
                    .body(bytes),
            )
            .send()
            .await
            .map_err(|e| UnbillError::Network(e.to_string()))?;
        Self::check(resp).await?;
        Ok(())
    }

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        self.events.subscribe()
    }
}

// ---------------------------------------------------------------------------
// SSE background task
// ---------------------------------------------------------------------------

async fn sse_task(
    client: Client,
    url: String,
    api_key: String,
    tx: broadcast::Sender<AsymChannelEvent>,
) {
    loop {
        let _ = run_sse(&client, &url, &api_key, &tx).await;
        reconnect_delay().await;
    }
}

#[cfg(not(target_arch = "wasm32"))]
async fn reconnect_delay() {
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
}

#[cfg(target_arch = "wasm32")]
async fn reconnect_delay() {}

async fn run_sse(
    client: &Client,
    url: &str,
    api_key: &str,
    tx: &broadcast::Sender<AsymChannelEvent>,
) -> std::result::Result<(), ()> {
    let resp = client
        .get(url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|_| ())?;

    if !resp.status().is_success() {
        return Err(());
    }

    let mut stream = resp.bytes_stream();
    let mut buf = Vec::<u8>::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| ())?;
        buf.extend_from_slice(&chunk);

        while let Some(pos) = buf.iter().position(|&b| b == b'\n') {
            let line_bytes: Vec<u8> = buf.drain(..=pos).collect();
            let line = std::str::from_utf8(&line_bytes)
                .unwrap_or("")
                .trim_end_matches(['\r', '\n']);

            if let Some(data) = line.strip_prefix("data: ")
                && let Ok(event) = serde_json::from_str::<AsymChannelEvent>(data)
            {
                let _ = tx.send(event);
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::sync::broadcast;

    use crate::AsymChannelEvent;

    use super::{Client, run_sse};

    /// Binds an ephemeral TCP listener, spawns a task that accepts one
    /// connection, reads the request (discarded), writes `response_body`
    /// after the HTTP headers, then closes the connection.  Returns the port.
    async fn serve_sse_once(response_body: String) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let _ = stream.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n{}",
                response_body
            );
            stream.write_all(response.as_bytes()).await.unwrap();
            // Dropping `stream` closes the connection → EOF for the client.
        });
        port
    }

    #[tokio::test]
    async fn test_run_sse_delivers_ledger_updated_event() {
        let body =
            "data: {\"type\":\"LedgerUpdated\",\"ledger_id\":\"00000000000000000000000001\"}\n\n"
                .to_string();
        let port = serve_sse_once(body).await;

        let (tx, mut rx) = broadcast::channel(16);
        let client = Client::new();
        let url = format!("http://127.0.0.1:{}/events", port);
        let _ = run_sse(&client, &url, "test-token", &tx).await;

        let event = rx
            .try_recv()
            .expect("expected one event on the broadcast channel");
        assert!(
            matches!(event, AsymChannelEvent::LedgerUpdated { .. }),
            "expected LedgerUpdated, got {:?}",
            event
        );
    }

    #[tokio::test]
    async fn test_run_sse_ignores_non_data_lines() {
        // Comment lines, event: lines, and id: lines must not produce events.
        let body = ": keep-alive\nevent: ping\nid: 42\n\n".to_string();
        let port = serve_sse_once(body).await;

        let (tx, mut rx) = broadcast::channel(16);
        let client = Client::new();
        let url = format!("http://127.0.0.1:{}/events", port);
        let _ = run_sse(&client, &url, "test-token", &tx).await;

        assert!(
            rx.try_recv().is_err(),
            "no event should be delivered for non-data SSE lines"
        );
    }
}
