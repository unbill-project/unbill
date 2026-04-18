//! Wire protocol: ALPN constants, message types, and CBOR framing helpers.
//!
//! Every message on every protocol is framed as:
//!   `[ u32 big-endian length ][ CBOR-encoded message bytes ]`

use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

// ---------------------------------------------------------------------------
// ALPN identifiers
// ---------------------------------------------------------------------------

/// ALPN token for the document-sync protocol.
pub const ALPN_SYNC: &[u8] = b"unbill/sync/v1";
/// ALPN token for the device-join protocol.
pub const ALPN_JOIN: &[u8] = b"unbill/join/v1";
/// ALPN token for the identity-transfer protocol.
pub const ALPN_IDENTITY: &[u8] = b"unbill/identity/v1";

#[allow(dead_code)]
pub const PROTOCOL_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Sync protocol (`unbill/sync/v1`)
// ---------------------------------------------------------------------------

/// Sent by the initiator immediately after the stream is opened.
///
/// The initiator's `NodeId` is *not* included — the responder reads it from
/// the TLS-verified Iroh connection.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Hello {
    /// ULIDs (as strings) of every ledger this device holds locally.
    pub ledger_ids: Vec<String>,
}

/// Sent by the responder after verifying which ledgers the initiator may sync.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct HelloAck {
    /// Ledger IDs the initiator is authorized for and that this device holds.
    pub accepted: Vec<String>,
    /// Ledger IDs that were rejected (not shared, not authorized, or unknown).
    pub rejected: Vec<String>,
}

/// Carries one Automerge sync message for a single ledger.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SyncMsg {
    pub ledger_id: String,
    /// Opaque bytes produced by `automerge::sync::Message::encode`.
    pub payload: Vec<u8>,
}

/// Signals that the sender has no more sync messages for this ledger.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct SyncDone {
    pub ledger_id: String,
}

/// Top-level envelope for every frame exchanged over the sync stream.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum SyncFrame {
    Hello(Hello),
    HelloAck(HelloAck),
    Msg(SyncMsg),
    Done(SyncDone),
}

// ---------------------------------------------------------------------------
// Join protocol (`unbill/join/v1`)
// ---------------------------------------------------------------------------

/// Sent by the joining device.  The joiner's `NodeId` is read from TLS — it
/// is not included in this message.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JoinRequest {
    /// Hex-encoded 32-byte token from the invite URL.
    pub token: String,
    /// ULID of the ledger to join (from the invite URL).
    pub ledger_id: String,
    /// Human-readable name for this device (e.g. "Alice's phone").
    pub label: String,
}

/// Sent by the host on success.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JoinResponse {
    /// Full Automerge document snapshot (output of `LedgerDoc::save`).
    pub ledger_bytes: Vec<u8>,
}

/// Sent by the host on failure.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct JoinError {
    pub reason: String,
}

/// Host-to-requester reply on the join stream.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum JoinReply {
    Ok(JoinResponse),
    Err(JoinError),
}

// ---------------------------------------------------------------------------
// Identity protocol (`unbill/identity/v1`)
// ---------------------------------------------------------------------------

/// Sent by the new device.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IdentityRequest {
    /// Hex-encoded 32-byte token from the identity invite URL.
    pub token: String,
}

/// Sent by the existing device on success.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IdentityResponse {
    /// Stable ULID for this user (26-character string).
    pub user_id: String,
    pub display_name: String,
}

/// Sent by the existing device on failure.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IdentityError {
    pub reason: String,
}

/// Existing-device-to-new-device reply on the identity stream.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum IdentityReply {
    Ok(IdentityResponse),
    Err(IdentityError),
}

// ---------------------------------------------------------------------------
// Framing helpers
// ---------------------------------------------------------------------------

/// Maximum accepted message payload (16 MiB).
const MAX_MSG_LEN: u32 = 16 * 1024 * 1024;

/// Serialize `msg` as CBOR and write it with a 4-byte big-endian length prefix.
pub async fn write_msg<T, W>(writer: &mut W, msg: &T) -> anyhow::Result<()>
where
    T: Serialize,
    W: AsyncWrite + Unpin,
{
    let mut buf = Vec::new();
    ciborium::into_writer(msg, &mut buf).map_err(|e| anyhow::anyhow!("CBOR encode: {e}"))?;
    let len = u32::try_from(buf.len())
        .map_err(|_| anyhow::anyhow!("message too large to frame"))?;
    if len > MAX_MSG_LEN {
        anyhow::bail!("outgoing message too large: {len} bytes");
    }
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&buf).await?;
    Ok(())
}

