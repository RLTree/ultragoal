mod context {
    pub use ultragoal::context::*;
}

#[allow(dead_code, unused_imports)]
mod inventory {
    pub use ultragoal::inventory::*;
}

#[path = "inventory_contract_cases/agent_manifest.rs"]
mod agent_manifest;
#[path = "inventory_contract_cases/agent_routes.rs"]
mod agent_routes;
#[path = "inventory_contract_cases/command_activation.rs"]
mod command_activation;
#[path = "inventory_contract_cases/compatibility_routes.rs"]
mod compatibility_routes;
#[path = "inventory_contract_cases/context_scopes.rs"]
mod context_scopes;
#[path = "inventory_contract_cases/contract_integrity.rs"]
mod contract_integrity;
#[path = "inventory_contract_cases/fixtures.rs"]
mod fixtures;
#[path = "inventory_contract_cases/generated_disposition.rs"]
mod generated_disposition;
#[path = "inventory_contract_cases/generated_regeneration.rs"]
mod generated_regeneration;
#[path = "inventory_contract_cases/legacy_limits.rs"]
mod legacy_limits;
#[path = "inventory_contract_cases/legacy_scope.rs"]
mod legacy_scope;
#[path = "inventory_contract_cases/plugin_hooks.rs"]
mod plugin_hooks;
#[path = "inventory_contract_cases/plugin_manifest.rs"]
mod plugin_manifest;
#[path = "inventory_contract_cases/plugin_manifest_semantics.rs"]
mod plugin_manifest_semantics;
#[path = "inventory_contract_cases/repository_fixture.rs"]
mod repository_fixture;
#[path = "inventory_contract_cases/routing_transitions.rs"]
mod routing_transitions;
#[path = "inventory_contract_cases/schema_references.rs"]
mod schema_references;

use context::LiveContext;
use inventory::{ActiveStatus, AuthorityState, InventoryBuilder};

