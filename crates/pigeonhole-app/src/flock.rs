//! Releasing a flock: registering thousands of pigeons and writing their
//! notes into the slot files their names hash to.
//!
//! Birds are dealt to [`WORKERS`] workers by slot, so one file is only ever
//! written by one worker: `SQLite` has one writer per file, and two workers on
//! one file would just queue on its lock.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;

use autumn_web::db::RuntimeConnection;
use autumn_web::db::fleet::{DatabaseFleet, FleetError};
use autumn_web::reexports::diesel;
use diesel_async::pooled_connection::deadpool::Pool;
use holes::{Bird, Flock};

use crate::board::Board;
use crate::slot_of;

/// How many slot files are written at once.
pub const WORKERS: usize = 8;

/// Why one bird failed.
#[derive(Debug, thiserror::Error)]
pub enum BirdError {
    /// The fleet refused.
    #[error(transparent)]
    Fleet(#[from] FleetError),
    /// No connection.
    #[error("no connection: {0}")]
    Pool(String),
    /// A query failed.
    #[error(transparent)]
    Query(#[from] diesel::result::Error),
}

/// The flock button's state.
#[derive(Debug, Default)]
pub struct FlockState {
    running: AtomicBool,
    releases: AtomicU64,
    total: AtomicU64,
    done: AtomicU64,
    failed: AtomicU64,
    notes: AtomicU64,
    next_offset: AtomicU64,
    peak_open: AtomicUsize,
    elapsed_ms: AtomicU64,
    last_error: Mutex<Option<String>>,
}

/// A copy of [`FlockState`] for drawing.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FlockReport {
    /// A flock is in the air.
    pub running: bool,
    /// Flocks released since boot.
    pub releases: u64,
    /// Birds in the current (or last) flock.
    pub total: u64,
    /// Birds landed.
    pub done: u64,
    /// Birds that failed.
    pub failed: u64,
    /// Notes written.
    pub notes: u64,
    /// The most slot files the fleet reported open at once.
    pub peak_open: usize,
    /// How long it took (so far).
    pub elapsed_ms: u64,
    /// The last failure.
    pub last_error: Option<String>,
}

impl FlockState {
    /// A copy for drawing.
    #[must_use]
    pub fn report(&self) -> FlockReport {
        FlockReport {
            running: self.running.load(Ordering::Acquire),
            releases: self.releases.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
            done: self.done.load(Ordering::Relaxed),
            failed: self.failed.load(Ordering::Relaxed),
            notes: self.notes.load(Ordering::Relaxed),
            peak_open: self.peak_open.load(Ordering::Relaxed),
            elapsed_ms: self.elapsed_ms.load(Ordering::Relaxed),
            last_error: self
                .last_error
                .lock()
                .map_or_else(|p| p.into_inner().clone(), |g| g.clone()),
        }
    }

