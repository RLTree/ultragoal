mod context {
    pub use ultragoal::context::*;
}

#[allow(dead_code, unused_imports)]
mod inventory {
    pub use ultragoal::inventory::*;
}

#[path = "inventory_contract_cases/contract_integrity.rs"]
mod contract_integrity;
#[path = "inventory_contract_cases/fixtures.rs"]
mod fixtures;
#[path = "inventory_contract_cases/generated_regeneration.rs"]
mod generated_regeneration;
#[path = "inventory_contract_cases/legacy_limits.rs"]
mod legacy_limits;
#[path = "inventory_contract_cases/repository_fixture.rs"]
mod repository_fixture;
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
        assert!(
            catalog
                .entries()
                .iter()
                .any(|entry| entry.stable_id == format!("SKILL:{skill}"))
        );
    }
    for agent in [
        "repo-recon",
        "research-verifier",
        "product-journey-reviewer",
        "claim-falsifier",
        "security-reviewer",
        "orchestration-recovery-reviewer",
    ] {
        assert!(
            catalog
                .entries()
                .iter()
                .any(|entry| entry.stable_id == format!("AGENT:{agent}"))
        );
    }
    for command in [
        "inspect", "next", "fit", "check", "diagnose", "prove", "observe", "package", "eval",
        "migrate",
    ] {
        assert!(
            catalog
                .entries()
                .iter()
                .any(|entry| entry.stable_id == format!("COMMAND:{command}"))
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
    assert_eq!(missing_topology.len(), 24);
    assert!(catalog.has_error_findings());
    for api in [
        "API:LiveContext::build",
        "API:EffectClass",
        "API:CapabilitySet",
        "API:CandidateIdentity",
        "API:InventoryBuilder",
        "API:AuthorityCatalog",
        "API:GeneratedSurfaceIndex",
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
    assert_eq!(
        catalog
            .findings()
            .iter()
            .filter(|finding| finding.code == "parallel_authority")
            .count(),
        legacy.len()
    );
}
