CREATE TABLE device_metadata (key TEXT PRIMARY KEY NOT NULL, value BLOB NOT NULL);
INSERT INTO device_metadata SELECT 'device_key.bin', secret_key FROM device_identity;
INSERT INTO device_metadata SELECT 'device_labels.json', CAST(json_group_object(node_id, label) AS BLOB) FROM device_labels;
INSERT INTO device_metadata
SELECT 'pending_invitations.json', CAST(json_group_object(token, json_object(
    'token', token, 'ledger_id', ledger_id, 'created_by_device', created_by_device,
    'created_at', created_at_ms, 'expires_at', expires_at_ms)) AS BLOB) FROM pending_invitations;
DROP TABLE pending_invitations;
DROP TABLE device_labels;
DROP TABLE device_identity;
