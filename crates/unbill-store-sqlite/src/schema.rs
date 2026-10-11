diesel::table! {
    ledgers (id) {
        id -> Text,
        metadata -> Nullable<Binary>,
        document -> Nullable<Binary>,
        revision -> BigInt,
    }
}

diesel::table! {
    device_identity (id) {
        id -> Integer,
        secret_key -> Binary,
    }
}

diesel::table! {
    pending_invitations (token) {
        token -> Text,
        ledger_id -> Text,
        created_by_device -> Text,
        created_at_ms -> BigInt,
        expires_at_ms -> BigInt,
    }
}

diesel::table! {
    storage_revisions (id) {
        id -> Integer,
        clock -> BigInt,
        identity_revision -> BigInt,
        invitations_revision -> BigInt,
    }
}

diesel::table! {
    ledger_archives (ledger_id) {
        ledger_id -> Text,
        archived -> Bool,
    }
}
diesel::joinable!(ledger_archives -> ledgers (ledger_id));
diesel::allow_tables_to_appear_in_same_query!(ledgers, ledger_archives);
