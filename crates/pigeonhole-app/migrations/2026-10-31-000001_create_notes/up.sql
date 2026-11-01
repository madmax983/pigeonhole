-- Notes. Applied to every slot file, which any number of tenants share:
-- `tenant_id` is what keeps them apart, and every repository query filters
-- on it (`#[repository(tenant_scoped)]`).
CREATE TABLE notes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    body TEXT NOT NULL,
    tenant_id TEXT,
    created_at TIMESTAMP NOT NULL
);
CREATE INDEX notes_tenant ON notes (tenant_id);
