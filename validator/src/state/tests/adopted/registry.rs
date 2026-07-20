use crate::context::{BuildRequest, EffectClass, LiveContext};
use crate::inventory::{
    ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256, InventoryBuilder,
};
use crate::state::adopted::{derive_adopted, issue_adopted};
use crate::state::adopted_registry::{load_claims_for_test, validate_registry_for_test};
use crate::state::{NextActionKind, ProductGoalState, StateError};
use std::fs;
use std::path::Path;
use std::process::Command;

const HANDOFF: &[u8] = include_bytes!(
    "../../../../../docs/ultragoal-contract-2026-07-successor-v2/FINAL-HANDOFF-MANIFEST.sha256"
);
const MANIFEST: &[u8] = include_bytes!(
    "../../../../../docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CONTRACT_MANIFEST.json"
);
const CLAIMS: &[u8] = include_bytes!(
    "../../../../../docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT/CLAIM_REGISTRY.json"
);
const HANDOFF_SHA256: &str = "d61c897a68d3aa985996f595a17c80f49e0730d07434b6b81de36878ef28dc51";

#[test]
fn adopted_registry_chain_loads_exact_fourteen_claims() {
    let loaded = load_claims_for_test(HANDOFF, MANIFEST, CLAIMS, HANDOFF_SHA256).unwrap();
    assert_eq!(
        loaded.claim_registry_sha256,
        "sha256:67e81c4eabe87d16d816a3d7dad352dc21a4a1994dd83b6582e9e4ba5eaa61bc"
    );
    assert_eq!(loaded.registry.claims.len(), 14);
    assert!(loaded.registry.claims.iter().any(|claim| {
        claim.claim_id == "CL-COMPLETION" && claim.allowed_ceiling_on_pass == "complete"
    }));
    assert_eq!(
        loaded.registry.claim_topological_order,
        loaded
            .registry
            .claims
            .iter()
            .map(|claim| claim.claim_id.clone())
            .collect::<Vec<_>>()
    );
}

#[test]
fn any_manifest_chain_mutation_fails_closed() {
    for target in 0..3 {
        let mut handoff = HANDOFF.to_vec();
        let mut manifest = MANIFEST.to_vec();
        let mut claims = CLAIMS.to_vec();
        match target {
            0 => handoff[0] ^= 1,
            1 => manifest[0] ^= 1,
            _ => claims[0] ^= 1,
        }
        assert!(matches!(
            load_claims_for_test(&handoff, &manifest, &claims, HANDOFF_SHA256),
            Err(StateError::InvalidCatalog(_))
        ));
    }
}

#[test]
fn semantic_claim_graph_mutations_fail_closed() {
    let original: serde_json::Value = serde_json::from_slice(CLAIMS).unwrap();
    for mutation in 0..4 {
        let mut value = original.clone();
        match mutation {
            0 => value["claim_topological_order"]
                .as_array_mut()
                .unwrap()
                .swap(0, 13),
            1 => {
                value["claims"][1]["prerequisite_claim_ids"] = serde_json::json!(["CL-COMPLETION"])
            }
            2 => value["claims"][0]["false_pass_controls"] = serde_json::json!([]),
            _ => value["claims"][0]["claim_decision_owner"] = serde_json::json!("OWN-CLI"),
        }
        assert!(matches!(
            validate_registry_for_test(&serde_json::to_vec(&value).unwrap()),
            Err(StateError::InvalidCatalog(_))
        ));
    }
}

#[test]
fn live_issuer_covers_exact_inventory_codes_and_cannot_grant_completion() {
    let live = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .to_path_buf();
    let root = std::env::temp_dir().join(format!(
        "ultragoal-adopted-state-{}-{}",
        std::process::id(),
        std::thread::current().name().unwrap_or("test")
    ));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("tracked.txt"), b"tracked\n").unwrap();
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "state@example.invalid"]);
    git(&root, &["config", "user.name", "Adopted State"]);
    copy_authority_inputs(&live, &root);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "fixture"]);
    let context = LiveContext::build(
        BuildRequest::new(&root)
            .with_effect(EffectClass::Read)
            .bind_non_secret_configuration(
                ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
                ADOPTED_HANDOFF_MANIFEST_SHA256,
            ),
    )
    .unwrap();
    let inventory = InventoryBuilder::new(&context).build().unwrap();

    let policy = issue_adopted(&context, &inventory).unwrap();
    assert_ne!(policy.catalog_id(), inventory.catalog_id());
    let state = derive_adopted(&context, &inventory).unwrap();
    assert_eq!(state.product_goal(), ProductGoalState::Operating);
    assert_eq!(state.next_action().kind, NextActionKind::Command);
    assert_eq!(state.next_action().effect, EffectClass::Read);
    assert_eq!(
        state.next_action().command_id.as_deref(),
        Some("migrate-plan")
    );
    assert!(state.findings().iter().any(|finding| {
        finding.code == "dependency-missing"
            && finding.dependency_ids.contains("reconciliation-kernel")
    }));
    let completion = state
        .claim_ceilings()
        .iter()
        .find(|ceiling| ceiling.claim_id() == "CL-COMPLETION")
        .unwrap();
    assert!(completion.dimensions().is_empty());
    assert_eq!(
        state
            .claim_ceilings()
            .iter()
            .filter(|ceiling| !ceiling.dimensions().is_empty())
            .count(),
        0
    );
    fs::remove_dir_all(root).unwrap();
}

pub(super) fn copy_authority_inputs(live: &Path, root: &Path) {
    let lane_bytes = fs::read(live.join("LANE_REGISTRY.json")).unwrap();
    fs::copy(
        live.join("LANE_REGISTRY.json"),
        root.join("LANE_REGISTRY.json"),
    )
    .unwrap();
    let lane_registry: serde_json::Value = serde_json::from_slice(&lane_bytes).unwrap();
    for reference in lane_registry["root_freeze"]["payload_refs"]
        .as_array()
        .unwrap()
    {
        let relative = Path::new(reference["path"].as_str().unwrap());
        let destination = root.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(live.join(relative), destination).unwrap();
    }
    let source = live.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    let target = root.join("docs/ultragoal-contract-2026-07-successor-v2/FINAL-CONTRACT");
    fs::create_dir_all(&target).unwrap();
    let mut contract_files = fs::read_dir(&source)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    contract_files.sort();
    for path in contract_files {
        fs::copy(&path, target.join(path.file_name().unwrap())).unwrap();
    }
    for name in ["FINAL-HANDOFF-MANIFEST.sha256", "README.md"] {
        fs::copy(
            source.parent().unwrap().join(name),
            target.parent().unwrap().join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(root.join("migration")).unwrap();
    for name in ["authority-routes.json", "generated-surface-authority.json"] {
        fs::copy(
            live.join("migration").join(name),
            root.join("migration").join(name),
        )
        .unwrap();
    }
    super::generated_inputs::copy_generated_inputs(live, root);
    fs::create_dir_all(root.join(".codex-plugin")).unwrap();
    fs::write(
        root.join(".codex-plugin/plugin.json"),
        br#"{"name":"harness-ultragoal","version":"0.0.0-test"}"#,
    )
    .unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed: {output:?}");
}
