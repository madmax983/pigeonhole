//! Pigeonhole's engine. No IO, no async, no clock.
//!
//! - [`name`] — which strings may be a tenant id.
//! - [`grid`] — the 128 × 128 board of the fleet's 16,384 slots
//!   (`verus/src/grid.rs` proves the drawing is a bijection).
//! - [`census`] — folding every slot file's report into one picture, and
//!   finding rows in the wrong file (`verus/src/census.rs` proves the totals
//!   conserve).
//! - [`flock`] — the deterministic workload behind the flock button.
#![forbid(unsafe_code)]

pub mod census;
pub mod flock;
pub mod grid;
pub mod name;

pub use census::{Census, FileReport, Rows, Stray, expected_files};
pub use flock::{Bird, Flock};
pub use grid::{SIDE, SLOTS, Slot, cell, slot_at};
pub use name::{TenantName, TenantNameError};
