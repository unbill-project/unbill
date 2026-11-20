CREATE TABLE accounts (
    name TEXT PRIMARY KEY NOT NULL,
    password_hash TEXT NOT NULL
);
CREATE TABLE account_ledgers (
    account_name TEXT NOT NULL REFERENCES accounts(name) ON DELETE CASCADE,
    ledger_id TEXT NOT NULL REFERENCES ledgers(id) ON DELETE CASCADE,
    PRIMARY KEY (account_name, ledger_id)
);
CREATE TABLE account_sessions (
    token_hash TEXT PRIMARY KEY NOT NULL,
    account_name TEXT NOT NULL REFERENCES accounts(name) ON DELETE CASCADE
);
CREATE INDEX account_ledgers_by_ledger ON account_ledgers(ledger_id);
CREATE INDEX account_sessions_by_account ON account_sessions(account_name);
