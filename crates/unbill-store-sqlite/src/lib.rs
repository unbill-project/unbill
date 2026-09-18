//! Single-owner SQLite storage. Opening a store does not import flat-file data.
mod meta;
mod schema;

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use meta::MetaJson;
use rand::TryRng as _;
use schema::{device_metadata, ledgers};
use std::{
    fs::File,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::sync::broadcast;
use unbill_event::ServiceEvent;
use unbill_model::{LedgerDoc, LedgerMeta, NodeId, SecretKey, StorageError};
use unbill_storage::{LedgerStore, StorageResult as Result};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");
const SECRET_KEY: &str = "device_key.bin";

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

    async fn load_device_meta(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let key = key.to_owned();
        self.run(move |conn| {
            device_metadata::table
                .find(key)
                .select(device_metadata::value)
                .first(conn)
                .optional()
                .map_err(io_error)
        })
        .await
    }

    async fn save_device_meta(&self, key: &str, value: &[u8]) -> Result<()> {
        let key = key.to_owned();
        let value = value.to_owned();
        self.run(move |conn| {
            diesel::insert_into(device_metadata::table)
                .values((
                    device_metadata::key.eq(key),
                    device_metadata::value.eq(&value),
                ))
                .on_conflict(device_metadata::key)
                .do_update()
                .set(device_metadata::value.eq(&value))
                .execute(conn)
                .map_err(io_error)?;
            Ok(())
        })
        .await
    }

    async fn create_secret_key(&self) -> Result<()> {
        self.run(|conn| {
            let mut bytes = [0u8; 32];
            rand::rngs::SysRng
                .try_fill_bytes(&mut bytes)
                .map_err(io_error)?;
            diesel::insert_into(device_metadata::table)
                .values((
                    device_metadata::key.eq(SECRET_KEY),
                    device_metadata::value.eq(bytes.as_slice()),
                ))
                .on_conflict(device_metadata::key)
                .do_nothing()
                .execute(conn)
                .map_err(io_error)?;
            Ok(())
        })
        .await
    }

    async fn is_device_initialized(&self) -> Result<bool> {
        Ok(self.load_device_meta(SECRET_KEY).await?.is_some())
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
            .load_device_meta(SECRET_KEY)
            .await?
            .ok_or_else(|| serialization("device not initialized"))?;
        let bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| serialization("device_key.bin: wrong length"))?;
        Ok(SecretKey::from_bytes(bytes))
    }
}
// sirno:witness:sqlite-store:end
