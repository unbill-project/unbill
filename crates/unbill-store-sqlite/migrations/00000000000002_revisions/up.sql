ALTER TABLE ledgers ADD COLUMN revision BIGINT NOT NULL DEFAULT 0;
CREATE TABLE storage_revisions (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 1),
    clock BIGINT NOT NULL DEFAULT 0,
    identity_revision BIGINT NOT NULL DEFAULT 0,
    labels_revision BIGINT NOT NULL DEFAULT 0,
    invitations_revision BIGINT NOT NULL DEFAULT 0
);
INSERT INTO storage_revisions (id) VALUES (1);
CREATE TRIGGER ledger_revision_insert AFTER INSERT ON ledgers BEGIN
    UPDATE storage_revisions SET clock = clock + 1 WHERE id = 1;
    UPDATE ledgers SET revision = (SELECT clock FROM storage_revisions WHERE id = 1) WHERE id = NEW.id;
END;
CREATE TRIGGER ledger_revision_update AFTER UPDATE OF metadata, document ON ledgers BEGIN
    UPDATE storage_revisions SET clock = clock + 1 WHERE id = 1;
    UPDATE ledgers SET revision = (SELECT clock FROM storage_revisions WHERE id = 1) WHERE id = NEW.id;
END;
CREATE TRIGGER device_identity_insert_revision AFTER INSERT ON device_identity BEGIN
    UPDATE storage_revisions SET clock = clock + 1, identity_revision = clock + 1 WHERE id = 1;
END;
CREATE TRIGGER device_identity_update_revision AFTER UPDATE ON device_identity BEGIN
    UPDATE storage_revisions SET clock = clock + 1, identity_revision = clock + 1 WHERE id = 1;
END;
CREATE TRIGGER device_identity_delete_revision AFTER DELETE ON device_identity BEGIN
    UPDATE storage_revisions SET clock = clock + 1, identity_revision = clock + 1 WHERE id = 1;
END;
CREATE TRIGGER device_labels_insert_revision AFTER INSERT ON device_labels BEGIN
    UPDATE storage_revisions SET clock = clock + 1, labels_revision = clock + 1 WHERE id = 1;
END;
CREATE TRIGGER device_labels_update_revision AFTER UPDATE ON device_labels BEGIN
    UPDATE storage_revisions SET clock = clock + 1, labels_revision = clock + 1 WHERE id = 1;
END;
CREATE TRIGGER device_labels_delete_revision AFTER DELETE ON device_labels BEGIN
    UPDATE storage_revisions SET clock = clock + 1, labels_revision = clock + 1 WHERE id = 1;
END;
CREATE TRIGGER pending_invitations_insert_revision AFTER INSERT ON pending_invitations BEGIN
    UPDATE storage_revisions SET clock = clock + 1, invitations_revision = clock + 1 WHERE id = 1;
END;
CREATE TRIGGER pending_invitations_update_revision AFTER UPDATE ON pending_invitations BEGIN
    UPDATE storage_revisions SET clock = clock + 1, invitations_revision = clock + 1 WHERE id = 1;
END;
CREATE TRIGGER pending_invitations_delete_revision AFTER DELETE ON pending_invitations BEGIN
    UPDATE storage_revisions SET clock = clock + 1, invitations_revision = clock + 1 WHERE id = 1;
END;