    fn claim(&self) -> bool {
        self.running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn set_error(&self, e: Option<String>) {
        match self.last_error.lock() {
            Ok(mut g) => *g = e,
            Err(p) => *p.into_inner() = e,
        }
    }

    /// Reserve `n` fresh names; the offset the flock starts at.
    fn reserve(&self, n: u32) -> u32 {
        let start = self.next_offset.fetch_add(u64::from(n), Ordering::Relaxed);
        u32::try_from(start).unwrap_or(u32::MAX - n)
    }

    /// Start numbering new birds after `n` (the registry's size at boot).
    pub fn skip_names(&self, n: u64) {
        self.next_offset.fetch_max(n, Ordering::Relaxed);
    }
}

/// Release `n` pigeons in the background. `false` when a flock is already up.
pub fn release(
    fleet: DatabaseFleet,
    control: Pool<RuntimeConnection>,
    board: Arc<Board>,
    state: Arc<FlockState>,
    n: u32,
) -> bool {
    if !state.claim() {
        return false;
    }
    let offset = state.reserve(n);
    tokio::spawn(async move {
        if let Ok(flock) = Flock::new(offset, n) {
            run(&fleet, &control, &board, &state, flock).await;
        }
        state.running.store(false, Ordering::Release);
    });
    true
}

/// Play `flock` to the end. Public so the integration test can run one in
/// the foreground.
pub async fn run(
    fleet: &DatabaseFleet,
    control: &Pool<RuntimeConnection>,
    board: &Arc<Board>,
    state: &Arc<FlockState>,
    flock: Flock,
) {
    state.releases.fetch_add(1, Ordering::Relaxed);
    for c in [&state.done, &state.failed, &state.notes, &state.elapsed_ms] {
        c.store(0, Ordering::Relaxed);
    }
    state.total.store(u64::from(flock.len()), Ordering::Relaxed);
    state.peak_open.store(0, Ordering::Relaxed);
    state.set_error(None);
    let started = Instant::now();

    let mut lanes: Vec<Vec<Bird>> = vec![Vec::new(); WORKERS];
    for bird in flock.birds() {
        let lane = usize::from(slot_of(bird.name.as_str()).get()) % WORKERS;
        lanes[lane].push(bird);
    }
    let mut set = tokio::task::JoinSet::new();
    for lane in lanes {
        let (fleet, control, board, state) = (
            fleet.clone(),
            control.clone(),
            Arc::clone(board),
            Arc::clone(state),
        );
        set.spawn(async move {
            for bird in lane {
                match land(&fleet, &control, &board, &bird).await {
                    Ok(notes) => {
                        state.notes.fetch_add(notes, Ordering::Relaxed);
                    }
                    Err(e) => {
                        state.failed.fetch_add(1, Ordering::Relaxed);
                        state.set_error(Some(format!("{}: {e}", bird.name)));
                    }
                }
                state.done.fetch_add(1, Ordering::Relaxed);
                state
                    .peak_open
                    .fetch_max(fleet.stats().open, Ordering::Relaxed);
                state.elapsed_ms.store(
                    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
                    Ordering::Relaxed,
                );
            }
        });
    }
    while set.join_next().await.is_some() {}
}

/// Register one bird and write its notes into its slot file.
///
/// # Errors
///
/// The registry, the fleet or the slot file refused.
pub async fn land(
    fleet: &DatabaseFleet,
    control: &Pool<RuntimeConnection>,
    board: &Board,
    bird: &Bird,
) -> Result<u64, BirdError> {
    use diesel_async::RunQueryDsl as _;

    let name = bird.name.as_str();
    let slot = slot_of(name);
    let now = chrono::Utc::now().naive_utc();
    {
        let mut c = control
            .get()
            .await
            .map_err(|e| BirdError::Pool(e.to_string()))?;
        let inserted = diesel::sql_query(
            "INSERT OR IGNORE INTO pigeons (name, slot, created_at) VALUES (?, ?, ?)",
        )
        .bind::<diesel::sql_types::Text, _>(name)
        .bind::<diesel::sql_types::Integer, _>(i32::from(slot.get()))
        .bind::<diesel::sql_types::Timestamp, _>(now)
        .execute(&mut *c)
        .await?;
        if inserted == 1 {
            board.add_pigeon(slot);
        }
    }
    let key = fleet.key_for(name)?;
    let db = fleet.open(&key).await?;
    let _lease = db.lease();
    let mut conn = db
        .pool()
        .get()
        .await
        .map_err(|e| BirdError::Pool(e.to_string()))?;
    for body in &bird.notes {
        diesel::sql_query("INSERT INTO notes (body, tenant_id, created_at) VALUES (?, ?, ?)")
            .bind::<diesel::sql_types::Text, _>(body)
            .bind::<diesel::sql_types::Text, _>(name)
            .bind::<diesel::sql_types::Timestamp, _>(now)
            .execute(&mut *conn)
            .await?;
    }
    Ok(bird.notes.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_reserved_without_reuse() {
        let s = FlockState::default();
        assert_eq!(s.reserve(100), 0);
        assert_eq!(s.reserve(50), 100);
        s.skip_names(1_000);
        assert_eq!(s.reserve(1), 1_000);
        s.skip_names(10);
        assert_eq!(s.reserve(1), 1_001);
    }

    #[test]
    fn one_flock_at_a_time() {
        let s = FlockState::default();
        assert!(s.claim());
        assert!(!s.claim());
    }
}
