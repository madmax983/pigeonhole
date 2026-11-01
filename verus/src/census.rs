// Pigeonhole — the verified core, part two: the census conserves.
//
//   verus verus/src/census.rs
//
// The census runs over every slot file and adds up what each reports: how
// many tenants it holds and how many notes. If the fold dropped or double
// counted a report, the board's totals would disagree with the fleet, and the
// "no strays" check would be checking the wrong population. This file proves
// the executable fold computes exactly the spec sums, so
//
//   files   == number of reports
//   tenants == Σ tenants(report)
//   notes   == Σ notes(report)
//
// and that the sums do not depend on the order the reports arrive in (the
// fleet's `each` returns them in key order, but `each_shard` runs them
// concurrently, and the fold must not care).
//
// ── The modelling assumption ────────────────────────────────────────────────
//
// A report is abstracted to its two counts `(tenants, notes)`. The runtime's
// per-row work (stray detection, the occupancy histogram) reads the same
// reports but does not feed these three totals. Counts are bounded so the
// u64 sums cannot overflow (the runtime holds at most 16,384 reports with at
// most a few million rows each).

use vstd::prelude::*;

verus! {

pub struct Report {
    pub tenants: u64,
    pub notes: u64,
}

pub open spec fn sum_tenants(rs: Seq<Report>) -> int
    decreases rs.len(),
{
    if rs.len() == 0 { 0 } else { sum_tenants(rs.drop_last()) + rs.last().tenants as int }
}

pub open spec fn sum_notes(rs: Seq<Report>) -> int
    decreases rs.len(),
{
    if rs.len() == 0 { 0 } else { sum_notes(rs.drop_last()) + rs.last().notes as int }
}

pub open spec fn bounded(rs: Seq<Report>) -> bool {
    &&& rs.len() <= 16384
    &&& forall|i: int| 0 <= i < rs.len() ==> rs[i].tenants <= 1_000_000 && rs[i].notes <= 1_000_000_000
}

proof fn lemma_sums_bounded(rs: Seq<Report>)
    requires
        bounded(rs),
    ensures
        0 <= sum_tenants(rs) <= rs.len() * 1_000_000,
        0 <= sum_notes(rs) <= rs.len() * 1_000_000_000,
    decreases rs.len(),
{
    if rs.len() > 0 {
        let p = rs.drop_last();
        assert(bounded(p)) by {
            assert forall|i: int| 0 <= i < p.len() implies p[i].tenants <= 1_000_000 && p[i].notes
                <= 1_000_000_000 by {
                assert(p[i] == rs[i]);
            }
        }
        lemma_sums_bounded(p);
        assert(rs.last() == rs[rs.len() - 1]);
    }
}

pub struct Totals {
    pub files: u64,
    pub tenants: u64,
    pub notes: u64,
}

/// SPEC 3.1. The fold, as `Census::fold` does it for the three totals.
pub fn fold(reports: &Vec<Report>) -> (t: Totals)
    requires
        bounded(reports@),
    ensures
        t.files as int == reports@.len(),
        t.tenants as int == sum_tenants(reports@),
        t.notes as int == sum_notes(reports@),
{
    let mut t = Totals { files: 0, tenants: 0, notes: 0 };
    let mut i: usize = 0;
    while i < reports.len()
        invariant
            bounded(reports@),
            i <= reports.len(),
            t.files as int == i,
            t.tenants as int == sum_tenants(reports@.subrange(0, i as int)),
            t.notes as int == sum_notes(reports@.subrange(0, i as int)),
        decreases reports.len() - i,
    {
        let pre = reports@.subrange(0, i as int);
        let next = reports@.subrange(0, i as int + 1);
        assert(next.drop_last() =~= pre);
        assert(next.last() == reports@[i as int]);
        proof {
            assert(bounded(next)) by {
                assert forall|k: int| 0 <= k < next.len() implies next[k].tenants <= 1_000_000
                    && next[k].notes <= 1_000_000_000 by {
                    assert(next[k] == reports@[k]);
                }
            }
            lemma_sums_bounded(next);
        }
        t.files = t.files + 1;
        t.tenants = t.tenants + reports[i].tenants;
        t.notes = t.notes + reports[i].notes;
        i = i + 1;
    }
    assert(reports@.subrange(0, reports@.len() as int) =~= reports@);
    t
}

/// SPEC 3.2. Order does not matter: moving the last report to the front
/// leaves both sums unchanged. Any permutation is a sequence of such moves on
/// prefixes, which the property test `totals_conserve_and_ignore_order`
/// exercises at runtime; this lemma is the step that makes it true.
pub proof fn lemma_rotate_last_to_front(rs: Seq<Report>)
    requires
        rs.len() > 0,
    ensures
        sum_tenants(seq![rs.last()] + rs.drop_last()) == sum_tenants(rs),
        sum_notes(seq![rs.last()] + rs.drop_last()) == sum_notes(rs),
{
    lemma_prepend(rs.last(), rs.drop_last());
}

/// Prepending adds the element's counts.
pub proof fn lemma_prepend(x: Report, rs: Seq<Report>)
    ensures
        sum_tenants(seq![x] + rs) == x.tenants as int + sum_tenants(rs),
        sum_notes(seq![x] + rs) == x.notes as int + sum_notes(rs),
    decreases rs.len(),
{
    let s = seq![x] + rs;
    if rs.len() == 0 {
        assert(s.drop_last() =~= Seq::<Report>::empty());
    } else {
        assert(s.drop_last() =~= seq![x] + rs.drop_last());
        assert(s.last() == rs.last());
        lemma_prepend(x, rs.drop_last());
    }
}

fn main() {
}

} // verus!
