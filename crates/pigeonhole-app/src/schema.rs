//! Diesel table definitions, mirroring `migrations/`.

diesel::table! {
    notes (id) {
        id -> BigInt,
        body -> Text,
        tenant_id -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::table! {
    pigeons (name) {
        name -> Text,
        slot -> Integer,
        created_at -> Timestamp,
    }
}
