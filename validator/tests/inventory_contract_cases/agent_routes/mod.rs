use crate::context::LiveContext;
use crate::inventory::{ActiveStatus, AuthorityState, InventoryBuilder};
use crate::repository_fixture::{TestRepo, inventory_request, live_root};
use serde_json::json;
use std::fs;

#[path = "reader_false_pass/mod.rs"]
mod reader_false_pass;
#[path = "registry_fixture/mod.rs"]
mod registry_fixture;

use registry_fixture::{
    CASES, Case, READER_PROOF, catalog, entry, prepare as prepare_without_agent_discovery_readers,
    registry, route_id, target_path, write_registry,
};

const CURRENT_AGENT_DISCOVERY_READERS: [&str; 3] = [
    "validator/src/plugin_product/agent_discovery/host.rs",
    "validator/src/plugin_product/agent_discovery/model.rs",
    "validator/src/plugin_product/agent_discovery/source.rs",
];

fn prepare(repo: &TestRepo, cases: &[Case], reader_proof: bool) {
    prepare_without_agent_discovery_readers(repo, cases, reader_proof);
    if reader_proof {
        for path in CURRENT_AGENT_DISCOVERY_READERS {
            repo.write(path, &fs::read(live_root().join(path)).unwrap());
        }
    }
}

#[test]
fn all_fourteen_exact_agent_routes_demote_only_to_preserved_context() {
    let repo = TestRepo::new("agent-routes-all");
    prepare(&repo, &CASES, true);
    repo.commit();

    let catalog = catalog(&repo);
    for case in CASES {
        let entry = entry(&catalog, case);
        let route_findings = catalog
            .findings()
            .iter()
            .filter(|finding| {
                matches!(
                    finding.code.as_str(),
                    "invalid_agent_route_transition"
                        | "duplicate_component_path"
                        | "duplicate_stable_id"
                        | "renamed_required_component"
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            entry.authority_state,
            AuthorityState::Context,
            "{}: {:#?}",
            case.0,
            route_findings
        );
        assert_eq!(entry.active_status, ActiveStatus::ContextOnly);
        assert!(entry.references.iter().any(|reference| reference == case.1));
        assert!(entry.input_provenance.iter().any(|row| row == case.0));
        assert!(
            entry
                .input_provenance
                .iter()
                .any(|row| row == &target_path(case))
        );
        assert!(entry.input_provenance.iter().any(|row| row == READER_PROOF));
        assert!(repo.root.join(case.0).is_file());
        assert!(!catalog.findings().iter().any(|finding| {
            finding.entry_id.as_deref() == Some(entry.stable_id.as_str())
                && matches!(
                    finding.code.as_str(),
                    "parallel_authority" | "invalid_agent_route_transition"
                )
        }));
    }
    assert_eq!(
        catalog
            .findings()
            .iter()
            .filter(|finding| finding.code == "verified_agent_route_context")
            .count(),
        14
    );
}

#[test]
fn source_target_reader_and_positive_reader_drift_leave_legacy_active() {
    for mutation in ["source", "target", "reader", "positive-reader"] {
        let case = CASES[0];
        let repo = TestRepo::new(&format!("agent-route-{mutation}-drift"));
        prepare(&repo, &[case], mutation != "reader");
        match mutation {
            "source" => repo.write(case.0, b"retained descriptor changed\n"),
            "target" => {
                let target_path = target_path(case);
                let mut bytes = fs::read(repo.root.join(&target_path)).unwrap();
                bytes.extend_from_slice(b"\n");
                repo.write(&target_path, &bytes);
            }
            "positive-reader" => {
                let path = "validator/src/cli/control/plane/registry/agent_rows.rs";
                let text = fs::read_to_string(repo.root.join(path)).unwrap();
                let changed = text.replace(
                    "\"role\": role_name,",
                    "\"role\": role_name, \"custom_agent_path\": \"custom-agents/reintroduced.toml\",",
                );
                assert_ne!(text, changed);
                repo.write(path, changed.as_bytes());
                assert_eq!(
                    fs::read(repo.root.join(READER_PROOF)).unwrap(),
                    fs::read(live_root().join(READER_PROOF)).unwrap()
                );
            }
            _ => {}
        }
        repo.commit();
        let catalog = catalog(&repo);
        let entry = entry(&catalog, case);
        assert_eq!(entry.authority_state, AuthorityState::Legacy);
        assert_eq!(entry.active_status, ActiveStatus::Active);
        assert!(catalog.findings().iter().any(|finding| {
            finding.code == "invalid_agent_route_transition"
                && finding.entry_id.as_deref() == Some(entry.stable_id.as_str())
        }));
        assert!(catalog.findings().iter().any(|finding| {
            finding.code == "parallel_authority"
                && finding.entry_id.as_deref() == Some(entry.stable_id.as_str())
        }));
    }
}

#[test]
fn forged_registry_transition_is_rejected_before_application() {
    for mutation in ["matcher", "target", "proof", "cleanup"] {
        let case = CASES[0];
        let repo = TestRepo::new(&format!("agent-route-forged-{mutation}"));
        prepare(&repo, &[case], true);
        let mut value = registry(&repo);
        let row = value["routes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["route_id"] == route_id(case))
            .unwrap();
        match mutation {
            "matcher" => row["match"] = json!({"kind": "legacy-agent-authority"}),
            "target" => row["canonical_target"] = json!("AGENT:repo-recon"),
            "proof" => row["transition"]["proof_refs"][0] = json!("agents/forged.md"),
            _ => row["transition"]["physical_cleanup_state"] = json!("blocked-by-OD-009"),
        }
        write_registry(&repo, &value);
        repo.commit();
        let context = LiveContext::build(inventory_request(&repo.root)).unwrap();
        assert!(
            InventoryBuilder::new(&context)
                .build()
                .unwrap_err()
                .to_string()
                .contains("exact compiled proof")
        );
    }
}
