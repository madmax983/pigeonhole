//! HTTP surface.
//!
//! Nothing here has a login: the session holds which pigeon you are, and
//! anybody can be any pigeon. Every route is therefore `#[public]` on
//! purpose, which `autumn routes audit` requires to be declared.

pub mod about;
pub mod assets;
pub mod board;
pub mod census;
pub mod layout;
pub mod pigeon;

/// Every handler `main.rs` must register (checked by a test in `lib.rs`).
pub const ALL: &[&str] = &[
    "routes::board::index",
    "routes::board::picture",
    "routes::board::panel",
    "routes::board::stream",
    "routes::board::found",
    "routes::board::enter",
    "routes::board::release",
    "routes::board::slot",
    "routes::pigeon::index",
    "routes::pigeon::notes",
    "routes::pigeon::add_note",
    "routes::census::index",
    "routes::census::take",
    "routes::about::show",
    "routes::assets::script",
];

/// The session key the tenancy middleware reads the tenant from. Matches
/// `[tenancy] session_key` in `autumn.toml`.
pub const TENANT_KEY: &str = "tenant_id";
