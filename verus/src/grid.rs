// Pigeonhole — the verified core, part one: the board is a faithful drawing.
//
//   verus verus/src/grid.rs
//
// The board draws slot s at (s / 128, s % 128). If that were not a bijection
// between the fleet's 16,384 slots and the board's 128 × 128 cells, two slot
// files could share a cell (and one would be invisible) or a cell could show a
// slot that does not exist. This file proves it is one: `cell` and `slot_at`
// are inverse on the whole board, and every cell names an in-range slot.
//
// ── The modelling assumption ────────────────────────────────────────────────
//
// The runtime wraps the slot in a newtype that refuses values >= 16,384; here
// that refusal is the `requires s < SLOTS`. Nothing else differs.

use vstd::prelude::*;

verus! {

pub const SLOTS: u16 = 16384;
pub const SIDE: u16 = 128;

/// SPEC 2.1. Where a slot is drawn.
pub fn cell(s: u16) -> (rc: (u16, u16))
    requires
        s < SLOTS,
    ensures
        rc.0 < SIDE,
        rc.1 < SIDE,
        rc.0 as int * 128 + rc.1 as int == s as int,
{
    proof {
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod(s as int, 128);
        vstd::arithmetic::div_mod::lemma_div_pos_is_pos(s as int, 128);
        vstd::arithmetic::div_mod::lemma_mod_bound(s as int, 128);
        assert(s as int / 128 < 128) by (nonlinear_arith)
            requires s < 16384;
    }
    (s / SIDE, s % SIDE)
}

/// SPEC 2.2. The slot drawn at a cell.
pub fn slot_at(row: u16, col: u16) -> (r: Option<u16>)
    ensures
        (row < SIDE && col < SIDE) <==> r is Some,
        r is Some ==> r->Some_0 < SLOTS,
        r is Some ==> r->Some_0 as int == row as int * 128 + col as int,
{
    if row < SIDE && col < SIDE {
        assert(row as int * 128 + col as int <= 127 * 128 + 127) by (nonlinear_arith)
            requires row < 128, col < 128;
        Some(row * SIDE + col)
    } else {
        None
    }
}

/// SPEC 2.3. Round trip from a slot.
pub fn round_trip_slot(s: u16)
    requires
        s < SLOTS,
{
    let (r, c) = cell(s);
    let back = slot_at(r, c);
    assert(back == Some(s));
}

/// SPEC 2.4. Round trip from a cell: two different cells never draw the same
/// slot, because the slot determines the cell.
pub proof fn lemma_cell_is_determined(r1: int, c1: int, r2: int, c2: int)
    requires
        0 <= r1 < 128,
        0 <= c1 < 128,
        0 <= r2 < 128,
        0 <= c2 < 128,
        r1 * 128 + c1 == r2 * 128 + c2,
    ensures
        r1 == r2,
        c1 == c2,
{
    assert(r1 == r2 && c1 == c2) by (nonlinear_arith)
        requires
            0 <= r1 < 128,
            0 <= c1 < 128,
            0 <= r2 < 128,
            0 <= c2 < 128,
            r1 * 128 + c1 == r2 * 128 + c2;
}

fn main() {
}

} // verus!
