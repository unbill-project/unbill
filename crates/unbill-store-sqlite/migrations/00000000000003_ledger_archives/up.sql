CREATE TABLE ledger_archives (
    ledger_id TEXT PRIMARY KEY NOT NULL REFERENCES ledgers(id) ON DELETE CASCADE,
    archived BOOLEAN NOT NULL DEFAULT 0 CHECK (archived IN (0, 1))
);
CREATE TRIGGER ledger_archive_insert_revision AFTER INSERT ON ledger_archives BEGIN
    UPDATE storage_revisions SET clock = clock + 1 WHERE id = 1;
    UPDATE ledgers SET revision = (SELECT clock FROM storage_revisions WHERE id = 1) WHERE id = NEW.ledger_id;
END;
CREATE TRIGGER ledger_archive_update_revision AFTER UPDATE OF archived ON ledger_archives BEGIN
    UPDATE storage_revisions SET clock = clock + 1 WHERE id = 1;
    UPDATE ledgers SET revision = (SELECT clock FROM storage_revisions WHERE id = 1) WHERE id = NEW.ledger_id;
END;
CREATE TRIGGER ledger_archive_delete_revision AFTER DELETE ON ledger_archives BEGIN
    UPDATE storage_revisions SET clock = clock + 1 WHERE id = 1;
    UPDATE ledgers SET revision = (SELECT clock FROM storage_revisions WHERE id = 1) WHERE id = OLD.ledger_id;
END;