/// Read a length-prefixed CBOR frame and deserialize it into `T`.
pub async fn read_msg<T, R>(reader: &mut R) -> anyhow::Result<T>
where
    T: DeserializeOwned,
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 4];
    reader.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);
    if len > MAX_MSG_LEN {
        anyhow::bail!("incoming message too large: {len} bytes");
    }
    let mut buf = vec![0u8; len as usize];
    reader.read_exact(&mut buf).await?;
    ciborium::from_reader(buf.as_slice()).map_err(|e| anyhow::anyhow!("CBOR decode: {e}"))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_round_trip_sync_frame_done() {
        let frame = SyncFrame::Done(SyncDone {
            ledger_id: "01ABCDEFGHIJ0000000000000".to_string(),
        });
        let mut buf = Vec::new();
        write_msg(&mut buf, &frame).await.unwrap();
        let decoded: SyncFrame = read_msg(&mut buf.as_slice()).await.unwrap();
        match decoded {
            SyncFrame::Done(d) => assert_eq!(d.ledger_id, "01ABCDEFGHIJ0000000000000"),
            _ => panic!("wrong variant"),
        }
    }

    #[tokio::test]
    async fn test_round_trip_hello() {
        let frame = SyncFrame::Hello(Hello {
            ledger_ids: vec!["AAAA".to_string(), "BBBB".to_string()],
        });
        let mut buf = Vec::new();
        write_msg(&mut buf, &frame).await.unwrap();
        let decoded: SyncFrame = read_msg(&mut buf.as_slice()).await.unwrap();
        match decoded {
            SyncFrame::Hello(h) => assert_eq!(h.ledger_ids, vec!["AAAA", "BBBB"]),
            _ => panic!("wrong variant"),
        }
    }

    #[tokio::test]
    async fn test_round_trip_join_reply_ok() {
        let reply = JoinReply::Ok(JoinResponse {
            ledger_bytes: vec![1, 2, 3, 4],
        });
        let mut buf = Vec::new();
        write_msg(&mut buf, &reply).await.unwrap();
        let decoded: JoinReply = read_msg(&mut buf.as_slice()).await.unwrap();
        match decoded {
            JoinReply::Ok(r) => assert_eq!(r.ledger_bytes, vec![1, 2, 3, 4]),
            _ => panic!("wrong variant"),
        }
    }

    #[tokio::test]
    async fn test_round_trip_join_reply_err() {
        let reply = JoinReply::Err(JoinError {
            reason: "token expired".to_string(),
        });
        let mut buf = Vec::new();
        write_msg(&mut buf, &reply).await.unwrap();
        let decoded: JoinReply = read_msg(&mut buf.as_slice()).await.unwrap();
        match decoded {
            JoinReply::Err(e) => assert_eq!(e.reason, "token expired"),
            _ => panic!("wrong variant"),
        }
    }

    #[tokio::test]
    async fn test_round_trip_identity_reply() {
        let reply = IdentityReply::Ok(IdentityResponse {
            user_id: "01HX000000000000000000000".to_string(),
            display_name: "Alice".to_string(),
        });
        let mut buf = Vec::new();
        write_msg(&mut buf, &reply).await.unwrap();
        let decoded: IdentityReply = read_msg(&mut buf.as_slice()).await.unwrap();
        match decoded {
            IdentityReply::Ok(r) => {
                assert_eq!(r.user_id, "01HX000000000000000000000");
                assert_eq!(r.display_name, "Alice");
            }
            _ => panic!("wrong variant"),
        }
    }

    #[tokio::test]
    async fn test_multiple_frames_sequential() {
        let mut buf = Vec::new();
        write_msg(&mut buf, &SyncFrame::Hello(Hello { ledger_ids: vec![] }))
            .await
            .unwrap();
        write_msg(
            &mut buf,
            &SyncFrame::Done(SyncDone {
                ledger_id: "X".to_string(),
            }),
        )
        .await
        .unwrap();

        let mut cursor = buf.as_slice();
        let f1: SyncFrame = read_msg(&mut cursor).await.unwrap();
        let f2: SyncFrame = read_msg(&mut cursor).await.unwrap();
        assert!(matches!(f1, SyncFrame::Hello(_)));
        assert!(matches!(f2, SyncFrame::Done(_)));
    }
}
