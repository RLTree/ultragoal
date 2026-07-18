use super::compatibility::RETAINED_KIND;
use super::routing::RoutingData;
use super::types::{
    ActiveStatus, AuthorityCatalog, AuthorityState, InventoryClosureStatus, InventoryEntry,
    InventoryFinding, InventoryFindingDisposition,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

mod duplicates;
use duplicates::{duplicate_paths, duplicate_stable_ids};

include!("merge_one.rs");

include!("candidate_components.rs");

#[cfg(test)]
mod closure_tests {
    use super::*;
    use crate::inventory::types::{FindingSeverity, InventoryClosureState};

    fn finding(code: &str, severity: FindingSeverity) -> InventoryFinding {
        InventoryFinding {
            code: code.to_owned(),
            severity,
            entry_id: None,
            relative_path: None,
            message: "test finding".to_owned(),
        }
    }

    #[test]
    fn only_exact_warning_codes_are_open_migration_obligations() {
        for code in [
            "sole_current_authority_pending_migration",
            "compatibility_route_retained",
        ] {
            assert_eq!(
                finding(code, FindingSeverity::Warning).closure_disposition(),
                InventoryFindingDisposition::OpenMigrationObligation
            );
            assert_eq!(
                finding(code, FindingSeverity::Error).closure_disposition(),
                InventoryFindingDisposition::Blocking
            );
        }
        for code in ["candidate_component_not_active", "unknown_warning_code"] {
            assert_eq!(
                finding(code, FindingSeverity::Warning).closure_disposition(),
                InventoryFindingDisposition::Blocking
            );
        }
        assert_eq!(
            finding("informational", FindingSeverity::Info).closure_disposition(),
            InventoryFindingDisposition::Informational
        );
    }

    #[test]
    fn zero_blockers_close_with_explicit_open_obligations() {
        let status = InventoryClosureStatus::new(
            "catalog".to_owned(),
            "context".to_owned(),
            BTreeMap::new(),
            BTreeMap::from([
                ("compatibility_route_retained".to_owned(), 14),
                ("sole_current_authority_pending_migration".to_owned(), 16),
            ]),
        );
        assert_eq!(status.state(), InventoryClosureState::Closed);
        assert!(status.is_closed());
        assert_eq!(status.blocker_count(), 0);
        assert_eq!(status.open_obligation_count(), 30);
    }
}
