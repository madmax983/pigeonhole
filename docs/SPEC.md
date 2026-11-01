# Pigeonhole — specification

## 1. The fleet as Pigeonhole uses it

`[database.fleet] mode = "slot"`, `max_open = 64`, `pool_size = 2`,
`create_on_demand = true` (the slot-mode default; slots are a closed set of
16,384). The control database holds sessions and the pigeon registry.

Observable (`crates/pigeonhole-app/tests/app.rs`):

| # | Property | Test |
|---|---|---|
| 1.1 | Hatching a pigeon registers it and creates exactly its slot's file. Unknown pigeon `404`, invalid name `400`. | `hatching_registers_the_pigeon_and_creates_only_its_slot_file` |
| 1.2 | Two pigeons that hash to one slot share one file and never see each other's notes through the tenant-scoped repository; the pigeon page reports both the file's rows and the pigeon's. | `neighbours_share_a_file_and_never_see_each_other` |
| 1.3 | A flock of 400 lands every note; the slot files are exactly the slots the names hash to; the census is clean. | `a_flock_lands_every_note_in_the_right_file_and_the_census_agrees` |
| 1.4 | A row planted in a file its tenant does not hash to is named as a stray, with both slots. | `the_census_finds_a_planted_stray` |
| 1.5 | The board encodes all 16,384 slots and the slot page lists a slot's pigeons. | `the_board_draws_every_slot_and_the_slot_page_lists_its_pigeons` |
| 1.6 | `slot_of` (the board and census) agrees with the fleet's own `key_for` (requests) for every valid name. 2,000 generated names. | `tests/router.rs` |

## 2. The board (`crates/holes/src/grid.rs`, `verus/src/grid.rs`)

Slot `s` is drawn at `(s / 128, s % 128)`.

- **2.1** `cell(s)` lands on the board and `row · 128 + col == s` — formal.
- **2.2** `slot_at(r, c)` is `Some` exactly on the board, in range, and equal to `r · 128 + c` — formal.
- **2.3** `slot_at(cell(s)) == s` — formal.
- **2.4** Distinct cells never draw the same slot — formal.

Runtime: unit tests on the corners, an exhaustive check that every slot has
exactly one cell, and a property test of the round trip.

## 3. The census (`crates/holes/src/census.rs`, `verus/src/census.rs`)

For each slot file, `Shards::each_shard` reports each tenant's note count.
`Census::fold` adds them up and checks each tenant against the router.

- **3.1** `files == #reports`, `tenants == Σ`, `notes == Σ` — formal, for the
  executable fold over bounded reports.
- **3.2** The sums are unchanged by moving the last report to the front (the
  step any permutation decomposes into) — formal.
- **3.3** A row whose tenant routes elsewhere is a stray — unit test and the
  planted-stray integration test.
- **3.4** Totals conserve under any order — property test.

## 4. Observed live (cloud, debug build)

3,000-pigeon flock: 16 s, 0 failures, **2,728 slot files against a predicted
2,742**, 250 shared, at most 4 pigeons in one slot. Peak open **437 against
`max_open = 64`** (the soft ceiling again; see Apiary's FINDINGS). Census of
2,728 files: 6.6 s, clean.

## 5. Scoreboard — none of the Verus has been run yet

| Obligation | File | Confidence | Why |
|---|---|---|---|
| 2.1–2.4 | `grid.rs` | medium-high | Division and modulus by 128 lean on `vstd::arithmetic::div_mod` lemmas and `nonlinear_arith`; the usual place for a first run to need a hint. |
| 3.1 | `census.rs` | medium | Loop invariant over `subrange` sums with `drop_last`; the bound lemma keeps the u64 adds in range. Expect to nudge one `=~=` extensionality assert. |
| 3.2 | `census.rs` | medium-high | Short induction on `drop_last`. |
