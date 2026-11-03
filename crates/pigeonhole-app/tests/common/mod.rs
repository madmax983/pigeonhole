//! Shared fixtures: a real slot fleet in a temporary directory, migrated with
//! the app's own migrations, a control database beside it, and an app around
//! both.
#![allow(dead_code)]

use std::path::Path;
use std::time::Duration;

use autumn_web::config::{AutumnConfig, DatabaseConfig, DatabaseFleetConfig};
use autumn_web::db::fleet::DatabaseFleet;
use autumn_web::db::{RuntimeConnection, create_pool};
use autumn_web::fleet_layout::FleetMode;
use autumn_web::migrate::{EmbeddedMigrations, embed_migrations};
use autumn_web::prelude::*;
use autumn_web::reexports::{diesel, diesel_async};
use autumn_web::test::{TestApp, TestClient};
use diesel_async::RunQueryDsl as _;
use diesel_async::pooled_connection::deadpool::Pool;
use pigeonhole_app::routes;

/// The app's migrations, exactly as `main.rs` embeds them.
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

/// A control database in `dir`, with the registry.
pub async fn control_pool(dir: &Path) -> Pool<RuntimeConnection> {
    control_pool_sized(dir, DatabaseConfig::default().pool_size).await
}

/// A control database in `dir` with `pool_size` connections.
pub async fn control_pool_sized(dir: &Path, pool_size: usize) -> Pool<RuntimeConnection> {
    let pool = create_pool(&DatabaseConfig {
        url: Some(format!("sqlite://{}", dir.join("control.db").display())),
        pool_size,
        ..Default::default()
    })
    .expect("control pool builds")
    .expect("url configured");
    let mut conn = pool.get().await.expect("control connection");
    for sql in [
        "CREATE TABLE notes (id INTEGER PRIMARY KEY AUTOINCREMENT, body TEXT NOT NULL, tenant_id TEXT, created_at TIMESTAMP NOT NULL)",
        "CREATE TABLE pigeons (name TEXT PRIMARY KEY NOT NULL, slot INTEGER NOT NULL, created_at TIMESTAMP NOT NULL)",
    ] {
        diesel::sql_query(sql)
            .execute(&mut *conn)
            .await
            .expect("control table");
    }
    pool
}

/// A slot fleet under `dir/fleet` that keeps `max_open` files open.
pub fn fleet(dir: &Path, max_open: usize) -> DatabaseFleet {
    DatabaseFleet::builder(DatabaseFleetConfig {
        mode: FleetMode::Slot,
        root: dir.join("fleet").display().to_string(),
        path: None,
        max_open,
        pool_size: 2,
        create_on_demand: Some(true),
        idle_close_secs: 0,
        restore_missing: false,
        ownership: None,
    })
    .migrations("app", MIGRATIONS)
    .min_residency(Duration::ZERO)
    .build()
    .expect("fleet builds")
}

/// Header tenancy, no CSRF.
pub fn config() -> AutumnConfig {
    let mut config = AutumnConfig {
        profile: Some("test".into()),
        ..AutumnConfig::default()
    };
    config.security.csrf.enabled = false;
    config.tenancy.enabled = true;
    "header".clone_into(&mut config.tenancy.source);
    "x-tenant-id".clone_into(&mut config.tenancy.header_name);
    config.tenancy.public_paths = [
        "/",
        "/_board.json",
        "/_panel",
        "/stream",
        "/pigeons",
        "/enter",
        "/flock",
        "/slot",
        "/census",
        "/about",
        "/pigeonhole.js",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    config
}

/// The whole app around `fleet`.
pub fn app(control: Pool<RuntimeConnection>, fleet: DatabaseFleet) -> TestClient {
    TestApp::new()
        .config(config())
        .routes(routes![
            routes::board::index,
            routes::board::picture,
            routes::board::panel,
            routes::board::found,
            routes::board::enter,
            routes::board::release,
            routes::board::slot,
            routes::pigeon::index,
            routes::pigeon::notes,
            routes::pigeon::add_note,
            routes::census::index,
            routes::census::take,
            routes::about::show,
            routes::assets::script,
        ])
        .with_db(control)
        .with_fleet(fleet)
        .state_initializer(|state| {
            pigeonhole_app::install(state).expect("pigeonhole installs");
        })
        .build()
}

/// Two flock names that hash to the same slot (a neighbour pair).
pub fn neighbours() -> (String, String) {
    let mut seen = std::collections::HashMap::new();
    for i in 0..20_000 {
        let n = holes::TenantName::flock(i).to_string();
        let s = pigeonhole_app::slot_of(&n);
        if let Some(prev) = seen.insert(s, n.clone()) {
            return (prev, n);
        }
    }
    unreachable!("20,000 names into 16,384 slots must collide")
}
