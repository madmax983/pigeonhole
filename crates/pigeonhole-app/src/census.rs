//! Taking the census: every slot file on disk, read through
//! `Shards::each_shard`, folded by [`holes::Census::fold`], and checked
//! against the registry.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use autumn_web::db::RuntimeConnection;
use autumn_web::error::AutumnError;
use autumn_web::reexports::{diesel, diesel_async};
use autumn_web::sharding::Shards;
use diesel_async::RunQueryDsl as _;
use holes::{Census, FileReport, Rows, Slot};

use crate::models::TenantRows;
use crate::slot_of;

/// What the census page shows.
#[derive(Debug, Clone)]
pub struct Taken {
    /// The folded census.
    pub census: Census,
    /// Pigeons in the registry.
    pub registered: u64,
    /// Registered pigeons with no rows in their slot file.
    pub missing: Vec<String>,
    /// Tenants found in files that the registry does not know.
    pub unknown: Vec<String>,
    /// Files that could not be read.
    pub unreadable: Vec<(String, String)>,
    /// How long the sweep took.
    pub took: Duration,
}

impl Taken {
    /// Whether everything agrees: no strays, nothing missing, nothing
    /// unknown, nothing unreadable.
    #[must_use]
    pub const fn clean(&self) -> bool {
        self.census.strays.is_empty()
            && self.missing.is_empty()
            && self.unknown.is_empty()
            && self.unreadable.is_empty()
    }
}

/// Read every slot file and the registry, and fold them.
///
/// # Errors
///
/// The registry could not be read.
pub async fn take(shards: &Shards, control: &mut RuntimeConnection) -> Result<Taken, AutumnError> {
    #[derive(diesel::QueryableByName)]
    struct Name {
        #[diesel(sql_type = diesel::sql_types::Text)]
        name: String,
    }
    let started = Instant::now();
    let results = shards
        .each_shard(|shard, mut db| {
            let name = shard.name().to_owned();
            async move {
                let rows: Vec<TenantRows> = diesel::sql_query(
                    "SELECT COALESCE(tenant_id, '') AS tenant, COUNT(*) AS n \
                     FROM notes GROUP BY tenant_id ORDER BY tenant_id",
                )
                .load(&mut *db)
                .await?;
                Ok((name, rows))
            }
        })
        .await;

    let mut reports = Vec::with_capacity(results.len());
    let mut unreadable = Vec::new();
    for (id, result) in results {
        match result {
            Ok((name, rows)) => match Slot::from_db_name(&name) {
                Some(slot) => reports.push(FileReport {
                    slot,
                    rows: rows
                        .into_iter()
                        .map(|r| Rows {
                            tenant: r.tenant,
                            notes: u64::try_from(r.n).unwrap_or(0),
                        })
                        .collect(),
                }),
                None => unreadable.push((name, "not a slot database".to_owned())),
            },
            Err(e) => unreadable.push((format!("{id:?}"), e.to_string())),
        }
    }
    let census = Census::fold(&reports, slot_of);

    let registry: Vec<Name> = diesel::sql_query("SELECT name FROM pigeons ORDER BY name")
        .load(control)
        .await?;
    let registered: BTreeSet<String> = registry.into_iter().map(|n| n.name).collect();
    let found: BTreeSet<String> = reports
        .iter()
        .flat_map(|r| r.rows.iter().map(|row| row.tenant.clone()))
        .collect();
    Ok(Taken {
        registered: registered.len() as u64,
        missing: registered.difference(&found).take(50).cloned().collect(),
        unknown: found.difference(&registered).take(50).cloned().collect(),
        unreadable,
        census,
        took: started.elapsed(),
    })
}
