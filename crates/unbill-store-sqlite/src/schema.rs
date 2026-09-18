diesel::table! {
    ledgers (id) {
        id -> Text,
        metadata -> Nullable<Binary>,
        document -> Nullable<Binary>,
    }
}

diesel::table! {
    device_identity (id) {
        id -> Integer,
        secret_key -> Binary,
    }
}

diesel::table! {
    device_labels (node_id) {
        node_id -> Text,
        label -> Text,
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
