use crate::{
    io_error,
    schema::{ledgers, storage_revisions},
};
use diesel::prelude::*;
use std::sync::{Arc, Mutex};
use tokio::{sync::broadcast, task::JoinHandle};
use unbill_event::ServiceEvent;
use unbill_storage::StorageResult as Result;

// sirno:witness:sqlite-store:begin
pub(crate) fn clock(conn: &mut SqliteConnection) -> Result<i64> {
    storage_revisions::table
        .find(1)
        .select(storage_revisions::clock)
        .first(conn)
        .map_err(io_error)
}

// A single read transaction fixes both the high-water mark and the changed rows.
fn changes(conn: &mut SqliteConnection, after: i64) -> Result<(i64, Vec<ServiceEvent>)> {
    conn.transaction::<_, diesel::result::Error, _>(|conn| {
        let (_, clock, identity, labels, invitations): (i32, i64, i64, i64, i64) =
            storage_revisions::table.find(1).first(conn)?;
        let ids = ledgers::table
            .filter(ledgers::revision.gt(after))
            .filter(ledgers::revision.le(clock))
            .select(ledgers::id)
            .load::<String>(conn)?;
        let mut events: Vec<_> = ids
            .into_iter()
            .map(|ledger_id| ServiceEvent::LedgerUpdated { ledger_id })
            .collect();
        if identity > after {
            events.push(ServiceEvent::DeviceIdentityInitialized);
        }
        if labels > after {
            events.push(ServiceEvent::DeviceLabelsUpdated);
        }
        if invitations > after {
            events.push(ServiceEvent::PendingInvitationsUpdated);
        }
        Ok((clock, events))
    })
    .map_err(io_error)
}

pub(crate) fn spawn(
    connection: SqliteConnection,
    events: broadcast::Sender<ServiceEvent>,
    mut cursor: i64,
) -> JoinHandle<()> {
    let connection = Arc::new(Mutex::new(connection));
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            let connection = Arc::clone(&connection);
            let result = tokio::task::spawn_blocking(move || {
                let mut connection = connection.lock().map_err(io_error)?;
                changes(&mut connection, cursor)
            })
            .await;
            match result {
                Ok(Ok((next, changes))) => {
                    for event in changes {
                        let _ = events.send(event);
                    }
                    cursor = next;
                }
                Ok(Err(error)) => {
                    tracing::warn!(%error, "SQLite notification poll failed; will retry")
                }
                Err(error) => {
                    tracing::warn!(%error, "SQLite notification worker failed; will retry")
                }
            }
        }
    })
}

// sirno:witness:sqlite-store:end

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SqliteStore, transaction};
    use diesel::connection::SimpleConnection;
    use std::time::Duration;
    use unbill_storage::LedgerStore;

    #[tokio::test]
    async fn failed_poll_retries_without_skipping_committed_changes() {
        let dir = tempfile::tempdir().unwrap();
        let store = SqliteStore::open(dir.path().into()).await.unwrap();
        let mut events = store.subscribe();
        let mut conn =
            SqliteConnection::establish(dir.path().join("unbill.sqlite3").to_str().unwrap())
                .unwrap();
        transaction::configure(&mut conn).unwrap();
        // Make the ledger query fail after the revision clock can be read.
        // The label commit must remain pending until a complete poll succeeds.
        conn.batch_execute("BEGIN IMMEDIATE; ALTER TABLE ledgers RENAME TO unavailable_ledgers; INSERT INTO device_labels (node_id, label) VALUES ('peer', 'label'); COMMIT;").unwrap();
        tokio::time::sleep(Duration::from_millis(1100)).await;
        assert!(events.try_recv().is_err());
        conn.batch_execute("ALTER TABLE unavailable_ledgers RENAME TO ledgers;")
            .unwrap();
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(2), events.recv())
                .await
                .unwrap()
                .unwrap(),
            ServiceEvent::DeviceLabelsUpdated
        ));
    }
}
