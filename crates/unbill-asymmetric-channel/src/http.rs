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
#[serde(rename_all = "camelCase")]
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

        Ok(Self {
            client,
            base_url,
            api_key,
            device_node_id,
            events,
        })
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
        // TODO: unbill-server has no SSE endpoint yet.
        self.events.subscribe()
    }
}
