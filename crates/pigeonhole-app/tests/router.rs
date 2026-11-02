//! The board and the census route with `slot_of`; requests route with the
//! fleet. They must agree for every name, or the census would call correctly
//! placed rows strays (or miss real ones).

mod common;

use autumn_web::fleet_layout::TenantDbId;
use holes::TenantName;
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 2_000, ..ProptestConfig::default() })]

    #[test]
    fn slot_of_agrees_with_the_fleets_key(raw in "[a-z][a-z0-9-]{0,38}[a-z0-9]") {
        let tmp = tempfile::tempdir().unwrap();
        let fleet = common::fleet(tmp.path(), 2);
        if let Ok(name) = TenantName::parse(&raw) {
            let key = fleet.key_for(name.as_str()).unwrap();
            prop_assert_eq!(key.name(), pigeonhole_app::slot_of(name.as_str()).db_name());
            prop_assert!(fleet.path_of(&key).starts_with(tmp.path().join("fleet")));
        }
    }

    /// FINDINGS 3: slot mode takes any routing key, so the app checks names
    /// itself. Every pigeon name is also a valid tenant-mode id: a move to
    /// tenant mode refuses no pigeon.
    #[test]
    fn every_pigeon_name_is_a_tenant_mode_id(raw in "[ -~]{0,48}|[a-z][a-z0-9-]{0,38}[a-z0-9]") {
        if let Ok(name) = TenantName::parse(&raw) {
            prop_assert!(TenantDbId::parse(name.as_str()).is_ok(), "{name}");
        }
    }
}

/// The same, for every flock name a 20,000-bird flock uses.
#[test]
fn every_flock_name_is_a_tenant_mode_id() {
    for i in 0..20_000 {
        let name = TenantName::flock(i);
        assert!(TenantDbId::parse(name.as_str()).is_ok(), "{name}");
    }
}
