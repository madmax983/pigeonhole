//! The census: what every slot file holds, folded into one picture.
//!
//! The app runs `Shards::each_shard` over the fleet. For each slot file it
//! reports, per tenant id found in the file, how many notes that tenant has
//! there. [`Census::fold`] adds the reports up and checks each row against the
//! router: a tenant's rows belong in exactly one slot — the one its id hashes
//! to — and a row anywhere else is a **stray**.
//!
//! Formal (`verus/src/census.rs`): the folded totals conserve the reports —
//! `tenants == Σ per-file tenants` and `notes == Σ per-file notes` — however
//! the reports are ordered.

use std::collections::BTreeMap;

use crate::grid::Slot;

/// One tenant's rows in one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rows {
    /// The tenant id found in the file.
    pub tenant: String,
    /// How many of its notes the file holds.
    pub notes: u64,
}

/// What one slot file reported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileReport {
    /// The file's slot.
    pub slot: Slot,
    /// Its tenants, each once.
    pub rows: Vec<Rows>,
}

/// A row in the wrong file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stray {
    /// Where the row is.
    pub found_in: Slot,
    /// Where the router sends its tenant.
    pub belongs_in: Slot,
    /// The tenant.
    pub tenant: String,
}

/// The folded census.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Census {
    /// Slot files reported.
    pub files: u64,
    /// Tenants found, counted once per file they appear in.
    pub tenants: u64,
    /// Notes found.
    pub notes: u64,
    /// Files holding two or more tenants.
    pub shared_files: u64,
    /// The most tenants any one file holds.
    pub most_in_one_file: u64,
    /// How many files hold exactly `k` tenants, by `k`.
    pub by_occupancy: BTreeMap<u64, u64>,
    /// Rows found in a file their tenant's id does not hash to.
    pub strays: Vec<Stray>,
}

impl Census {
    /// Fold `reports`. `route` is the router: the slot a tenant id hashes to.
    pub fn fold(reports: &[FileReport], route: impl Fn(&str) -> Slot) -> Self {
        let mut c = Self::default();
        for r in reports {
            let k = r.rows.len() as u64;
            c.files += 1;
            c.tenants += k;
            *c.by_occupancy.entry(k).or_insert(0) += 1;
            if k >= 2 {
                c.shared_files += 1;
            }
            c.most_in_one_file = c.most_in_one_file.max(k);
            for row in &r.rows {
                c.notes += row.notes;
                let home = route(&row.tenant);
                if home != r.slot {
                    c.strays.push(Stray {
                        found_in: r.slot,
                        belongs_in: home,
                        tenant: row.tenant.clone(),
                    });
                }
            }
        }
        c
    }
}

/// Expected number of distinct slots hit by `n` uniformly hashed keys:
/// `S · (1 − (1 − 1/S)ⁿ)`. The board shows it next to the real count.
#[must_use]
pub fn expected_files(n: u64) -> f64 {
    let s = f64::from(crate::grid::SLOTS);
    let n = i32::try_from(n).unwrap_or(i32::MAX);
    s * (1.0 - (1.0 - 1.0 / s).powi(n))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn s(n: u32) -> Slot {
        Slot::new(n).unwrap()
    }

    fn rows(pairs: &[(&str, u64)]) -> Vec<Rows> {
        pairs
            .iter()
            .map(|(t, n)| Rows {
                tenant: (*t).to_owned(),
                notes: *n,
            })
            .collect()
    }

    /// A toy router: the tenant's first byte.
    fn route(t: &str) -> Slot {
        s(u32::from(t.as_bytes()[0]))
    }

    #[test]
    fn a_clean_fleet_has_no_strays() {
        let reports = vec![
            FileReport {
                slot: s(u32::from(b'a')),
                rows: rows(&[("ash", 3), ("amber", 1)]),
            },
            FileReport {
                slot: s(u32::from(b'b')),
                rows: rows(&[("brass", 2)]),
            },
        ];
        let c = Census::fold(&reports, route);
        assert_eq!((c.files, c.tenants, c.notes), (2, 3, 6));
        assert_eq!((c.shared_files, c.most_in_one_file), (1, 2));
        assert_eq!(c.by_occupancy, BTreeMap::from([(1, 1), (2, 1)]));
        assert!(c.strays.is_empty());
    }

    #[test]
    fn a_row_in_the_wrong_file_is_named() {
        let reports = vec![FileReport {
            slot: s(u32::from(b'a')),
            rows: rows(&[("brass", 1)]),
        }];
        let c = Census::fold(&reports, route);
        assert_eq!(
            c.strays,
            vec![Stray {
                found_in: s(u32::from(b'a')),
                belongs_in: s(u32::from(b'b')),
                tenant: "brass".into()
            }]
        );
    }

    #[test]
    fn expected_files_matches_the_birthday_curve() {
        assert!(expected_files(0).abs() < 1e-9);
        assert!((expected_files(1) - 1.0).abs() < 1e-9);
        // 3,000 keys into 16,384 slots: ~2,746 distinct.
        let e = expected_files(3_000);
        assert!((2_740.0..2_755.0).contains(&e), "{e}");
        assert!(expected_files(1_000_000) < f64::from(crate::grid::SLOTS) + 1e-6);
    }

    proptest! {
        /// The Verus theorem, run: totals conserve the reports, in any order.
        #[test]
        fn totals_conserve_and_ignore_order(
            files in proptest::collection::vec(
                (0u32..16_384, proptest::collection::vec(("[a-z]{1,6}", 0u64..50), 0..6)), 0..40),
            rot in 0usize..40,
        ) {
            let reports: Vec<FileReport> = files.iter().map(|(slot, rs)| FileReport {
                slot: s(*slot),
                rows: rs.iter().map(|(t, n)| Rows { tenant: t.clone(), notes: *n }).collect(),
            }).collect();
            let c = Census::fold(&reports, route);
            let tenants: u64 = reports.iter().map(|r| r.rows.len() as u64).sum();
            let notes: u64 = reports.iter().flat_map(|r| r.rows.iter()).map(|r| r.notes).sum();
            prop_assert_eq!(c.files, reports.len() as u64);
            prop_assert_eq!(c.tenants, tenants);
            prop_assert_eq!(c.notes, notes);
            prop_assert_eq!(c.by_occupancy.values().sum::<u64>(), c.files);
            let mut rotated = reports;
            if !rotated.is_empty() { let k = rot % rotated.len(); rotated.rotate_left(k); }
            let d = Census::fold(&rotated, route);
            prop_assert_eq!((d.files, d.tenants, d.notes, d.shared_files), (c.files, c.tenants, c.notes, c.shared_files));
        }
    }
}
