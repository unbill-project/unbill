CREATE TABLE ledgers (
    id TEXT PRIMARY KEY NOT NULL,
    metadata BLOB,
    document BLOB
);
CREATE TABLE device_metadata (
    key TEXT PRIMARY KEY NOT NULL,
    value BLOB NOT NULL
);
