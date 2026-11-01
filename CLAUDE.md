# CLAUDE.md — Pigeonhole

A test bed for autumn-web 0.9's SQLite fleet in slot mode. Read `PITCH.md`,
then `docs/SPEC.md`.

- `crates/holes` — pure engine (no IO, async or clock): names, the 128 × 128
  board, the census fold, the flock plan. Rules go here, with property tests.
- `crates/pigeonhole-app` — HTTP (Maud + htmx + one canvas script), the
  `DatabaseFleet`, the control-side pigeon registry.
- `verus/src/*.rs` — standalone Verus models of `grid` and the census fold.
  Change the runtime and the model together.

Rules that matter here:

- Route with `slot_of` (which calls autumn-web's `slot_for_key`); never
  re-implement the hash. `tests/router.rs` checks it against the fleet.
- Raw SQL on a `ShardedDb` sees every tenant in the slot file. Use the
  tenant-scoped repository for anything a tenant reads; raw SQL only for
  counts that are meant to cross tenants (the pigeon page, the census).
- Do not hold a `FleetDatabase` across requests.
- Do not `use diesel::prelude::*` beside the model macros (E0034).
- Every route must be in `main.rs` and `routes::ALL`; every route is
  deliberately `#[public]` (no login).

Gates: `./verify.sh` (authoritative, on the Mac). In the cloud:
`cargo fmt --all -- --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace`.
