//! Multi-process SQLite storage. Opening a store does not import flat-file data.
mod meta;
mod notifications;
mod schema;
mod transaction;

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use meta::MetaJson;
use rand::TryRng as _;
use schema::{device_identity, device_labels, ledgers, pending_invitations};
use std::collections::HashMap;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use unbill_event::ServiceEvent;
use unbill_model::{Invitation, LedgerId, Timestamp};
use unbill_model::{LedgerDoc, LedgerMeta, NodeId, SecretKey, StorageError};
use unbill_storage::{LedgerStore, StorageResult as Result};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

fn io_error(error: impl std::fmt::Display) -> StorageError {
    std::io::Error::other(error.to_string()).into()
}
fn serialization(error: impl std::fmt::Display) -> StorageError {
    StorageError::Serialization(error.to_string())
}

// sirno:witness:sqlite-store:begin
struct Database {
    connection: Mutex<SqliteConnection>,
}

pub struct SqliteStore {
    database: Arc<Database>,
    events: broadcast::Sender<ServiceEvent>,
    watcher: tokio::task::JoinHandle<()>,
}

impl SqliteStore {
    /// Open `root/unbill.sqlite3`, applying embedded migrations.
    /// Uses SQLite database locking without acquiring `unbill.lock`.
    pub async fn open(root: PathBuf) -> Result<Self> {
        let (database, observer, cursor) = tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&root)?;
            let path = std::fs::canonicalize(root)?.join("unbill.sqlite3");
            let path = path
                .to_str()
                .ok_or_else(|| io_error("database path is not UTF-8"))?;
            let mut connection = SqliteConnection::establish(path).map_err(io_error)?;
            transaction::configure(&mut connection)?;
            transaction::write(&mut connection, |conn| {
                conn.run_pending_migrations(MIGRATIONS).map_err(io_error)?;
                Ok(())
            })?;
            let mut observer = SqliteConnection::establish(path).map_err(io_error)?;
            transaction::configure(&mut observer)?;
            let cursor = notifications::clock(&mut observer)?;
            Ok::<_, StorageError>((
                Arc::new(Database {
                    connection: Mutex::new(connection),
                }),
                observer,
                cursor,
            ))
        })
        .await
        .map_err(io_error)??;
        let (events, _) = broadcast::channel(256);
        let watcher = notifications::spawn(observer, events.clone(), cursor);
        Ok(Self {
            database,
            events,
            watcher,
        })
    }

    async fn run<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut SqliteConnection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let database = Arc::clone(&self.database);
        tokio::task::spawn_blocking(move || {
            let mut connection = database.connection.lock().map_err(io_error)?;
            operation(&mut connection)
        })
        .await
        .map_err(io_error)?
    }
}

impl Drop for SqliteStore {
    fn drop(&mut self) {
        self.watcher.abort();
    }
}

