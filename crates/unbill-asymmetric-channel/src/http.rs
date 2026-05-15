// HTTP client implementing AsymChannel against unbill-server.
//
// Control plane:  POST /api/v1/ledgers/{id}/invitations
//                 POST /api/v1/ledgers/join
//                 POST /api/v1/peers/{node_id}/sync
// Data plane:     POST /api/v1/ledgers/{id}/sync  (Automerge binary, single round)
// Events:         stub — unbill-server has no SSE endpoint yet

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use unbill_model::error::{Result, UnbillError};
use unbill_model::{LedgerId, NodeId};

use crate::{AsymChannel, AsymChannelEvent};

#[derive(Deserialize)]
struct InvitationJson {
    url: String,
}

#[derive(Serialize)]
struct JoinBody {
    url: String,
}

pub struct HttpAsymChannel {
    client: Client,
    base_url: String,
    api_key: String,
    events: broadcast::Sender<AsymChannelEvent>,
}

impl HttpAsymChannel {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            client: Client::new(),
            base_url: format!("{}/api/v1", base_url.into().trim_end_matches('/')),
            api_key: api_key.into(),
            events,
        }
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

#[async_trait]
impl AsymChannel for HttpAsymChannel {
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

    async fn join_ledger(&self, url: String) -> Result<()> {
        let resp = self
            .auth(
                self.client
                    .post(self.url("/ledgers/join"))
                    .json(&JoinBody { url }),
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

    fn subscribe_to_server(&self) -> broadcast::Receiver<AsymChannelEvent> {
        // TODO: unbill-server has no SSE endpoint yet.
        self.events.subscribe()
    }
}
