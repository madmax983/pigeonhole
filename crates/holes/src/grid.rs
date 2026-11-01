//! The board: 16,384 slots as a 128 × 128 grid.
//!
//! autumn-web's slot fleet routes every key to one of [`SLOTS`] slots, and a
//! slot is one `SQLite` file. The board draws slot `s` at row `s / 128`,
//! column `s % 128`. [`cell`] and [`slot_at`] are inverse to each other on the
//! whole board, which `verus/src/grid.rs` proves.

/// Slots in an autumn-web slot fleet (`autumn_web::config::SLOT_COUNT`).
pub const SLOTS: u16 = 16_384;
/// The board's side.
pub const SIDE: u16 = 128;

/// A slot number, `0..SLOTS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Slot(u16);

/// A slot number at or past [`SLOTS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("slot {0} is outside 0..16384")]
pub struct OutOfRange(pub u32);

impl Slot {
    /// Slot 0.
    pub const FIRST: Self = Self(0);

    /// Check and wrap.
    ///
    /// # Errors
    ///
    /// [`OutOfRange`] for `n >= SLOTS`.
    pub fn new(n: u32) -> Result<Self, OutOfRange> {
        u16::try_from(n)
            .ok()
            .filter(|&s| s < SLOTS)
            .map(Self)
            .ok_or(OutOfRange(n))
    }

    /// The number.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }

    /// The fleet's name for this slot's database: `slot:00042`.
    #[must_use]
    pub fn db_name(self) -> String {
        format!("slot:{:05}", self.0)
    }

    /// Parse a fleet database name back into a slot.
    #[must_use]
    pub fn from_db_name(name: &str) -> Option<Self> {
        let digits = name.strip_prefix("slot:")?;
        if digits.len() != 5 || !digits.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        Self::new(digits.parse().ok()?).ok()
    }
}

/// Where a slot sits on the board: `(row, column)`.
#[must_use]
pub const fn cell(s: Slot) -> (u16, u16) {
    (s.0 / SIDE, s.0 % SIDE)
}

/// The slot at `(row, column)`, if both are on the board.
#[must_use]
pub const fn slot_at(row: u16, col: u16) -> Option<Slot> {
    if row < SIDE && col < SIDE {
        Some(Slot(row * SIDE + col))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn corners() {
        assert_eq!(cell(Slot::new(0).unwrap()), (0, 0));
        assert_eq!(cell(Slot::new(127).unwrap()), (0, 127));
        assert_eq!(cell(Slot::new(128).unwrap()), (1, 0));
        assert_eq!(cell(Slot::new(16_383).unwrap()), (127, 127));
        assert_eq!(slot_at(128, 0), None);
        assert_eq!(slot_at(0, 128), None);
    }

    #[test]
    fn range_is_checked() {
        assert!(Slot::new(16_383).is_ok());
        assert_eq!(Slot::new(16_384), Err(OutOfRange(16_384)));
        assert_eq!(Slot::new(70_000), Err(OutOfRange(70_000)));
    }

    #[test]
    fn db_names_round_trip_and_reject_lookalikes() {
        let s = Slot::new(42).unwrap();
        assert_eq!(s.db_name(), "slot:00042");
        assert_eq!(Slot::from_db_name("slot:00042"), Some(s));
        for bad in [
            "slot:42",
            "slot:16384",
            "slot:0004x",
            "tenant:acme",
            "slot:-0001",
            "slot:000042",
        ] {
            assert_eq!(Slot::from_db_name(bad), None, "{bad}");
        }
    }

    #[test]
    fn every_slot_has_exactly_one_cell() {
        let mut seen = vec![false; usize::from(SLOTS)];
        for row in 0..SIDE {
            for col in 0..SIDE {
                let s = slot_at(row, col).unwrap();
                assert!(!seen[usize::from(s.get())]);
                seen[usize::from(s.get())] = true;
            }
        }
        assert!(seen.iter().all(|b| *b));
    }

    proptest! {
        #[test]
        fn cell_and_slot_at_are_inverse(n in 0u32..16_384) {
            let s = Slot::new(n).unwrap();
            let (r, c) = cell(s);
            prop_assert_eq!(slot_at(r, c), Some(s));
            prop_assert_eq!(Slot::from_db_name(&s.db_name()), Some(s));
        }
    }
}