#[async_trait]
impl LedgerStore for SqliteStore {
    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        let mut meta = meta.clone();
        let events = self.events.clone();
        self.run(move |conn| {
            let id = meta.ledger_id.to_string();
            transaction::write(conn, |conn| {
                let (old_meta, document) = stored(conn, &id)?;
                if let Some(bytes) = document {
                    let doc = LedgerDoc::from_bytes(&bytes).map_err(serialization)?;
                    meta = metadata(&doc, meta.updated_at)?;
                }
                if let Some(old) = old_meta
                    && old.updated_at > meta.updated_at
                {
                    meta.updated_at = old.updated_at;
                }
                let bytes =
                    serde_json::to_vec(&MetaJson::from_meta(&meta)).map_err(serialization)?;
                diesel::insert_into(ledgers::table)
                    .values((ledgers::id.eq(&id), ledgers::metadata.eq(&bytes)))
                    .on_conflict(ledgers::id)
                    .do_update()
                    .set(ledgers::metadata.eq(&bytes))
                    .execute(conn)
                    .map_err(io_error)?;
                Ok(())
            })?;
            let _ = events.send(ServiceEvent::LedgerUpdated { ledger_id: id });
            Ok(())
        })
        .await
    }

    async fn list_ledgers(&self) -> Result<Vec<LedgerMeta>> {
        self.run(|conn| {
            let rows = ledgers::table
                .select(ledgers::metadata)
                .filter(ledgers::metadata.is_not_null())
                .order(ledgers::id)
                .load::<Option<Vec<u8>>>(conn)
                .map_err(io_error)?;
            rows.into_iter()
                .flatten()
                .map(|bytes| {
                    serde_json::from_slice::<MetaJson>(&bytes)
                        .map_err(serialization)?
                        .into_ledger_meta()
                        .map_err(serialization)
                })
                .collect()
        })
        .await
    }

    async fn load_ledger(&self, ledger_id: &str) -> Result<Option<LedgerDoc>> {
        let id = ledger_id.to_owned();
        let bytes = self
            .run(move |conn| {
                ledgers::table
                    .find(id)
                    .select(ledgers::document)
                    .first::<Option<Vec<u8>>>(conn)
                    .optional()
                    .map(Option::flatten)
                    .map_err(io_error)
            })
            .await?;
        bytes
            .map(|bytes| LedgerDoc::from_bytes(&bytes).map_err(serialization))
            .transpose()
    }

    async fn save_ledger(&self, ledger_id: &str, doc: &mut LedgerDoc) -> Result<()> {
        let id = ledger_id.to_owned();
        let bytes = doc.save();
        let events = self.events.clone();
        let merged = self
            .run(move |conn| {
                let merged = transaction::write(conn, |conn| {
                    let mut incoming = LedgerDoc::from_bytes(&bytes).map_err(serialization)?;
                    let incoming_meta =
                        metadata(&incoming, Timestamp::now().map_err(std::io::Error::other)?)?;
                    if incoming_meta.ledger_id.to_string() != id {
                        return Err(serialization(
                            "document ledger ID does not match storage key",
                        ));
                    }
                    let (old_meta, current) = stored(conn, &id)?;
                    let mut merged = if let Some(current) = current {
                        let mut current = LedgerDoc::from_bytes(&current).map_err(serialization)?;
                        let current_meta = metadata(&current, incoming_meta.updated_at)?;
                        if current_meta.ledger_id != incoming_meta.ledger_id
                            || current_meta.name != incoming_meta.name
                            || current_meta.currency != incoming_meta.currency
                            || current_meta.created_at != incoming_meta.created_at
                        {
                            return Err(serialization("incompatible immutable ledger fields"));
                        }
                        current.merge(&mut incoming).map_err(serialization)?;
                        current
                    } else {
                        incoming
                    };
                    let mut meta =
                        metadata(&merged, Timestamp::now().map_err(std::io::Error::other)?)?;
                    if let Some(old) = old_meta
                        && old.updated_at > meta.updated_at
                    {
                        meta.updated_at = old.updated_at;
                    }
                    let document = merged.save();
                    let metadata =
                        serde_json::to_vec(&MetaJson::from_meta(&meta)).map_err(serialization)?;
                    diesel::insert_into(ledgers::table)
                        .values((
                            ledgers::id.eq(&id),
                            ledgers::metadata.eq(&metadata),
                            ledgers::document.eq(&document),
                        ))
                        .on_conflict(ledgers::id)
                        .do_update()
                        .set((
                            ledgers::metadata.eq(&metadata),
                            ledgers::document.eq(&document),
                        ))
                        .execute(conn)
                        .map_err(io_error)?;
                    Ok(merged)
                })?;
                let _ = events.send(ServiceEvent::LedgerUpdated { ledger_id: id });
                Ok(merged)
            })
            .await?;
        *doc = merged;
        Ok(())
    }

    fn subscribe(&self) -> broadcast::Receiver<ServiceEvent> {
        self.events.subscribe()
    }

    async fn list_device_labels(&self) -> Result<HashMap<String, String>> {
        self.run(|conn| {
            device_labels::table
                .load::<(String, String)>(conn)
                .map(|rows| rows.into_iter().collect())
                .map_err(io_error)
        })
        .await
    }
    async fn set_device_label(&self, node_id: &NodeId, label: Option<&str>) -> Result<()> {
        let node = node_id.to_string();
        let label = label.map(str::to_owned);
        let events = self.events.clone();
        self.run(move |conn| {
            if let Some(label) = label {
                diesel::insert_into(device_labels::table)
                    .values((
                        device_labels::node_id.eq(node),
                        device_labels::label.eq(&label),
                    ))
                    .on_conflict(device_labels::node_id)
                    .do_update()
                    .set(device_labels::label.eq(&label))
                    .execute(conn)
                    .map_err(io_error)?;
            } else {
                diesel::delete(device_labels::table.find(node))
                    .execute(conn)
                    .map_err(io_error)?;
            }
            let _ = events.send(ServiceEvent::DeviceLabelsUpdated);
            Ok(())
        })
        .await
    }
    async fn list_pending_invitations(&self) -> Result<Vec<Invitation>> {
        self.run(|conn| {
            pending_invitations::table
                .load::<InvitationRow>(conn)
                .map_err(io_error)?
                .into_iter()
                .map(InvitationRow::into_invitation)
                .collect()
        })
        .await
    }
    async fn create_invitation(
        &self,
        ledger_id: LedgerId,
        created_by_device: &NodeId,
        created_at: Timestamp,
        expires_at: Timestamp,
    ) -> Result<Invitation> {
        let invitation = Invitation {
            token: unbill_model::InviteToken::generate().map_err(std::io::Error::from)?,
            ledger_id,
            created_by_device: created_by_device.clone(),
            created_at,
            expires_at,
        };
        let row = InvitationRow::from(&invitation);
        let events = self.events.clone();
        self.run(move |conn| {
            diesel::insert_into(pending_invitations::table)
                .values(&row)
                .execute(conn)
                .map_err(io_error)?;
            let _ = events.send(ServiceEvent::PendingInvitationsUpdated);
            Ok(invitation)
        })
        .await
    }
    async fn consume_invitation(&self, token: &str) -> Result<Option<Invitation>> {
        let token = token.to_owned();
        let events = self.events.clone();
        self.run(move |conn| {
            let invitation = transaction::write(conn, |conn| {
                let row = pending_invitations::table
                    .find(&token)
                    .first::<InvitationRow>(conn)
                    .optional()
                    .map_err(io_error)?;
                let invitation = row.map(InvitationRow::into_invitation).transpose()?;
                diesel::delete(pending_invitations::table.find(&token))
                    .execute(conn)
                    .map_err(io_error)?;
                Ok(invitation)
            })?;
            if invitation.is_some() {
                let _ = events.send(ServiceEvent::PendingInvitationsUpdated);
            }
            Ok(invitation)
        })
        .await
    }
    async fn create_secret_key(&self) -> Result<()> {
        let events = self.events.clone();
        self.run(move |conn| {
            let mut bytes = [0u8; 32];
            rand::rngs::SysRng
                .try_fill_bytes(&mut bytes)
                .map_err(io_error)?;
            let inserted = diesel::insert_into(device_identity::table)
                .values((
                    device_identity::id.eq(1),
                    device_identity::secret_key.eq(bytes.as_slice()),
                ))
                .on_conflict(device_identity::id)
                .do_nothing()
                .execute(conn)
                .map_err(io_error)?;
            if inserted != 0 {
                let _ = events.send(ServiceEvent::DeviceIdentityInitialized);
            }
            Ok(())
        })
        .await
    }
    async fn is_device_initialized(&self) -> Result<bool> {
        self.run(|conn| {
            diesel::select(diesel::dsl::exists(device_identity::table.find(1)))
                .get_result(conn)
                .map_err(io_error)
        })
        .await
    }

    async fn get_device_id(&self) -> Result<NodeId> {
        let secret = self.get_secret_key().await?;
        Ok(NodeId::new(
            iroh::SecretKey::from(*secret.as_bytes())
                .public()
                .to_string(),
        ))
    }

    async fn get_secret_key(&self) -> Result<SecretKey> {
        let bytes = self
            .run(|conn| {
                device_identity::table
                    .find(1)
                    .select(device_identity::secret_key)
                    .first::<Vec<u8>>(conn)
                    .optional()
                    .map_err(io_error)
            })
            .await?
            .ok_or_else(|| serialization("device not initialized"))?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| serialization("device_key.bin: wrong length"))?;
        Ok(SecretKey::from_bytes(bytes))
    }
}
// sirno:witness:sqlite-store:end

