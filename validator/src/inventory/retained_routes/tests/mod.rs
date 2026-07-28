use super::{ROUTE_COUNT, routes};
use crate::inventory::behavioral_role;
use serde_json::Value;
use std::path::Path;

#[test]
fn former_pending_routes_are_a_closed_behavioral_role_guard_set() {
    assert_eq!(ROUTE_COUNT, 10);
    for spec in routes() {
        let binding = behavioral_role::for_path(Path::new(spec.path))
            .expect("former pending route must have a current behavioral role");
        assert!(behavioral_role::legacy_route_targets_active_role(
            Some(spec.stable_id),
            Some(binding.relative_path)
        ));
    }
}

#[test]
fn active_behavioral_roles_are_absent_from_the_migration_registry() {
    let bytes = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("migration/authority-routes.json"),
    )
    .unwrap();
    let registry: Value = serde_json::from_slice(&bytes).unwrap();
    let route_ids = registry["routes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|route| route["route_id"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(route_ids.len(), 28);
    for spec in routes() {
        assert!(!route_ids.contains(spec.route_id));
    }
}
