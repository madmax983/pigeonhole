# Findings: autumn-web 0.9's slot fleet, from building Pigeonhole

See Apiary's `docs/FINDINGS.md` for the soft `max_open` ceiling, migration-set
scoping and the other notes; they all apply here too (peak open 437 against a
cap of 64 during a flock). New in slot mode:

1. **The slow-query log misattributes a fan-out.** A census
   (`Shards::each_shard` over 2,728 files, 6.6 s) logs
   `slow database query, route: POST /census, sql: "SELECT ?", duration_ms: 6653`
   — the whole fan-out is charged to one statement that is not in the code.

2. **The connection-budget warning speaks Postgres to a SQLite fleet.** Boot
   warns that "the control topology and all shard pools together may open 138
   connections … Ensure each Postgres server's max_connections …". For a
   SQLite fleet the real budget is file descriptors
   (`max_open × (2 × pool_size + 1)` per the fleet guide), not a server's
   `max_connections`.

3. **Slot mode accepts any routing key.** `DatabaseFleet::key_for` validates
   tenant ids only in tenant mode, which is correct (the id never names a
   file in slot mode), but it means an app moving from tenant to slot mode
   silently loses id validation. Worth one line in the guide.
