//! A flock: the deterministic workload behind the "release a flock" button.
//!
//! `n` pigeons, each named by [`TenantName::flock`], each writing between one
//! and three notes. The plan is a pure value so the integration test can check
//! the fleet afterwards against exactly what the plan wrote.

use crate::name::TenantName;

/// The most pigeons one flock may hold.
pub const MAX_FLOCK: u32 = 20_000;

/// One pigeon and the notes it writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bird {
    /// Its name (the tenant id).
    pub name: TenantName,
    /// What it writes, in order.
    pub notes: Vec<String>,
}

/// A flock of `n` pigeons starting at `offset` (so a second flock does not
/// reuse the first one's names).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flock {
    offset: u32,
    n: u32,
}

/// A flock bigger than [`MAX_FLOCK`], or one that would run past the name space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("a flock is 1 to {MAX_FLOCK} pigeons")]
pub struct FlockTooBig;

impl Flock {
    /// A flock of `n` starting at name number `offset`.
    ///
    /// # Errors
    ///
    /// [`FlockTooBig`] for `n == 0`, `n > MAX_FLOCK`, or an offset that
    /// would overflow.
    pub const fn new(offset: u32, n: u32) -> Result<Self, FlockTooBig> {
        if n == 0 || n > MAX_FLOCK || offset.checked_add(n).is_none() {
            return Err(FlockTooBig);
        }
        Ok(Self { offset, n })
    }

    /// How many pigeons.
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.n
    }

    /// Never empty; for clippy's sake.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        false
    }

    /// The birds, in order.
    pub fn birds(&self) -> impl Iterator<Item = Bird> + '_ {
        (self.offset..self.offset + self.n).map(|i| {
            let name = TenantName::flock(i);
            let count = 1 + (i % 3);
            let notes = (1..=count)
                .map(|k| format!("{name} was here ({k} of {count})"))
                .collect();
            Bird { name, notes }
        })
    }

    /// Total notes the flock writes.
    #[must_use]
    pub fn total_notes(&self) -> u64 {
        self.birds().map(|b| b.notes.len() as u64).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_flock_is_deterministic_and_counts_its_notes() {
        let f = Flock::new(0, 6).unwrap();
        let birds: Vec<Bird> = f.birds().collect();
        assert_eq!(birds.len(), 6);
        assert_eq!(
            birds.iter().map(|b| b.notes.len()).collect::<Vec<_>>(),
            vec![1, 2, 3, 1, 2, 3]
        );
        assert_eq!(f.total_notes(), 12);
        assert_eq!(birds, f.birds().collect::<Vec<_>>());
        assert_eq!(
            birds[2].notes[2],
            format!("{} was here (3 of 3)", birds[2].name)
        );
    }

    #[test]
    fn offsets_keep_flocks_apart() {
        let a: Vec<String> = Flock::new(0, 100)
            .unwrap()
            .birds()
            .map(|b| b.name.to_string())
            .collect();
        let b: Vec<String> = Flock::new(100, 100)
            .unwrap()
            .birds()
            .map(|b| b.name.to_string())
            .collect();
        assert!(a.iter().all(|n| !b.contains(n)));
    }

    #[test]
    fn bad_sizes_are_refused() {
        assert_eq!(Flock::new(0, 0), Err(FlockTooBig));
        assert_eq!(Flock::new(0, MAX_FLOCK + 1), Err(FlockTooBig));
        assert_eq!(Flock::new(u32::MAX, 1), Err(FlockTooBig));
        assert!(Flock::new(0, MAX_FLOCK).is_ok());
    }
}
