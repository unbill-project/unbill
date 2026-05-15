// Axum HTTP server exposing a AsymChannel over HTTP + SSE.
//
// Routes:
//   POST /ledgers/:id/sync          — asym_sync
//   POST /invitations/:id           — create_invitation
//   POST /join                      — join_ledger  (body: JoinBody)
//   POST /sync/:peer                — trigger_peer_sync
//   GET  /events                    — SSE stream of AsymChannelEvent

use std::convert::Infallible;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use unbill_model::{LedgerId, NodeId};

use crate::AsymChannel;

pub fn build_router(device: Arc<dyn AsymChannel>) -> Router {
    Router::new()
        .route("/ledgers/{id}/sync", post(asym_sync))
        .route("/invitations/{id}", post(create_invitation))
        .route("/join", post(join_ledger))
        .route("/sync/{peer}", post(trigger_peer_sync))
        .route("/events", get(sse_events))
        .with_state(device)
}

// --- Data plane ---

async fn asym_sync(
    State(device): State<Arc<dyn AsymChannel>>,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let ledger_id = match LedgerId::from_string(&id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    match device.asym_sync(ledger_id, body.to_vec()).await {
        Ok(Some(resp)) => (
            StatusCode::OK,
            [("content-type", "application/octet-stream")],
            resp,
        )
            .into_response(),
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// --- Control plane ---

async fn create_invitation(
    State(device): State<Arc<dyn AsymChannel>>,
    Path(id): Path<String>,
) -> Response {
    let ledger_id = match LedgerId::from_string(&id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    match device.create_invitation(ledger_id).await {
        Ok(url) => (StatusCode::CREATED, Json(InvitationResponse { url })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Serialize)]
struct InvitationResponse {
    url: String,
}

#[derive(Deserialize)]
struct JoinBody {
    url: String,
}

async fn join_ledger(
    State(device): State<Arc<dyn AsymChannel>>,
    Json(body): Json<JoinBody>,
) -> Response {
    match device.join_ledger(body.url).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn trigger_peer_sync(
    State(device): State<Arc<dyn AsymChannel>>,
    Path(peer): Path<String>,
) -> Response {
    let node_id = NodeId::new(peer);
    match device.trigger_peer_sync(node_id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// --- SSE event stream ---

async fn sse_events(
    State(device): State<Arc<dyn AsymChannel>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = device.subscribe_to_server();
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        let evt = result.ok()?;
        let data = serde_json::to_string(&evt).ok()?;
        Some(Ok(Event::default().data(data)))
    });
    Sse::new(stream)
}
