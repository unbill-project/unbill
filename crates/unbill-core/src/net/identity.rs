// Identity transfer protocol handler (`unbill/identity/v1`).
//
// `run_identity_host`      — existing device: validate token, send identity.
// `run_identity_requester` — new device: present token, receive and store identity.
//
// No Iroh dependency — operates on abstract streams for testability.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::model::Ulid;
use crate::service::Identity;
use crate::storage::LedgerStore;

use super::protocol::{
    IdentityError, IdentityReply, IdentityRequest, IdentityResponse, read_msg, write_msg,
};

/// Shared map of pending identity tokens: token hex → `(user_id, display_name)`.
pub type PendingIdentityTokens = Arc<Mutex<HashMap<String, (Ulid, String)>>>;

const IDENTITIES_KEY: &str = "identities.json";

// ---------------------------------------------------------------------------
// Host side
// ---------------------------------------------------------------------------

/// Validate the token and send the associated identity to the new device.
pub async fn run_identity_host<R, W>(
    tokens: &PendingIdentityTokens,
    mut reader: R,
    mut writer: W,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let req: IdentityRequest = read_msg(&mut reader).await?;

    // Consume the token.
    let entry = {
        let mut map = tokens.lock().unwrap();
        map.remove(&req.token)
    };

    match entry {
        None => {
            write_msg(
                &mut writer,
                &IdentityReply::Err(IdentityError {
                    reason: "unknown or expired token".to_string(),
                }),
            )
            .await?;
        }
        Some((user_id, display_name)) => {
            write_msg(
                &mut writer,
                &IdentityReply::Ok(IdentityResponse {
                    user_id: user_id.to_string(),
                    display_name,
                }),
            )
            .await?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Requester side
// ---------------------------------------------------------------------------

/// Send the token, receive the identity, and persist it to the device store.
/// Returns the received `Identity` on success.
pub async fn run_identity_requester<R, W>(
    token: String,
    store: &Arc<dyn LedgerStore>,
    mut reader: R,
    mut writer: W,
) -> anyhow::Result<Identity>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    write_msg(&mut writer, &IdentityRequest { token }).await?;

    let reply: IdentityReply = read_msg(&mut reader).await?;
    match reply {
        IdentityReply::Ok(resp) => {
            let user_id = Ulid::from_string(&resp.user_id)
                .map_err(|e| anyhow::anyhow!("received invalid user_id: {e}"))?;
            let identity = Identity {
                user_id,
                display_name: resp.display_name,
            };

            let mut identities: Vec<Identity> =
                match store.load_device_meta(IDENTITIES_KEY).await? {
                    None => vec![],
                    Some(bytes) => serde_json::from_slice(&bytes)
                        .map_err(|e| anyhow::anyhow!("identities.json: {e}"))?,
                };

            if !identities.iter().any(|i| i.user_id == identity.user_id) {
                identities.push(identity.clone());
                let bytes = serde_json::to_vec(&identities)
                    .map_err(|e| anyhow::anyhow!("serialize identities: {e}"))?;
                store.save_device_meta(IDENTITIES_KEY, &bytes).await?;
            }

            Ok(identity)
        }
        IdentityReply::Err(e) => {
            anyhow::bail!("identity transfer rejected: {}", e.reason)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    use crate::model::{Timestamp, Ulid};
    use crate::service::Identity;
    use crate::storage::InMemoryStore;

    use super::{run_identity_host, run_identity_requester, PendingIdentityTokens};

    fn make_store() -> Arc<InMemoryStore> {
        Arc::new(InMemoryStore::default())
    }

    #[tokio::test]
    async fn test_identity_transfer_round_trip() {
        let user_id = Ulid::new();
        let display_name = "Alice".to_string();

        let token = format!("{:0>64x}", rand::random::<u128>());
        let tokens: PendingIdentityTokens = Arc::new(Mutex::new(HashMap::from([(
            token.clone(),
            (user_id, display_name.clone()),
        )])));

        let requester_store: Arc<dyn crate::storage::LedgerStore> = make_store();

        let (stream_host, stream_requester) = tokio::io::duplex(8 * 1024);
        let (host_read, host_write) = tokio::io::split(stream_host);
        let (req_read, req_write) = tokio::io::split(stream_requester);

        let tokens2 = Arc::clone(&tokens);
        let requester_store2 = Arc::clone(&requester_store);
        let token2 = token.clone();

        let task_host =
            tokio::spawn(
                async move { run_identity_host(&tokens2, host_read, host_write).await.unwrap() },
            );
        let received: Identity = tokio::spawn(async move {
            run_identity_requester(token2, &requester_store2, req_read, req_write)
                .await
                .unwrap()
        })
        .await
        .unwrap();

        task_host.await.unwrap();

        assert_eq!(received.user_id, user_id);
        assert_eq!(received.display_name, display_name);

        // Token was consumed.
        assert!(tokens.lock().unwrap().is_empty());

        // Identity is persisted to the store.
        let stored_bytes = requester_store
            .load_device_meta("identities.json")
            .await
            .unwrap()
            .unwrap();
        let stored: Vec<Identity> = serde_json::from_slice(&stored_bytes).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].user_id, user_id);
    }

    #[tokio::test]
    async fn test_identity_transfer_invalid_token_returns_error() {
        let tokens: PendingIdentityTokens = Arc::new(Mutex::new(HashMap::new()));
        let requester_store: Arc<dyn crate::storage::LedgerStore> = make_store();

        let bad_token = format!("{:0>64x}", rand::random::<u128>());

        let (stream_host, stream_requester) = tokio::io::duplex(8 * 1024);
        let (host_read, host_write) = tokio::io::split(stream_host);
        let (req_read, req_write) = tokio::io::split(stream_requester);

        let tokens2 = Arc::clone(&tokens);
        let requester_store2 = Arc::clone(&requester_store);

        let task_host = tokio::spawn(async move {
            run_identity_host(&tokens2, host_read, host_write).await.unwrap();
        });
        let result = tokio::spawn(async move {
            run_identity_requester(bad_token, &requester_store2, req_read, req_write).await
        })
        .await
        .unwrap();

        task_host.await.unwrap();
        assert!(result.is_err(), "should fail with unknown token");
    }
}
