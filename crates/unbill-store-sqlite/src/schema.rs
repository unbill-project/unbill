diesel::table! {
    ledgers (id) {
        id -> Text,
        metadata -> Nullable<Binary>,
        document -> Nullable<Binary>,
    }
}

diesel::table! {
    device_metadata (key) {
        key -> Text,
        value -> Binary,
    }
}
