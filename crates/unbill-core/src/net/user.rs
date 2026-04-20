// Saved-user transfer protocol handler (`unbill/user/v1`).
//
// `run_user_host`      — existing device: validate token, send the saved user.
// `run_user_requester` — new device: present token, receive and store the saved user.
//
// No Iroh dependency — operates on abstract streams for testability.

use std::sync::Arc;

use tokio::io::{AsyncRead, AsyncWrite};

use crate::model::Ulid;
use crate::service::{LocalUser, load_pending_user_tokens, save_pending_user_tokens};
use crate::storage::LedgerStore;

use super::protocol::{UserError, UserReply, UserRequest, UserResponse, read_msg, write_msg};

const LOCAL_USERS_KEY: &str = "users.json";

// ---------------------------------------------------------------------------
// Host side
// ---------------------------------------------------------------------------

/// Validate the token and send the associated saved user to the new device.
pub async fn run_user_host<R, W>(
    store: &Arc<dyn LedgerStore>,
    mut reader: R,
    mut writer: W,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let req: UserRequest = read_msg(&mut reader).await?;

    // Load and consume the token.
    let entry = {
        let mut map = load_pending_user_tokens(&**store).await?;
        let entry = map.remove(&req.token);
        save_pending_user_tokens(&**store, &map).await?;
        entry
    };

    match entry {
        None => {
            write_msg(
                &mut writer,
                &UserReply::Err(UserError {
                    reason: "unknown or expired token".to_string(),
                }),
            )
            .await?;
        }
        Some((user_id, display_name)) => {
            write_msg(
                &mut writer,
                &UserReply::Ok(UserResponse {
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

/// Send the token, receive the saved user, and persist it to the device store.
/// Returns the received `LocalUser` on success.
pub async fn run_user_requester<R, W>(
    token: String,
    store: &Arc<dyn LedgerStore>,
    mut reader: R,
    mut writer: W,
) -> anyhow::Result<LocalUser>
where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    write_msg(&mut writer, &UserRequest { token }).await?;

    let reply: UserReply = read_msg(&mut reader).await?;
    match reply {
        UserReply::Ok(resp) => {
            let user_id = Ulid::from_string(&resp.user_id)
                .map_err(|e| anyhow::anyhow!("received invalid user_id: {e}"))?;
            let local_user = LocalUser {
                user_id,
                display_name: resp.display_name,
            };

            let mut local_users: Vec<LocalUser> =
                match store.load_device_meta(LOCAL_USERS_KEY).await? {
                    None => vec![],
                    Some(bytes) => serde_json::from_slice(&bytes)
                        .map_err(|e| anyhow::anyhow!("users.json: {e}"))?,
                };

            if !local_users.iter().any(|i| i.user_id == local_user.user_id) {
                local_users.push(local_user.clone());
                let bytes = serde_json::to_vec(&local_users)
                    .map_err(|e| anyhow::anyhow!("serialize users: {e}"))?;
                store.save_device_meta(LOCAL_USERS_KEY, &bytes).await?;
            }

            Ok(local_user)
        }
        UserReply::Err(e) => {
            anyhow::bail!("user transfer rejected: {}", e.reason)
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use crate::model::Ulid;
    use crate::service::{LocalUser, save_pending_user_tokens};
    use crate::storage::InMemoryStore;

    use super::{run_user_host, run_user_requester};

    fn make_store() -> Arc<InMemoryStore> {
        Arc::new(InMemoryStore::default())
    }

    #[tokio::test]
    async fn test_user_transfer_round_trip() {
        let user_id = Ulid::new();
        let display_name = "Alice".to_string();

        let token = format!("{:0>64x}", rand::random::<u128>());
        let host_store: Arc<dyn crate::storage::LedgerStore> = make_store();

        // Save the token to the store (lazy load — no in-memory map).
        let map = HashMap::from([(token.clone(), (user_id, display_name.clone()))]);
        save_pending_user_tokens(&*host_store, &map).await.unwrap();

        let requester_store: Arc<dyn crate::storage::LedgerStore> = make_store();

        let (stream_host, stream_requester) = tokio::io::duplex(8 * 1024);
        let (host_read, host_write) = tokio::io::split(stream_host);
        let (req_read, req_write) = tokio::io::split(stream_requester);

        let host_store2 = Arc::clone(&host_store);
        let requester_store2 = Arc::clone(&requester_store);
        let token2 = token.clone();

        let task_host = tokio::spawn(async move {
            run_user_host(&host_store2, host_read, host_write)
                .await
                .unwrap()
        });
        let received: LocalUser = tokio::spawn(async move {
            run_user_requester(token2, &requester_store2, req_read, req_write)
                .await
                .unwrap()
        })
        .await
        .unwrap();

        task_host.await.unwrap();

        assert_eq!(received.user_id, user_id);
        assert_eq!(received.display_name, display_name);

        // Token was consumed (store should have an empty map now).
        let remaining = crate::service::load_pending_user_tokens(&*host_store)
            .await
            .unwrap();
        assert!(remaining.is_empty());

        // Local user is persisted to the store.
        let stored_bytes = requester_store
            .load_device_meta("users.json")
            .await
            .unwrap()
            .unwrap();
        let stored: Vec<LocalUser> = serde_json::from_slice(&stored_bytes).unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].user_id, user_id);
    }

    #[tokio::test]
    async fn test_user_transfer_invalid_token_returns_error() {
        // No tokens saved to store.
        let host_store: Arc<dyn crate::storage::LedgerStore> = make_store();
        let requester_store: Arc<dyn crate::storage::LedgerStore> = make_store();

        let bad_token = format!("{:0>64x}", rand::random::<u128>());

        let (stream_host, stream_requester) = tokio::io::duplex(8 * 1024);
        let (host_read, host_write) = tokio::io::split(stream_host);
        let (req_read, req_write) = tokio::io::split(stream_requester);

        let host_store2 = Arc::clone(&host_store);
        let requester_store2 = Arc::clone(&requester_store);

        let task_host = tokio::spawn(async move {
            run_user_host(&host_store2, host_read, host_write)
                .await
                .unwrap();
        });
        let result = tokio::spawn(async move {
            run_user_requester(bad_token, &requester_store2, req_read, req_write).await
        })
        .await
        .unwrap();

        task_host.await.unwrap();
        assert!(result.is_err(), "should fail with unknown token");
    }
}
