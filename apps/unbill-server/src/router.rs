use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::trace::TraceLayer;

use unbill_device::{ServiceEvent, UnbillDevice};
use unbill_model::{Currency, LedgerId, LedgerMeta, NodeId, Timestamp};

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

pub struct AppState {
    pub service: Arc<UnbillDevice>,
    pub api_key: String,
}

// ---------------------------------------------------------------------------
// LedgerMeta JSON shape (mirrors FsStore / HttpStore)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct MetaJson {
    pub ledger_id: String,
    pub name: String,
    pub currency: String,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

// ---------------------------------------------------------------------------
// Join request / response shapes
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct JoinBody {
    url: String,
    label: Option<String>,
}

#[derive(Debug, Serialize)]
struct InvitationJson {
    url: String,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn build_router(state: Arc<AppState>) -> Router {
    let protected = Router::new()
        .route("/ledgers", get(list_ledgers))
        .route("/ledgers/{id}/meta", put(save_ledger_meta))
        .route("/ledgers/{id}/sync", post(sync_ledger))
        .route("/ledgers/{id}/invitations", post(create_invitation))
        .route("/ledgers/join", post(join_ledger))
        .route("/peers/{node_id}/sync", post(sync_with_peer))
        .route("/events", get(stream_events))
        .route("/device/id", get(get_device_id))
        .route("/device/{key}", get(load_device_meta).put(save_device_meta))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state);

    Router::new()
        .nest("/api/v1", protected)
        .layer(TraceLayer::new_for_http())
}

// ---------------------------------------------------------------------------
// Auth middleware
// ---------------------------------------------------------------------------

