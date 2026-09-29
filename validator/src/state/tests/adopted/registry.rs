use crate::context::{BuildRequest, EffectClass, LiveContext};
use crate::inventory::behavioral_role;
use crate::state::adopted_registry::{load_claims_for_test, validate_registry_for_test};
use crate::state::{NextActionKind, ProductGoalState, StateError, derive_current};
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
const HANDOFF_SHA256: &str = "89b0d7f17aca16c262500533677e54803643939a19c71ec2fe71fb395aeb97ea";

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
fn current_catalog_binds_one_current_claim_without_legacy_actions() {
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
    git(&root, &["init", "-q"]);
    git(&root, &["config", "user.email", "state@example.invalid"]);
    git(&root, &["config", "user.name", "Adopted State"]);
    copy_current_authority_inputs(&live, &root);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "fixture"]);
    let context =
        LiveContext::build(BuildRequest::new(&root).with_effect(EffectClass::Read)).unwrap();
    let state = derive_current(&context).unwrap();
    assert_eq!(state.product_goal(), ProductGoalState::Operating);
    assert_eq!(state.next_action().kind, NextActionKind::Command);
    assert_eq!(state.next_action().effect, EffectClass::Read);
    assert_eq!(state.next_action().action_id, "inspect-current-usable-loop");
    assert_eq!(
        state.next_action().command_id.as_deref(),
        Some("fit-inspect")
    );
    assert!(state.findings().iter().any(|finding| {
        finding.code == "dependency-missing"
            && finding.dependency_ids.contains("current-usable-loop")
    }));
    assert_eq!(state.claim_ceilings().len(), 1);
    let usable_loop = &state.claim_ceilings()[0];
    assert_eq!(usable_loop.claim_id(), "CL-USABLE-LOOP");
    assert!(usable_loop.dimensions().is_empty());
    assert_eq!(
        state
            .claim_ceilings()
            .iter()
            .filter(|ceiling| ceiling.claim_id() != "CL-USABLE-LOOP")
            .count(),
        0
    );
    fs::remove_dir_all(root).unwrap();
}

fn copy_current_authority_inputs(live: &Path, root: &Path) {
    for relative in [
        "GOAL_CONTRACT.md",
        "PRODUCT_SUCCESS_CONTRACT.md",
        "docs/exec-plans/active/usable-product-milestone.md",
        ".codex-plugin/plugin.json",
    ] {
        let destination = root.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(live.join(relative), destination).unwrap();
    }
    let agents = root.join(".codex/agents");
    fs::create_dir_all(&agents).unwrap();
    for entry in fs::read_dir(live.join(".codex/agents")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), agents.join(entry.file_name())).unwrap();
    }
}

pub(super) fn copy_authority_inputs(live: &Path, root: &Path) {
    for relative in [
        "GOAL_CONTRACT.md",
        "PRODUCT_SUCCESS_CONTRACT.md",
        "docs/exec-plans/active/usable-product-milestone.md",
    ] {
        let destination = root.join(relative);
        fs::create_dir_all(destination.parent().unwrap_or(root)).unwrap();
        fs::copy(live.join(relative), destination).unwrap();
    }
    let advisory_decision = Path::new(
        "docs/ultragoal-successor-live/root-decisions/AGENTIC-ENGINEERING-V3-LIFECYCLE-ADVISORY-004.json",
    );
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
        if !live.join(relative).is_file() {
            continue;
        }
        let destination = root.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(live.join(relative), destination).unwrap();
    }
    let advisory_destination = root.join(advisory_decision);
    fs::create_dir_all(advisory_destination.parent().unwrap()).unwrap();
    fs::copy(live.join(advisory_decision), advisory_destination).unwrap();
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
    fs::copy(
        live.join(".codex-plugin/plugin.json"),
        root.join(".codex-plugin/plugin.json"),
    )
    .unwrap();
    for binding in behavioral_role::bindings() {
        let source = live.join(binding.relative_path);
        let target = root.join(binding.relative_path);
        fs::create_dir_all(target.parent().unwrap_or(root)).unwrap();
        fs::copy(source, target).unwrap();
    }
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?} failed: {output:?}");
}
