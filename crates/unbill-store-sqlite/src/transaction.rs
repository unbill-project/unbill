use crate::io_error;
use diesel::{SqliteConnection, connection::SimpleConnection};
use unbill_model::StorageError;
use unbill_storage::StorageResult as Result;

// sirno:witness:sqlite-store:begin
struct TransactionError(StorageError);
impl From<diesel::result::Error> for TransactionError {
    fn from(error: diesel::result::Error) -> Self {
        Self(io_error(error))
    }
}

pub(crate) fn write<T>(
    conn: &mut SqliteConnection,
    operation: impl FnOnce(&mut SqliteConnection) -> Result<T>,
) -> Result<T> {
    conn.immediate_transaction::<_, TransactionError, _>(|conn| {
        operation(conn).map_err(TransactionError)
    })
    .map_err(|error| error.0)
}

pub(crate) fn configure(conn: &mut SqliteConnection) -> Result<()> {
    conn.batch_execute("PRAGMA busy_timeout = 5000;")
        .map_err(io_error)?;
    // Changing journal mode can return BUSY immediately during concurrent first
    // opens, even with a busy handler. Retry only that transient startup error.
    let retry_started = std::time::Instant::now();
    loop {
        match conn.batch_execute("PRAGMA journal_mode = WAL;") {
            Ok(()) => break,
            Err(diesel::result::Error::DatabaseError(_, ref info))
                if info.message() == "database is locked"
                    && retry_started.elapsed() < std::time::Duration::from_secs(5) =>
            {
                std::thread::sleep(std::time::Duration::from_millis(25));
            }
            Err(error) => return Err(io_error(error)),
        }
    }
    conn.batch_execute("PRAGMA synchronous = FULL;")
        .map_err(io_error)?;
    Ok(())
}
// sirno:witness:sqlite-store:end
