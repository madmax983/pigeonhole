//! The board's live state.
//!
//! How many pigeons each slot holds (from the registry), and which slot files
//! are open now (from a lifecycle hook on the fleet). Browsers fetch it as one
//! compact JSON document and paint it on a canvas; a ticker tells them over
//! SSE when it changed.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use autumn_web::db::fleet::{FleetDatabase, FleetLifecycle};
use holes::{SLOTS, Slot};
use serde::Serialize;

/// The live-events topic the board redraws on.
pub const TOPIC: &str = "board";

/// The digits a slot's occupancy is drawn with: `0`–`9`, then `a`–`z` for
/// 10–35. Anything above 35 draws as `z`.
const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";

#[derive(Debug)]
struct State {
    pigeons: Vec<u32>,
    open: Vec<bool>,
    open_now: usize,
    peak_open: usize,
    opens: u64,
}

/// The board.
#[derive(Debug)]
pub struct Board {
    state: Mutex<State>,
    dirty: AtomicBool,
}

impl Default for Board {
    fn default() -> Self {
        Self::new()
    }
}

/// What a browser draws.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Picture {
    /// One character per slot, in slot order: its pigeon count in base 36.
    pub occupancy: String,
    /// Slots whose file is open now.
    pub open: Vec<u16>,
    /// Pigeons in the registry.
    pub pigeons: u64,
    /// Slots holding at least one pigeon.
    pub occupied: u64,
    /// Slots holding two or more.
    pub shared: u64,
    /// The most pigeons in one slot.
    pub most: u32,
    /// Files open now / the most ever at once / opens since boot.
    pub open_now: usize,
    /// The most files ever open at once.
    pub peak_open: usize,
    /// Opens since boot.
    pub opens: u64,
}

impl Board {
    /// An empty board.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                pigeons: vec![0; usize::from(SLOTS)],
                open: vec![false; usize::from(SLOTS)],
                open_now: 0,
                peak_open: 0,
                opens: 0,
            }),
            dirty: AtomicBool::new(false),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        // A panic mid-update leaves a slightly wrong picture, which the next
        // event repairs; better than taking the board down.
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Set a slot's pigeon count (loading the registry at boot).
    pub fn set_pigeons(&self, slot: Slot, n: u32) {
        self.lock().pigeons[usize::from(slot.get())] = n;
        self.dirty.store(true, Ordering::Release);
    }

    /// One more pigeon in `slot`.
    pub fn add_pigeon(&self, slot: Slot) {
        self.lock().pigeons[usize::from(slot.get())] += 1;
        self.dirty.store(true, Ordering::Release);
    }

    /// A slot file opened or closed.
    pub fn set_open(&self, slot: Slot, open: bool) {
        let mut s = self.lock();
        let i = usize::from(slot.get());
        match (s.open[i], open) {
            (false, true) => {
                s.open_now += 1;
                s.opens += 1;
                s.peak_open = s.peak_open.max(s.open_now);
            }
            (true, false) => s.open_now -= 1,
            _ => {}
        }
        s.open[i] = open;
        drop(s);
        self.dirty.store(true, Ordering::Release);
    }

    /// Pigeons registered in `slot`.
    #[must_use]
    pub fn pigeons_in(&self, slot: Slot) -> u32 {
        self.lock().pigeons[usize::from(slot.get())]
    }

    /// Whether anything changed since the last call; clears the flag.
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }

    /// The picture.
    #[must_use]
    pub fn picture(&self) -> Picture {
        let s = self.lock();
        let occupancy: String = s
            .pigeons
            .iter()
            .map(|&n| char::from(DIGITS[n.min(35) as usize]))
            .collect();
        let open = s
            .open
            .iter()
            .enumerate()
            .filter(|(_, o)| **o)
            .filter_map(|(i, _)| u16::try_from(i).ok())
            .collect();
        Picture {
            occupancy,
            open,
            pigeons: s.pigeons.iter().map(|&n| u64::from(n)).sum(),
            occupied: s.pigeons.iter().filter(|&&n| n > 0).count() as u64,
            shared: s.pigeons.iter().filter(|&&n| n > 1).count() as u64,
            most: s.pigeons.iter().copied().max().unwrap_or(0),
            open_now: s.open_now,
            peak_open: s.peak_open,
            opens: s.opens,
        }
    }
}

/// The lifecycle hook that lights slots as their files open.
#[derive(Debug, Clone)]
pub struct Observer(pub std::sync::Arc<Board>);

impl FleetLifecycle for Observer {
    fn on_open(&self, db: &FleetDatabase) -> Result<(), String> {
        if let Some(s) = Slot::from_db_name(&db.key().name()) {
            self.0.set_open(s, true);
        }
        Ok(())
    }

    fn on_close(&self, db: &FleetDatabase) {
        if let Some(s) = Slot::from_db_name(&db.key().name()) {
            self.0.set_open(s, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(n: u32) -> Slot {
        Slot::new(n).unwrap()
    }

    #[test]
    fn the_picture_encodes_one_char_per_slot() {
        let b = Board::new();
        b.set_pigeons(s(0), 1);
        b.set_pigeons(s(1), 12);
        b.set_pigeons(s(16_383), 99);
        let p = b.picture();
        assert_eq!(p.occupancy.len(), 16_384);
        assert!(p.occupancy.starts_with("1c0"));
        assert!(p.occupancy.ends_with('z'));
        assert_eq!((p.pigeons, p.occupied, p.shared, p.most), (112, 3, 2, 99));
    }

    #[test]
    fn open_counts_follow_opens_and_closes() {
        let b = Board::new();
        b.set_open(s(5), true);
        b.set_open(s(5), true);
        b.set_open(s(9), true);
        b.set_open(s(5), false);
        let p = b.picture();
        assert_eq!((p.open_now, p.peak_open, p.opens), (1, 2, 2));
        assert_eq!(p.open, vec![9]);
        assert!(b.take_dirty());
        assert!(!b.take_dirty());
    }
}