#[test]
fn current_live_catalog_reports_contract_definitions_and_missing_topology() {
    let root = repository_fixture::live_root();
    let context = LiveContext::build(repository_fixture::inventory_request(&root)).unwrap();
    let catalog = InventoryBuilder::new(&context).build().unwrap();
    for id in [
        "PS-SOURCE",
        "HCT-CONTEXT",
        "CL-SOURCE",
        "REQ-STATE-001",
        "SKILL:harness-ultragoal",
        "AGENT:repo-recon",
        "COMMAND:inspect",
    ] {
        assert!(
            catalog.entries().iter().any(|entry| entry.stable_id == id),
            "missing {id}"
        );
    }
    for skill in [
        "harness-ultragoal",
        "repository-fit",
        "routine-work",
        "diagnose-and-observe",
        "goal-run",
        "prove",
        "improve-and-maintain",
        "product-journey-review",
    ] {
        let entry = catalog
            .entries()
            .iter()
            .find(|entry| entry.stable_id == format!("SKILL:{skill}"))
            .unwrap();
        assert_eq!(entry.active_status, ActiveStatus::Active);
        assert_eq!(entry.authority_state, AuthorityState::Canonical);
    }
    for agent in [
        "repo-recon",
        "research-verifier",
        "product-journey-reviewer",
        "claim-falsifier",
        "security-reviewer",
        "orchestration-recovery-reviewer",
    ] {
        assert_eq!(
            catalog
                .entries()
                .iter()
                .find(|entry| entry.stable_id == format!("AGENT:{agent}"))
                .unwrap()
                .active_status,
            ActiveStatus::Active
        );
    }
    for command in [
        "inspect", "next", "fit", "check", "diagnose", "prove", "observe", "package", "eval",
        "migrate",
    ] {
        assert_eq!(
            catalog
                .entries()
                .iter()
                .find(|entry| entry.stable_id == format!("COMMAND:{command}"))
                .unwrap()
                .active_status,
            ActiveStatus::Candidate
        );
    }
    assert_eq!(catalog.source_registry_counts().get("surfaces"), Some(&24));
    assert_eq!(catalog.source_registry_counts().get("tools"), Some(&12));
    assert_eq!(catalog.source_registry_counts().get("claims"), Some(&14));
    assert_eq!(
        catalog.source_registry_counts().get("requirements"),
        Some(&105)
    );
    assert_eq!(
        catalog.source_registry_counts().get("canonical_skills"),
        Some(&8)
    );
    assert_eq!(
        catalog
            .source_registry_counts()
            .get("read_only_agent_roles"),
        Some(&6)
    );
    assert_eq!(
        catalog.source_registry_counts().get("cli_command_groups"),
        Some(&10)
    );
    assert_eq!(
        catalog.source_registry_counts().get("contract_sources"),
        Some(&26)
    );
    let missing_topology = catalog
        .entries()
        .iter()
        .filter(|entry| {
            entry.active_status == ActiveStatus::Missing
                && matches!(entry.kind.as_str(), "skill" | "agent" | "command-group")
        })
        .collect::<Vec<_>>();
    assert!(
        missing_topology
            .iter()
            .all(|entry| matches!(entry.kind.as_str(), "agent" | "command-group"))
    );
    assert_eq!(
        catalog
            .findings()
            .iter()
            .filter(|finding| finding.code == "candidate_component_not_active")
            .count(),
        10
    );
    assert_eq!(
        catalog
            .entries()
            .iter()
            .filter(|entry| {
                entry.stable_id.starts_with("LEGACY-SKILL:")
                    && matches!(
                        entry.kind.as_str(),
                        "legacy-skill-route-witness" | "compatibility-route-retained"
                    )
            })
            .count(),
        14
    );
    assert!(
        !catalog
            .findings()
            .iter()
            .any(|finding| finding.code == "invalid_compatibility_route_wrapper")
    );
    assert!(catalog.has_error_findings());
    for api in [
        "API:LiveContext::build",
        "API:EffectClass",
        "API:CapabilitySet",
        "API:CandidateIdentity",
        "API:InventoryBuilder",
        "API:AuthorityCatalog",
        "API:GeneratedSurfaceIndex",
        "API:PackageSnapshot",
        "API:InstallSnapshot",
        "API:MarketplaceSnapshot",
        "API:DiscoveryObservation",
        "API:RuntimeObservation",
    ] {
        assert_eq!(
            catalog
                .entries()
                .iter()
                .find(|entry| entry.stable_id == api)
                .unwrap()
                .active_status,
            ActiveStatus::Active
        );
    }
    assert_eq!(
        catalog
            .entries()
            .iter()
            .find(|entry| entry.stable_id == "API:EvaluationSpec")
            .unwrap()
            .active_status,
        ActiveStatus::Missing
    );
    let hct_eval = catalog
        .entries()
        .iter()
        .find(|entry| entry.stable_id == "HCT-EVAL")
        .unwrap();
    assert_eq!(hct_eval.kind, "custom-tool-definition");
    assert_eq!(hct_eval.active_status, ActiveStatus::Definition);
    let legacy = catalog
        .entries()
        .iter()
        .filter(|entry| entry.authority_state == AuthorityState::Legacy)
        .collect::<Vec<_>>();
    assert!(!legacy.is_empty());
    assert!(
        legacy
            .iter()
            .all(|entry| entry.active_status == ActiveStatus::Active)
    );
    let parallel_ids = catalog
        .findings()
        .iter()
        .filter(|finding| finding.code == "parallel_authority")
        .filter_map(|finding| finding.entry_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    let retained_ids = catalog
        .findings()
        .iter()
        .filter(|finding| finding.code == "compatibility_route_retained")
        .filter_map(|finding| finding.entry_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    let pending_ids = catalog
        .findings()
        .iter()
        .filter(|finding| finding.code == "sole_current_authority_pending_migration")
        .filter_map(|finding| finding.entry_id.as_deref())
        .collect::<std::collections::BTreeSet<_>>();
    assert!(legacy.iter().all(|entry| {
        [
            parallel_ids.contains(entry.stable_id.as_str()),
            retained_ids.contains(entry.stable_id.as_str()),
            pending_ids.contains(entry.stable_id.as_str()),
        ]
        .into_iter()
        .filter(|classified| *classified)
        .count()
            == 1
    }));
}
