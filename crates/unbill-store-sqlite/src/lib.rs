//! Single-owner SQLite storage. Opening a store does not import flat-file data.
mod meta;
mod schema;

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use meta::MetaJson;
use rand::TryRng as _;
use schema::{device_identity, device_labels, ledgers, pending_invitations};
use std::collections::HashMap;
use std::{
    fs::File,
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
    // Kept alive by outstanding blocking operations as well as the store itself.
    _lock: File,
}

pub struct SqliteStore {
    database: Arc<Database>,
    events: broadcast::Sender<ServiceEvent>,
}

impl SqliteStore {
    /// Open `root/unbill.sqlite3`, applying embedded migrations.
    /// Fails with an I/O WouldBlock error if the directory is already in use.
    pub async fn open(root: PathBuf) -> Result<Self> {
        tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&root)?;
            let lock = File::options()
                .write(true)
                .create(true)
                .truncate(false)
                .open(root.join("unbill.lock"))?;
            lock.try_lock().map_err(|error| match error {
                std::fs::TryLockError::WouldBlock => std::io::Error::new(
                    std::io::ErrorKind::WouldBlock,
                    format!("data directory is already in use: {}", root.display()),
                ),
                std::fs::TryLockError::Error(error) => error,
            })?;
            let path = std::fs::canonicalize(root)?.join("unbill.sqlite3");
            let path = path
                .to_str()
                .ok_or_else(|| io_error("database path is not UTF-8"))?;
            let mut connection = SqliteConnection::establish(path).map_err(io_error)?;
            connection
                .run_pending_migrations(MIGRATIONS)
                .map_err(io_error)?;
            let (events, _) = broadcast::channel(256);
            Ok(Self {
                database: Arc::new(Database {
                    connection: Mutex::new(connection),
                    _lock: lock,
                }),
                events,
            })
        })
        .await
        .map_err(io_error)?
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

#[async_trait]
impl LedgerStore for SqliteStore {
    async fn save_ledger_meta(&self, meta: &LedgerMeta) -> Result<()> {
        let id = meta.ledger_id.to_string();
        let bytes = serde_json::to_vec(&MetaJson::from_meta(meta)).map_err(serialization)?;
        self.run(move |conn| {
            diesel::insert_into(ledgers::table)
                .values((ledgers::id.eq(id), ledgers::metadata.eq(&bytes)))
                .on_conflict(ledgers::id)
                .do_update()
                .set(ledgers::metadata.eq(&bytes))
                .execute(conn)
                .map_err(io_error)?;
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
        self.run(move |conn| {
            diesel::insert_into(ledgers::table)
                .values((ledgers::id.eq(&id), ledgers::document.eq(&bytes)))
                .on_conflict(ledgers::id)
                .do_update()
                .set(ledgers::document.eq(&bytes))
                .execute(conn)
                .map_err(io_error)?;
            // Send even if the async caller was cancelled while the write completed.
            let _ = events.send(ServiceEvent::LedgerUpdated { ledger_id: id });
            Ok(())
        })
        .await
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
    async fn save_invitation(&self, invitation: &Invitation) -> Result<()> {
        let row = InvitationRow::from(invitation);
        self.run(move |conn| {
            diesel::insert_into(pending_invitations::table)
                .values(&row)
                .on_conflict(pending_invitations::token)
                .do_update()
                .set(&row)
                .execute(conn)
                .map_err(io_error)?;
            Ok(())
        })
        .await
    }
    async fn consume_invitation(&self, token: &str) -> Result<Option<Invitation>> {
        let token = token.to_owned();
        self.run(move |conn| {
            conn.immediate_transaction::<_, diesel::result::Error, _>(|conn| {
                let row = pending_invitations::table
                    .find(&token)
                    .first::<InvitationRow>(conn)
                    .optional()?;
                let invitation = row
                    .map(InvitationRow::into_invitation)
                    .transpose()
                    .map_err(|e| diesel::result::Error::DeserializationError(Box::new(e)))?;
                diesel::delete(pending_invitations::table.find(&token)).execute(conn)?;
                Ok(invitation)
            })
            .map_err(io_error)
        })
        .await
    }
    async fn create_secret_key(&self) -> Result<()> {
        self.run(|conn| {
            let mut bytes = [0u8; 32];
            rand::rngs::SysRng
                .try_fill_bytes(&mut bytes)
                .map_err(io_error)?;
            diesel::insert_into(device_identity::table)
                .values((
                    device_identity::id.eq(1),
                    device_identity::secret_key.eq(bytes.as_slice()),
                ))
                .on_conflict(device_identity::id)
                .do_nothing()
                .execute(conn)
                .map_err(io_error)?;
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