async fn auth(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Response {
    let authorized = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|token| token == state.api_key)
        .unwrap_or(false);

    if authorized {
        next.run(req).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}

// ---------------------------------------------------------------------------
// Device key validation — no path components allowed
// ---------------------------------------------------------------------------

fn valid_device_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

async fn list_ledgers(State(state): State<Arc<AppState>>) -> Response {
    match state.service.store().list_ledgers().await {
        Ok(metas) => {
            let json: Vec<MetaJson> = metas.into_iter().map(meta_to_json).collect();
            Json(json).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn save_ledger_meta(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<MetaJson>,
) -> Response {
    let ledger_id = match LedgerId::from_string(&body.ledger_id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    if ledger_id.to_string() != id {
        return (StatusCode::BAD_REQUEST, "id mismatch").into_response();
    }
    let currency = match Currency::from_code(&body.currency) {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                format!("unknown currency {:?}", body.currency),
            )
                .into_response();
        }
    };
    let meta = LedgerMeta {
        ledger_id,
        name: body.name,
        currency,
        created_at: Timestamp::from_millis(body.created_at_ms),
        updated_at: Timestamp::from_millis(body.updated_at_ms),
    };

    match state.service.store().save_ledger_meta(&meta).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /ledgers/{id}/sync` — Automerge delta sync endpoint.
async fn sync_ledger(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let ledger_id = match LedgerId::from_string(&id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    match state.service.asym_sync(ledger_id, body.to_vec()).await {
        Ok(Some(resp)) => (
            StatusCode::OK,
            [("content-type", "application/octet-stream")],
            resp,
        )
            .into_response(),
        Ok(None) => StatusCode::NO_CONTENT.into_response(),
        Err(unbill_model::UnbillError::Automerge(e)) => {
            (StatusCode::BAD_REQUEST, e).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_device_id(State(state): State<Arc<AppState>>) -> Response {
    (
        StatusCode::OK,
        [("content-type", "text/plain")],
        state.service.device_id().to_string(),
    )
        .into_response()
}

async fn load_device_meta(State(state): State<Arc<AppState>>, Path(key): Path<String>) -> Response {
    if !valid_device_key(&key) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    match state.service.store().load_device_meta(&key).await {
        Ok(Some(bytes)) => (
            StatusCode::OK,
            [("content-type", "application/octet-stream")],
            bytes,
        )
            .into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn save_device_meta(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
    body: Bytes,
) -> Response {
    if !valid_device_key(&key) {
        return StatusCode::BAD_REQUEST.into_response();
    }
    match state.service.store().save_device_meta(&key, &body).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /ledgers/{id}/invitations` — Create a join invitation for a ledger.
async fn create_invitation(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    let ledger_id = match LedgerId::from_string(&id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    match state.service.create_invitation(ledger_id).await {
        Ok(url) => (StatusCode::CREATED, Json(InvitationJson { url })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /ledgers/join` — Join a ledger hosted by another device.
async fn join_ledger(State(state): State<Arc<AppState>>, Json(body): Json<JoinBody>) -> Response {
    match state.service.join_ledger(&body.url, body.label).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /peers/{node_id}/sync` — Trigger a P2P Iroh sync with the given peer.
async fn sync_with_peer(
    State(state): State<Arc<AppState>>,
    Path(node_id): Path<String>,
) -> Response {
    let peer = NodeId::new(node_id);
    match state.service.trigger_peer_sync(peer).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `GET /events` — SSE stream of device events.
async fn stream_events(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let rx = state.service.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result.ok()? {
        ServiceEvent::LedgerUpdated { ledger_id } => {
            let data = serde_json::json!({
                "type": "LedgerUpdated",
                "ledger_id": ledger_id,
            })
            .to_string();
            Some(Ok::<Event, Infallible>(Event::default().data(data)))
        }
        _ => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn meta_to_json(m: LedgerMeta) -> MetaJson {
    MetaJson {
        ledger_id: m.ledger_id.to_string(),
        name: m.name,
        currency: m.currency.code().to_owned(),
        created_at_ms: m.created_at.as_millis(),
        updated_at_ms: m.updated_at.as_millis(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    const API_KEY: &str = "secret";

    async fn make_app(dir: &std::path::Path) -> Router {
        use unbill_device::UnbillDevice;
        use unbill_store_fs::FsStore;
        let store = Arc::new(FsStore::open(dir.to_path_buf()).unwrap());
        let device = UnbillDevice::open(store).await.unwrap();
        let state = Arc::new(AppState {
            service: device,
            api_key: API_KEY.to_owned(),
        });
        build_router(state)
    }

    async fn body_bytes(resp: axum::response::Response) -> Vec<u8> {
        resp.into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec()
    }

    fn auth_put(uri: &str, content_type: &str, body: Vec<u8>) -> Request<Body> {
        Request::builder()
            .method(Method::PUT)
            .uri(uri)
            .header("Authorization", format!("Bearer {API_KEY}"))
            .header("content-type", content_type)
            .body(Body::from(body))
            .unwrap()
    }

    fn auth_get(uri: &str) -> Request<Body> {
        Request::builder()
            .method(Method::GET)
            .uri(uri)
            .header("Authorization", format!("Bearer {API_KEY}"))
            .body(Body::empty())
            .unwrap()
    }

    fn auth_post(uri: &str, content_type: &str, body: Vec<u8>) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header("Authorization", format!("Bearer {API_KEY}"))
            .header("content-type", content_type)
            .body(Body::from(body))
            .unwrap()
    }

    // --- auth ---------------------------------------------------------------

    #[tokio::test]
    async fn test_missing_token_returns_401() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let req = Request::builder()
            .uri("/api/v1/ledgers")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_wrong_token_returns_401() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let req = Request::builder()
            .uri("/api/v1/ledgers")
            .header("Authorization", "Bearer wrong")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // --- list ledgers -------------------------------------------------------

    #[tokio::test]
    async fn test_list_ledgers_returns_empty_array_initially() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let resp = app.oneshot(auth_get("/api/v1/ledgers")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_bytes(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json, serde_json::json!([]));
    }

    // --- save + list ledger meta --------------------------------------------

    #[tokio::test]
    async fn test_save_and_list_ledger_meta_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;

        let meta = serde_json::json!({
            "ledger_id": "00000000000000000000000001",
            "name": "Groceries",
            "currency": "USD",
            "created_at_ms": 1000,
            "updated_at_ms": 2000
        });

        let resp = app
            .clone()
            .oneshot(auth_put(
                "/api/v1/ledgers/00000000000000000000000001/meta",
                "application/json",
                serde_json::to_vec(&meta).unwrap(),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        let resp = app.oneshot(auth_get("/api/v1/ledgers")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_bytes(resp).await;
        let list: Vec<serde_json::Value> = serde_json::from_slice(&body).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["name"], "Groceries");
    }

    // --- sync ---------------------------------------------------------------

    #[tokio::test]
    async fn test_sync_invalid_body_returns_400() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let resp = app
            .oneshot(auth_post(
                "/api/v1/ledgers/00000000000000000000000001/sync",
                "application/octet-stream",
                b"not a sync message".to_vec(),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_sync_converges_with_server() {
        use unbill_device::{LedgerStore as _, UnbillDevice};
        use unbill_model::{Currency, LedgerId, Timestamp};
        use unbill_store_fs::FsStore;

        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(FsStore::open(dir.path().to_path_buf()).unwrap());
        let device = UnbillDevice::open(Arc::clone(&store) as Arc<dyn unbill_device::LedgerStore>)
            .await
            .unwrap();
        let app = {
            let state = Arc::new(AppState {
                service: Arc::clone(&device),
                api_key: API_KEY.to_owned(),
            });
            build_router(state)
        };

        use unbill_storage::LedgerDoc;
        let ledger_id = LedgerId::from_u128(1);
        let id_str = ledger_id.to_string();
        let mut server_doc = LedgerDoc::new(
            ledger_id,
            "Groceries".to_string(),
            Currency::from_code("USD").unwrap(),
            Timestamp::from_millis(1000),
        )
        .unwrap();
        store.save_ledger(&id_str, &mut server_doc).await.unwrap();

        let mut client_doc = LedgerDoc::empty();
        let mut sync_state = automerge::sync::State::new();
        loop {
            let msg = match client_doc.generate_sync_message(&mut sync_state) {
                Some(m) => m,
                None => break,
            };
            let resp = app
                .clone()
                .oneshot(auth_post(
                    &format!("/api/v1/ledgers/{id_str}/sync"),
                    "application/octet-stream",
                    msg.encode(),
                ))
                .await
                .unwrap();
            if resp.status() == StatusCode::NO_CONTENT {
                continue;
            }
            assert_eq!(resp.status(), StatusCode::OK);
            let bytes = body_bytes(resp).await;
            let server_msg = automerge::sync::Message::decode(&bytes).unwrap();
            client_doc
                .receive_sync_message(&mut sync_state, server_msg)
                .unwrap();
        }

        assert_eq!(client_doc.get_ledger().unwrap().name, "Groceries");
    }

    // --- device meta --------------------------------------------------------

    #[tokio::test]
    async fn test_device_meta_save_and_load_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;

        let resp = app
            .clone()
            .oneshot(auth_put(
                "/api/v1/device/device_key.bin",
                "application/octet-stream",
                b"secret".to_vec(),
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NO_CONTENT);

        let resp = app
            .oneshot(auth_get("/api/v1/device/device_key.bin"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(body_bytes(resp).await, b"secret");
    }

    #[tokio::test]
    async fn test_device_meta_returns_404_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let resp = app
            .oneshot(auth_get("/api/v1/device/nonexistent.bin"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    // --- device id ----------------------------------------------------------

    #[tokio::test]
    async fn test_get_device_id_returns_node_id() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let resp = app.oneshot(auth_get("/api/v1/device/id")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = body_bytes(resp).await;
        assert!(!body.is_empty());
    }

    // --- path traversal -----------------------------------------------------

    #[tokio::test]
    async fn test_device_key_with_path_separator_returns_400() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let resp = app
            .oneshot(auth_get("/api/v1/device/../../etc/passwd"))
            .await
            .unwrap();
        assert!(resp.status() == StatusCode::BAD_REQUEST || resp.status() == StatusCode::NOT_FOUND);
    }
}
