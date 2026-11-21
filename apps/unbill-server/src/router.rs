use std::convert::Infallible;
use std::sync::Arc;

use axum::{
    Json, Router,
    body::Bytes,
    extract::{Extension, Path, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::{self, Next},
    response::sse::{Event, KeepAlive, Sse},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::{BroadcastStream, WatchStream};
use tower_http::trace::TraceLayer;

use unbill_device::{ServiceEvent, UnbillDevice};
use unbill_model::{Currency, LedgerId, LedgerMeta, NodeId, Timestamp};

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

pub struct AppState {
    pub service: Arc<UnbillDevice>,
}

#[derive(Deserialize)]
struct Credentials {
    name: String,
    password: String,
}
#[derive(Deserialize)]
struct PasswordChange {
    old_password: String,
    new_password: String,
}
#[derive(Deserialize)]
struct ShareBody {
    account_name: String,
}
#[derive(Serialize)]
struct SessionJson {
    token: String,
    account_name: String,
}

// ---------------------------------------------------------------------------
// LedgerMeta JSON shape shared with storage backends and HttpAsymChannel.
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct MetaJson {
    #[serde(default)]
    pub archived: bool,
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
    label: unbill_model::DeviceLabel,
}

#[derive(Debug, Serialize)]
struct InvitationJson {
    url: String,
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

pub fn build_router(state: Arc<AppState>) -> Router {
    // sirno:witness:server-http-api:begin
    let protected = Router::new()
        .route("/ledgers", get(list_ledgers))
        .route("/ledgers/{id}/meta", put(save_ledger_meta))
        .route("/ledgers/{id}/archive", put(set_ledger_archived))
        .route("/ledgers/{id}/sync", post(sync_ledger))
        .route("/ledgers/{id}/invitations", post(create_invitation))
        .route("/ledgers/{id}/share", post(share_ledger))
        .route("/ledgers/join", post(join_ledger))
        .route("/accounts", get(list_accounts))
        .route("/accounts/me", get(get_account_name))
        .route("/accounts/logout", post(logout))
        .route("/accounts/password", put(change_password))
        .route("/peers/{node_id}/sync", post(sync_with_peer))
        .route("/events", get(stream_events))
        .route("/device/id", get(get_device_id))
        .route("/device/ble", get(get_ble_snapshot))
        .route("/build-info", get(get_build_info))
        .layer(middleware::from_fn_with_state(state.clone(), auth))
        .with_state(state.clone());

    Router::new()
        .nest("/api/v1", protected)
        .route("/api/v1/accounts/register", post(register))
        .route("/api/v1/accounts/login", post(login))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
    // sirno:witness:server-http-api:end
}

// ---------------------------------------------------------------------------
// Auth middleware
// ---------------------------------------------------------------------------

// sirno:witness:security-model:begin
async fn auth(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Response {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let Some(token) = token else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match state.service.session_account(token).await {
        Ok(name) => {
            req.extensions_mut().insert(name);
            next.run(req).await
        }
        Err(_) => StatusCode::UNAUTHORIZED.into_response(),
    }
}
// sirno:witness:security-model:end

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

// sirno:witness:server-http-api:begin
async fn register(State(state): State<Arc<AppState>>, Json(body): Json<Credentials>) -> Response {
    match state.service.register(&body.name, &body.password).await {
        Ok(()) => StatusCode::CREATED.into_response(),
        Err(unbill_model::UnbillError::Validation(e)) => {
            (StatusCode::BAD_REQUEST, e).into_response()
        }
        Err(e) => (StatusCode::CONFLICT, e.to_string()).into_response(),
    }
}

async fn login(State(state): State<Arc<AppState>>, Json(body): Json<Credentials>) -> Response {
    match state.service.login(&body.name, &body.password).await {
        Ok(token) => Json(SessionJson {
            token,
            account_name: body.name,
        })
        .into_response(),
        Err(_) => StatusCode::UNAUTHORIZED.into_response(),
    }
}

async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let Some(token) = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match state.service.logout(token).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn change_password(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(body): Json<PasswordChange>,
) -> Response {
    let Some(token) = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match state
        .service
        .change_password(token, &body.old_password, &body.new_password)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(unbill_model::UnbillError::NotAuthorized) => StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    }
}

async fn list_accounts(
    State(state): State<Arc<AppState>>,
    Extension(name): Extension<String>,
) -> Response {
    match state.service.account_view(&name).await {
        Ok(view) => match view.list_accounts().await {
            Ok(names) => Json(names).into_response(),
            Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
        },
        Err(_) => StatusCode::UNAUTHORIZED.into_response(),
    }
}

async fn get_account_name(Extension(name): Extension<String>) -> Json<String> {
    Json(name)
}

async fn share_ledger(
    State(state): State<Arc<AppState>>,
    Extension(name): Extension<String>,
    Path(id): Path<String>,
    Json(body): Json<ShareBody>,
) -> Response {
    let Ok(id) = LedgerId::from_string(&id) else {
        return StatusCode::BAD_REQUEST.into_response();
    };
    let Ok(view) = state.service.account_view(&name).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match view.share_ledger(id, &body.account_name).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(unbill_model::UnbillError::NotAuthorized) => StatusCode::FORBIDDEN.into_response(),
        Err(unbill_model::UnbillError::UserNotFound(_)) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn list_ledgers(
    State(state): State<Arc<AppState>>,
    Extension(name): Extension<String>,
) -> Response {
    let Ok(view) = state.service.account_view(&name).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match view.list_ledgers().await {
        Ok(metas) => {
            let json: Vec<MetaJson> = metas.into_iter().map(meta_to_json).collect();
            Json(json).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
struct ArchiveBody {
    archived: bool,
}

async fn set_ledger_archived(
    State(state): State<Arc<AppState>>,
    Extension(name): Extension<String>,
    Path(id): Path<String>,
    Json(body): Json<ArchiveBody>,
) -> Response {
    let ledger_id = match LedgerId::from_string(&id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    let Ok(view) = state.service.account_view(&name).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match view.set_ledger_archived(ledger_id, body.archived).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(unbill_model::UnbillError::Storage(unbill_model::StorageError::Io(e)))
            if e.kind() == std::io::ErrorKind::NotFound =>
        {
            (StatusCode::NOT_FOUND, e.to_string()).into_response()
        }
        Err(unbill_model::UnbillError::NotAuthorized) => StatusCode::FORBIDDEN.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn save_ledger_meta(
    State(state): State<Arc<AppState>>,
    Extension(name): Extension<String>,
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
        archived: false,
        ledger_id,
        name: body.name,
        currency,
        created_at: Timestamp::from_millis(body.created_at_ms),
        updated_at: Timestamp::from_millis(body.updated_at_ms),
    };

    let Ok(view) = state.service.account_view(&name).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match view.save_ledger_meta(&meta).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(unbill_model::UnbillError::NotAuthorized) => StatusCode::FORBIDDEN.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /ledgers/{id}/sync` — Automerge delta sync endpoint.
async fn sync_ledger(
    State(state): State<Arc<AppState>>,
    Extension(name): Extension<String>,
    Path(id): Path<String>,
    body: Bytes,
) -> Response {
    let ledger_id = match LedgerId::from_string(&id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    let Ok(view) = state.service.account_view(&name).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match view.asym_sync(ledger_id, body.to_vec()).await {
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
        Err(unbill_model::UnbillError::NotAuthorized) => StatusCode::FORBIDDEN.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn get_build_info() -> Json<unbill_build_info::BuildInfo> {
    Json(unbill_build_info::BuildInfo::from(
        crate::compiled_build_info(),
    ))
}

async fn get_ble_snapshot(State(state): State<Arc<AppState>>) -> Json<unbill_model::BleSnapshot> {
    Json(state.service.ble_snapshot())
}

async fn get_device_id(State(state): State<Arc<AppState>>) -> Response {
    (
        StatusCode::OK,
        [("content-type", "text/plain")],
        state.service.device_id().to_string(),
    )
        .into_response()
}

/// `POST /ledgers/{id}/invitations` — Create a join invitation for a ledger.
async fn create_invitation(
    State(state): State<Arc<AppState>>,
    Extension(name): Extension<String>,
    Path(id): Path<String>,
) -> Response {
    let ledger_id = match LedgerId::from_string(&id) {
        Ok(id) => id,
        Err(e) => return (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
    };
    let Ok(view) = state.service.account_view(&name).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match view.create_invitation(ledger_id).await {
        Ok(url) => (StatusCode::CREATED, Json(InvitationJson { url })).into_response(),
        Err(unbill_model::UnbillError::NotAuthorized) => StatusCode::FORBIDDEN.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /ledgers/join` — Join a ledger hosted by another device.
async fn join_ledger(
    State(state): State<Arc<AppState>>,
    Extension(name): Extension<String>,
    Json(body): Json<JoinBody>,
) -> Response {
    let Ok(view) = state.service.account_view(&name).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match view.join_ledger(&body.url, body.label).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `POST /peers/{node_id}/sync` — Trigger a P2P Iroh sync with the given peer.
async fn sync_with_peer(
    State(state): State<Arc<AppState>>,
    Extension(_name): Extension<String>,
    Path(node_id): Path<String>,
) -> Response {
    let peer = NodeId::new(node_id);
    match state.service.trigger_peer_sync(peer).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

/// `GET /events` — SSE stream of device events.
async fn stream_events(
    State(state): State<Arc<AppState>>,
    Extension(name): Extension<String>,
    headers: HeaderMap,
) -> Response {
    let Ok(view) = state.service.account_view(&name).await else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let Some(token) = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let token = token.to_owned();
    let rx = view.subscribe();
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
    let ble = WatchStream::new(state.service.subscribe_ble()).map(|_| {
        Ok::<Event, Infallible>(
            Event::default().data(serde_json::json!({"type": "BleSnapshotChanged"}).to_string()),
        )
    });
    let service = Arc::clone(&state.service);
    let authenticated = stream
        .merge(ble)
        .then(move |event| {
            let service = Arc::clone(&service);
            let token = token.clone();
            let name = name.clone();
            async move {
                if service.session_account(&token).await.ok().as_deref() == Some(name.as_str()) {
                    Some(event)
                } else {
                    None
                }
            }
        })
        .filter_map(|event| event);
    Sse::new(authenticated)
        .keep_alive(KeepAlive::default())
        .into_response()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn meta_to_json(m: LedgerMeta) -> MetaJson {
    MetaJson {
        archived: m.archived,
        ledger_id: m.ledger_id.to_string(),
        name: m.name,
        currency: m.currency.code().to_owned(),
        created_at_ms: m.created_at.as_millis(),
        updated_at_ms: m.updated_at.as_millis(),
    }
}
// sirno:witness:server-http-api:end

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

    const API_KEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    async fn create_test_session(store: &unbill_store_sqlite::SqliteStore) {
        use sha2::{Digest, Sha256};
        use unbill_storage::LedgerStore as _;
        store.create_account("test", "test-only").await.unwrap();
        let digest: String = Sha256::digest(API_KEY.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        store.create_session(&digest, "test").await.unwrap();
    }

    async fn make_app(dir: &std::path::Path) -> Router {
        use unbill_device::UnbillDevice;
        use unbill_store_sqlite::SqliteStore;
        let store = Arc::new(SqliteStore::open(dir.to_path_buf()).await.unwrap());
        create_test_session(&store).await;
        let device = UnbillDevice::open(store).await.unwrap();
        let state = Arc::new(AppState { service: device });
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

    #[tokio::test]
    async fn build_info_returns_server_metadata_and_requires_auth() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let resp = app
            .clone()
            .oneshot(auth_get("/api/v1/build-info"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let info: unbill_build_info::BuildInfo =
            serde_json::from_slice(&body_bytes(resp).await).unwrap();
        assert_eq!(
            info,
            unbill_build_info::BuildInfo::from(crate::compiled_build_info())
        );
        let request = Request::builder()
            .uri("/api/v1/build-info")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn ble_events_start_with_invalidation_and_require_auth() {
        use http_body_util::BodyExt;
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let response = app
            .clone()
            .oneshot(auth_get("/api/v1/events"))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let mut body = response.into_body();
        let frame = tokio::time::timeout(std::time::Duration::from_secs(2), body.frame())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let data = std::str::from_utf8(frame.data_ref().unwrap()).unwrap();
        assert!(data.contains("BleSnapshotChanged"), "{data}");
        let request = Request::builder()
            .uri("/api/v1/events")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }

    // --- auth ---------------------------------------------------------------

    #[tokio::test]
    async fn ble_snapshot_is_read_only_and_requires_auth() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let resp = app
            .clone()
            .oneshot(auth_get("/api/v1/device/ble"))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let snapshot: unbill_model::BleSnapshot =
            serde_json::from_slice(&body_bytes(resp).await).unwrap();
        assert_eq!(snapshot.state, unbill_model::BleState::Disabled);
        assert!(snapshot.peers.is_empty());
        let request = Request::builder()
            .uri("/api/v1/device/ble")
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }

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

    #[tokio::test]
    async fn account_token_cannot_select_another_accounts_ledgers() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let id = "00000000000000000000000001";
        let meta = serde_json::json!({
            "ledger_id": id, "name": "Private", "currency": "USD",
            "created_at_ms": 1000, "updated_at_ms": 1000
        });
        assert_eq!(
            app.clone()
                .oneshot(auth_put(
                    &format!("/api/v1/ledgers/{id}/meta"),
                    "application/json",
                    serde_json::to_vec(&meta).unwrap()
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT
        );

        let credentials = serde_json::json!({"name":"bob","password":"bob password"});
        let register = Request::builder()
            .method("POST")
            .uri("/api/v1/accounts/register")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&credentials).unwrap()))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(register).await.unwrap().status(),
            StatusCode::CREATED
        );
        let login = Request::builder()
            .method("POST")
            .uri("/api/v1/accounts/login")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_vec(&credentials).unwrap()))
            .unwrap();
        let response = app.clone().oneshot(login).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let session: serde_json::Value =
            serde_json::from_slice(&body_bytes(response).await).unwrap();
        let token = session["token"].as_str().unwrap();
        let list = Request::builder()
            .uri("/api/v1/ledgers?account_name=test")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(list).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body_bytes(response).await).unwrap(),
            serde_json::json!([])
        );
        let invite = Request::builder()
            .method("POST")
            .uri(format!("/api/v1/ledgers/{id}/invitations"))
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        assert_eq!(
            app.clone().oneshot(invite).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
        let share = Request::builder()
            .method("POST")
            .uri(format!("/api/v1/ledgers/{id}/share"))
            .header("authorization", format!("Bearer {API_KEY}"))
            .header("content-type", "application/json")
            .body(Body::from(br#"{"account_name":"bob"}"#.to_vec()))
            .unwrap();
        assert_eq!(
            app.clone().oneshot(share).await.unwrap().status(),
            StatusCode::NO_CONTENT
        );
        let list = Request::builder()
            .uri("/api/v1/ledgers")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(list).await.unwrap();
        let ledgers: serde_json::Value =
            serde_json::from_slice(&body_bytes(response).await).unwrap();
        assert_eq!(ledgers.as_array().unwrap().len(), 1);
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

    #[tokio::test]
    async fn archive_route_restores_and_rejects_missing_ledgers() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let route = "/api/v1/ledgers/00000000000000000000000001/archive";
        assert_eq!(
            app.clone()
                .oneshot(auth_put(
                    route,
                    "application/json",
                    br#"{"archived":true}"#.to_vec()
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
        let meta = br#"{"ledger_id":"00000000000000000000000001","name":"Trip","currency":"USD","created_at_ms":1000,"updated_at_ms":2000}"#;
        assert_eq!(
            app.clone()
                .oneshot(auth_put(
                    "/api/v1/ledgers/00000000000000000000000001/meta",
                    "application/json",
                    meta.to_vec()
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::NO_CONTENT
        );
        for archived in [true, false] {
            assert_eq!(
                app.clone()
                    .oneshot(auth_put(
                        route,
                        "application/json",
                        serde_json::to_vec(&serde_json::json!({"archived":archived})).unwrap()
                    ))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::NO_CONTENT
            );
            // Ordinary metadata saves must never overwrite this preference.
            app.clone()
                .oneshot(auth_put(
                    "/api/v1/ledgers/00000000000000000000000001/meta",
                    "application/json",
                    meta.to_vec(),
                ))
                .await
                .unwrap();
            let list: Vec<serde_json::Value> = serde_json::from_slice(
                &body_bytes(
                    app.clone()
                        .oneshot(auth_get("/api/v1/ledgers"))
                        .await
                        .unwrap(),
                )
                .await,
            )
            .unwrap();
            assert_eq!(list.first().unwrap()["archived"], archived);
        }
        let request = Request::builder()
            .method("PUT")
            .uri(route)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"archived":true}"#))
            .unwrap();
        assert_eq!(
            app.oneshot(request).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
    }

    // --- sync ---------------------------------------------------------------

    #[tokio::test]
    async fn test_sync_invalid_body_returns_400() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let meta = br#"{"ledger_id":"00000000000000000000000001","name":"Trip","currency":"USD","created_at_ms":1000,"updated_at_ms":2000}"#;
        app.clone()
            .oneshot(auth_put(
                "/api/v1/ledgers/00000000000000000000000001/meta",
                "application/json",
                meta.to_vec(),
            ))
            .await
            .unwrap();
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
        use unbill_device::UnbillDevice;
        use unbill_model::{Currency, LedgerId, Timestamp};
        use unbill_store_sqlite::SqliteStore;

        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(SqliteStore::open(dir.path().to_path_buf()).await.unwrap());
        create_test_session(&store).await;
        let device = UnbillDevice::open(Arc::clone(&store) as Arc<dyn unbill_storage::LedgerStore>)
            .await
            .unwrap();
        let app = {
            let state = Arc::new(AppState {
                service: Arc::clone(&device),
            });
            build_router(state)
        };

        use unbill_model::LedgerDoc;
        use unbill_storage::LedgerStore as _;
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
        store.grant_ledger("test", ledger_id).await.unwrap();

        let mut client_doc = LedgerDoc::empty();
        let mut sync_state = automerge::sync::State::new();
        while let Some(msg) = client_doc.generate_sync_message(&mut sync_state) {
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

    #[tokio::test]
    async fn local_device_label_routes_are_not_exposed() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        assert_eq!(
            app.clone()
                .oneshot(auth_get("/api/v1/device/labels"))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            app.oneshot(auth_put(
                "/api/v1/device/labels/peer",
                "application/json",
                br#""Local alias""#.to_vec()
            ))
            .await
            .unwrap()
            .status(),
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn join_requires_a_valid_joining_device_name() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        for body in [
            serde_json::json!({"url":"invalid"}),
            serde_json::json!({"url":"invalid", "label":null}),
            serde_json::json!({"url":"invalid", "label":"  "}),
        ] {
            let request = Request::builder()
                .method("POST")
                .uri("/api/v1/ledgers/join")
                .header("authorization", format!("Bearer {API_KEY}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert!(matches!(
                response.status(),
                StatusCode::UNPROCESSABLE_ENTITY | StatusCode::BAD_REQUEST
            ));
        }
    }

    #[tokio::test]
    async fn generic_metadata_routes_are_not_exposed() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        for key in ["device_key.bin", "pending_invitations.json", "arbitrary"] {
            let url = format!("/api/v1/device/{key}");
            assert_eq!(
                app.clone().oneshot(auth_get(&url)).await.unwrap().status(),
                StatusCode::NOT_FOUND
            );
            assert_eq!(
                app.clone()
                    .oneshot(auth_put(&url, "application/octet-stream", vec![]))
                    .await
                    .unwrap()
                    .status(),
                StatusCode::NOT_FOUND
            );
        }
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

    // --- events (SSE) -------------------------------------------------------

    #[tokio::test]
    async fn test_events_unauthenticated_returns_401() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let req = Request::builder()
            .uri("/api/v1/events")
            .body(Body::empty())
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_events_authenticated_returns_text_event_stream() {
        let dir = tempfile::tempdir().unwrap();
        let app = make_app(dir.path()).await;
        let resp = app.oneshot(auth_get("/api/v1/events")).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            content_type.contains("text/event-stream"),
            "expected text/event-stream, got {:?}",
            content_type
        );
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
