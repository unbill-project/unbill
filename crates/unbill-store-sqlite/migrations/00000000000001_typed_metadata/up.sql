CREATE TABLE device_identity (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    secret_key BLOB NOT NULL CHECK (typeof(secret_key) = 'blob' AND length(secret_key) = 32)
);
CREATE TABLE device_labels (
    node_id TEXT PRIMARY KEY NOT NULL,
    label TEXT NOT NULL
);
CREATE TABLE pending_invitations (
    token TEXT PRIMARY KEY NOT NULL CHECK (length(token) = 64 AND token NOT GLOB '*[^0-9a-f]*'),
    ledger_id TEXT NOT NULL CHECK (
        length(ledger_id) = 26
        AND upper(ledger_id) NOT GLOB '*[^0123456789ABCDEFGHJKMNPQRSTVWXYZ]*'
        AND substr(ledger_id, 1, 1) BETWEEN '0' AND '7'
    ),
    created_by_device TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    expires_at_ms BIGINT NOT NULL
);

-- Reject unknown or malformed legacy metadata instead of silently losing it.
CREATE TABLE metadata_migration_check (valid INTEGER NOT NULL CHECK (valid = 1));
INSERT INTO metadata_migration_check
SELECT CASE WHEN count(*) = 0 THEN 1 ELSE 0 END FROM device_metadata
WHERE key NOT IN ('device_key.bin', 'device_labels.json', 'pending_invitations.json');
INSERT INTO metadata_migration_check
SELECT json_type(CAST(value AS TEXT)) = 'object' FROM device_metadata
WHERE key IN ('device_labels.json', 'pending_invitations.json');
INSERT INTO metadata_migration_check
SELECT j.type = 'text' FROM device_metadata AS m, json_each(CAST(m.value AS TEXT)) AS j
WHERE m.key = 'device_labels.json';
INSERT INTO metadata_migration_check
SELECT j.type = 'object'
    AND json_extract(j.value, '$.token') = j.key
    AND json_type(j.value, '$.token') = 'text'
    AND json_type(j.value, '$.ledger_id') = 'text'
    AND json_type(j.value, '$.created_by_device') = 'text'
    AND json_type(j.value, '$.created_at') = 'integer'
    AND json_type(j.value, '$.expires_at') = 'integer'
FROM device_metadata AS m, json_each(CAST(m.value AS TEXT)) AS j
WHERE m.key = 'pending_invitations.json';

INSERT INTO device_identity (id, secret_key)
SELECT 1, value FROM device_metadata WHERE key = 'device_key.bin';
INSERT INTO device_labels (node_id, label)
SELECT j.key, j.value FROM device_metadata AS m, json_each(CAST(m.value AS TEXT)) AS j
WHERE m.key = 'device_labels.json';
INSERT INTO pending_invitations (token, ledger_id, created_by_device, created_at_ms, expires_at_ms)
SELECT j.key, json_extract(j.value, '$.ledger_id'), json_extract(j.value, '$.created_by_device'),
    json_extract(j.value, '$.created_at'), json_extract(j.value, '$.expires_at')
FROM device_metadata AS m, json_each(CAST(m.value AS TEXT)) AS j
WHERE m.key = 'pending_invitations.json';
DROP TABLE metadata_migration_check;
DROP TABLE device_metadata;