#[derive(Queryable, Insertable, AsChangeset)]
#[diesel(table_name = pending_invitations)]
struct InvitationRow {
    token: String,
    ledger_id: String,
    created_by_device: String,
    created_at_ms: i64,
    expires_at_ms: i64,
}
impl From<&Invitation> for InvitationRow {
    fn from(inv: &Invitation) -> Self {
        Self {
            token: inv.token.to_string(),
            ledger_id: inv.ledger_id.to_string(),
            created_by_device: inv.created_by_device.to_string(),
            created_at_ms: inv.created_at.as_millis(),
            expires_at_ms: inv.expires_at.as_millis(),
        }
    }
}
impl InvitationRow {
    fn into_invitation(self) -> Result<Invitation> {
        Ok(Invitation {
            token: self.token.parse().map_err(serialization)?,
            ledger_id: LedgerId::from_string(&self.ledger_id).map_err(serialization)?,
            created_by_device: NodeId::new(self.created_by_device),
            created_at: Timestamp::from_millis(self.created_at_ms),
            expires_at: Timestamp::from_millis(self.expires_at_ms),
        })
    }
}

fn metadata(doc: &LedgerDoc, updated_at: Timestamp) -> Result<LedgerMeta> {
    let ledger = doc.get_ledger().map_err(serialization)?;
    Ok(LedgerMeta {
        ledger_id: ledger.ledger_id,
        name: ledger.name,
        currency: ledger.currency,
        created_at: ledger.created_at,
        updated_at,
    })
}
fn stored(conn: &mut SqliteConnection, id: &str) -> Result<(Option<LedgerMeta>, Option<Vec<u8>>)> {
    let row = ledgers::table
        .find(id)
        .select((ledgers::metadata, ledgers::document))
        .first::<(Option<Vec<u8>>, Option<Vec<u8>>)>(conn)
        .optional()
        .map_err(io_error)?;
    let (meta, doc) = row.unwrap_or_default();
    let meta = meta
        .map(|bytes| {
            serde_json::from_slice::<MetaJson>(&bytes)
                .map_err(serialization)?
                .into_ledger_meta()
                .map_err(serialization)
        })
        .transpose()?;
    Ok((meta, doc))
}
