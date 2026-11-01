//! The board and the census route with `slot_of`; requests route with the
//! fleet. They must agree for every name, or the census would call correctly
//! placed rows strays (or miss real ones).

mod common;

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
}
