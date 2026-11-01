//! Rows.

use chrono::NaiveDateTime;
use diesel::{Insertable, Queryable, QueryableByName, Selectable};

use crate::schema::{notes, pigeons};

/// A note. Lives in the slot file its tenant hashes to, beside the notes of
/// every other tenant that hashes there.
#[autumn_web::model]
pub struct Note {
    /// Row id, unique within the slot file only.
    #[id]
    pub id: i64,
    /// What was written.
    pub body: String,
    /// The tenant; the only thing separating neighbours in a shared file.
    pub tenant_id: Option<String>,
    /// When (UTC).
    pub created_at: NaiveDateTime,
}

/// Tenant-scoped and sharded: every query runs on the request's slot file
/// and filters on the request's tenant.
#[autumn_web::repository(Note, tenant_scoped, sharded)]
pub trait NoteRepository {}

/// A pigeon in the registry (control database).
#[derive(Debug, Clone, PartialEq, Eq, Queryable, Selectable, Insertable)]
#[diesel(table_name = pigeons)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Pigeon {
    /// The tenant id.
    pub name: String,
    /// The slot it hashes to.
    pub slot: i32,
    /// When it was founded (UTC).
    pub created_at: NaiveDateTime,
}

/// `SELECT COUNT(*) AS n`.
#[derive(Debug, Clone, Copy, QueryableByName)]
pub struct Count {
    /// The count.
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub n: i64,
}

/// One tenant's note count in a slot file.
#[derive(Debug, Clone, QueryableByName)]
pub struct TenantRows {
    /// The tenant id (empty for a row with none).
    #[diesel(sql_type = diesel::sql_types::Text)]
    pub tenant: String,
    /// Its notes in this file.
    #[diesel(sql_type = diesel::sql_types::BigInt)]
    pub n: i64,
}
