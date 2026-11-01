//! Pigeonhole's web application.
//!
//! ```text
//!   browser ──► routes ──► ShardedDb / PgNoteRepository ──► the tenant's slot file
//!                  │            (shared with every tenant that hashes there)
//!                  ├──► flock ──► DatabaseFleet (open, write, LRU)
//!                  ├──► census ──► Shards::each_shard ──► every slot file on disk
//!                  └──► control database (sessions, the pigeon registry)
//!   DatabaseFleet ──on_open/on_close──► board::Observer ──► Board ──tick──► SSE
//! ```
#![warn(missing_docs)]

pub mod board;
pub mod census;
pub mod flock;
// `#[autumn_web::model]` and `#[repository]` generate items they do not
// document.
#[allow(missing_docs)]
pub mod models;
pub mod routes;
#[allow(missing_docs)]
pub mod schema;

use std::sync::Arc;
use std::time::Duration;

use autumn_web::AppState;
use autumn_web::db::fleet::DatabaseFleet;
use autumn_web::error::AutumnError;
use autumn_web::reexports::{diesel, diesel_async};
use diesel_async::RunQueryDsl as _;
use holes::{Slot, TenantName};

use crate::board::{Board, Observer, TOPIC};
use crate::flock::FlockState;

/// How often, at most, the board is pushed to browsers.
pub const TICK: Duration = Duration::from_millis(300);

/// Everything Pigeonhole hangs on the `AppState`.
#[derive(Debug, Clone)]
pub struct Pigeonhole {
    /// The slot fleet.
    pub fleet: DatabaseFleet,
    /// The board.
    pub board: Arc<Board>,
    /// The flock button's state.
    pub flock: Arc<FlockState>,
}

impl Pigeonhole {
    /// Pigeonhole's state, from a handler.
    ///
    /// # Errors
    ///
    /// `503` before boot finished, or with no fleet configured.
    pub fn of(state: &AppState) -> Result<Arc<Self>, AutumnError> {
        state.extension::<Self>().ok_or_else(|| {
            AutumnError::service_unavailable_msg(
                "Pigeonhole is not ready: no [database.fleet] configured, or boot has not finished",
            )
        })
    }
}

/// The slot a tenant id hashes to — the router's own function, so the board
/// and the census can never disagree with where requests go.
#[must_use]
pub fn slot_of(tenant: &str) -> Slot {
    let id = autumn_web::sharding::slot_for_key(tenant.into()).0;
    // `slot_for_key` reduces modulo SLOT_COUNT, so this cannot fail; were it
    // ever to, the census would report the rows as strays in slot 0.
    Slot::new(u32::from(id)).unwrap_or(Slot::FIRST)
}

/// The tenant a session names, validated.
///
/// # Errors
///
/// `400` for a name that is not a pigeon name.
pub fn tenant_name(raw: &str) -> Result<TenantName, AutumnError> {
    TenantName::parse(raw).map_err(|e| AutumnError::bad_request_msg(e.to_string()))
}

/// The startup hook: hang Pigeonhole on the app, then load the registry onto
/// the board.
///
/// # Errors
///
/// No fleet or control database is configured, or the registry cannot be
/// read.
pub async fn boot(state: AppState) -> autumn_web::AutumnResult<()> {
    #[derive(diesel::QueryableByName)]
    struct SlotCount {
        #[diesel(sql_type = diesel::sql_types::Integer)]
        slot: i32,
        #[diesel(sql_type = diesel::sql_types::BigInt)]
        n: i64,
    }
    let pigeonhole = install(&state)?;
    let pool = state
        .pool()
        .ok_or_else(|| AutumnError::service_unavailable_msg("no control database"))?;
    let mut conn = pool
        .get()
        .await
        .map_err(|e| AutumnError::service_unavailable_msg(e.to_string()))?;
    let rows: Vec<SlotCount> =
        diesel::sql_query("SELECT slot, COUNT(*) AS n FROM pigeons GROUP BY slot")
            .load(&mut *conn)
            .await?;
    for r in &rows {
        if let (Ok(s), Ok(n)) = (
            Slot::new(u32::try_from(r.slot).unwrap_or(u32::MAX)),
            u32::try_from(r.n),
        ) {
            pigeonhole.board.set_pigeons(s, n);
        }
    }
    tracing::info!(slots = rows.len(), "the board is loaded from the registry");
    Ok(())
}

/// Hang Pigeonhole on `state`: the board hook on the fleet, the ticker, and
/// the extension. Synchronous so a test can call it from
/// `TestApp::state_initializer`; it must run inside a Tokio runtime.
///
/// # Errors
///
/// No fleet is configured.
pub fn install(state: &AppState) -> Result<Arc<Pigeonhole>, AutumnError> {
    let fleet = state
        .shards()
        .and_then(|s| s.fleet())
        .cloned()
        .ok_or_else(|| {
            AutumnError::service_unavailable_msg("Pigeonhole needs [database.fleet] in autumn.toml")
        })?;
    let board = Arc::new(Board::new());
    fleet.add_lifecycle(Arc::new(Observer(Arc::clone(&board))));

    let channels = state.channels().clone();
    let ticker = Arc::clone(&board);
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(TICK);
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            if ticker.take_dirty() {
                let _ = channels.publish(TOPIC, "tick");
            }
        }
    });

    state.insert_extension(Pigeonhole {
        fleet,
        board,
        flock: Arc::new(FlockState::default()),
    });
    Pigeonhole::of(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_declared_route_is_registered() {
        let main_source = include_str!("main.rs");
        for handler in crate::routes::ALL {
            assert!(main_source.contains(handler), "{handler} is not registered");
        }
    }

    #[test]
    fn the_config_is_a_sqlite_slot_fleet() {
        let base = include_str!("../../../autumn.toml");
        assert!(base.contains("[database.fleet]"));
        assert!(base.contains(r#"mode = "slot""#));
        let dev = include_str!("../../../autumn-dev.toml");
        assert!(dev.contains("sqlite://") && !dev.contains("postgres://"));
    }

    #[test]
    fn slot_of_is_the_routers_slot_and_stable() {
        let a = slot_of("brass-rock-1");
        assert_eq!(a, slot_of("brass-rock-1"));
        assert_eq!(
            u32::from(a.get()),
            u32::from(autumn_web::sharding::slot_for_key("brass-rock-1".into()).0)
        );
    }
}
