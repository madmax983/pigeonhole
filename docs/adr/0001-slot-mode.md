# ADR 0001 — Slot mode, with a control-side registry

Status: accepted, 2026-10-31

## Decision

- `mode = "slot"`: tenants hash into 16,384 slot files and share them.
- `create_on_demand = true` (the slot-mode default): a slot is created when
  its first pigeon lands. Slots are a closed set, so this cannot be abused the
  way creating a file per requested tenant id could.
- In slot mode the fleet accepts any string as a routing key, because the id
  never becomes a file name. Pigeonhole validates names itself
  (`TenantName`), with the tenant-mode rules, so the same names would work on a
  tenant fleet.
- A registry of pigeons (name → slot) lives in the control database. The board
  draws from it; the census checks the files against it.
- `slot_of` calls autumn-web's own `slot_for_key`, so the board and census can
  never route differently from requests (and `tests/router.rs` checks it
  against `DatabaseFleet::key_for`).
- `max_open = 64`.

## Consequences

- Isolation inside a file is the repository's `tenant_scoped` filter, not a
  file boundary. Raw SQL on a `ShardedDb` sees every tenant in the file — the
  pigeon page does that on purpose, to count the rows you cannot see.
- A census reads every file on disk; at 2,728 files it takes ~7 s and holds
  hundreds of files open at once (the soft cap).
