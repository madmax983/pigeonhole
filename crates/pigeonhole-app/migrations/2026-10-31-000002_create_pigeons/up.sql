-- The registry: every pigeon and the slot its name hashes to. Control-plane
-- state; it also lands, empty, in every slot file, because autumn-web 0.9
-- applies every app migration set to the control database and to every fleet
-- database (see docs/adr/0002-one-migration-set.md).
CREATE TABLE pigeons (
    name TEXT PRIMARY KEY NOT NULL,
    slot INTEGER NOT NULL,
    created_at TIMESTAMP NOT NULL
);
CREATE INDEX pigeons_slot ON pigeons (slot);
