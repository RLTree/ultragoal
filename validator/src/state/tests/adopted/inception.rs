use super::registry::copy_authority_inputs;
use crate::context::{BuildRequest, EffectClass, LiveContext, inception_subject_identity};
use crate::digest;
use crate::inventory::{
    ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256, InventoryBuilder,
};
use crate::state::adopted::derive_adopted;
use std::fs;
use std::path::Path;
use std::process::Command;

const CONTRACT: &str = "examples/generated/PRODUCT_SUCCESS_CONTRACT.json";

#[test]
fn active_brief_selects_the_current_first_truth_loop_route() {
    let live = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let root =
        std::env::temp_dir().join(format!("ultragoal-active-inception-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("tracked.txt"), b"tracked\n").unwrap();
    git(&root, &["init", "-q"]);
    git(
        &root,
        &["config", "user.email", "inception@example.invalid"],
    );
    git(&root, &["config", "user.name", "Active Inception"]);
    copy_authority_inputs(live, &root);
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-qm", "fixture"]);
    let initial = context(&root);
    let subject = inception_subject_identity(&initial.begin_read_session().unwrap()).unwrap();
    assert!(!subject.dirty);
    let repository_identity = digest::bytes(initial.roots().repository_root.as_bytes());
    fs::write(
        root.join("PRODUCT_SUCCESS_BRIEF.json"),
        brief(
            &repository_identity,
            &subject.digest,
            &digest::file(&root.join(CONTRACT)).unwrap(),
        ),
    )
    .unwrap();

    let context = context(&root);
    assert_eq!(
        inception_subject_identity(&context.begin_read_session().unwrap())
            .unwrap()
            .digest,
        subject.digest
    );
    let inventory = InventoryBuilder::new(&context).build().unwrap();
    let inception = crate::product_inception::inspect(&context)
        .unwrap()
        .to_json()
        .unwrap();
    assert!(
        String::from_utf8(inception)
            .unwrap()
            .contains("\"status\":\"active\""),
        "the fixture brief must remain current before ranking"
    );
    let state = derive_adopted(&context, &inventory).unwrap();
    assert_eq!(
        state.next_action().action_id,
        "inspect-installed-daily-driver"
    );
    assert_eq!(
        state.next_action().command_id.as_deref(),
        Some("fit-inspect")
    );
    assert_eq!(
        state.next_action().active_transition.as_deref(),
        Some("inspect")
    );
    assert_eq!(
        state.next_action().exact_command.as_ref().unwrap(),
        &vec![
            "ultragoal".to_owned(),
            "--json".to_owned(),
            "fit".to_owned(),
            "inspect".to_owned(),
        ]
    );
    fs::remove_dir_all(root).unwrap();
}

fn context(root: &Path) -> LiveContext {
    LiveContext::build(
        BuildRequest::new(root)
            .with_effect(EffectClass::Read)
            .bind_non_secret_configuration(
                ADOPTED_HANDOFF_DIGEST_CONFIG_KEY,
                ADOPTED_HANDOFF_MANIFEST_SHA256,
            ),
    )
    .unwrap()
}

fn brief(repository_identity: &str, candidate: &str, contract_digest: &str) -> String {
    serde_json::json!({
        "schema": "harness-ultragoal.product-success-brief.v2",
        "product_success_contract_id": "PSC-HARNESS-ULTRAGOAL-SUCCESSOR-LIVE-001",
        "product_success_contract_digest": contract_digest,
        "claim_ids": ["CL-FIT"],
        "target_problem": "Validate the installed public route before product expansion.",
        "audience": "repository operator",
        "job_to_be_done": "inspect the current repository safely",
        "context_of_use": "a local source candidate",
        "desired_outcome": "an exact read-only repository fit observation",
        "first_value_event": "the operator receives the fit observation",
        "operator": { "kind": "agent", "actor_reference": "test-operator" },
        "real_work": {
            "repository_identity": repository_identity,
            "starting_candidate": candidate,
            "dirty_state_expectation": "clean",
            "task_id": "installed-daily-driver",
            "task": "inspect the active repository through the public route",
            "expected_useful_outcome": "a bounded fit observation"
        },
        "public_entry_surface": {
            "surface_id": "PS-ENTRY",
            "route": "harness-ultragoal",
            "forbidden_bypasses": ["direct internal invocation"]
        },
        "protected_invariants": [{
            "id": "INV-READ-ONLY",
            "claim_ids": ["CL-FIT"],
            "surfaces": ["PS-ENTRY"],
            "required_condition": "inspection does not write repository state",
            "disposition": "fail_closed"
        }],
        "first_truth_loop": {
            "loop_id": "installed-daily-driver",
            "positive_path": [{
                "transition_id": "inspect",
                "action_id": "inspect-installed-daily-driver",
                "command_id": "fit-inspect",
                "effect": "read",
                "order": 1,
                "dependency_ids": [],
                "capability_ids": [],
                "claim_ids": ["CL-FIT"],
                "product_surfaces": ["PS-ENTRY"],
                "expected_observation": "a read-only repository fit observation",
                "evidence_class": "source"
            }],
            "first_value_transition": "inspect",
            "failure_control": {
                "transition_id": "inspect",
                "failure": "an unsafe repository input is rejected",
                "diagnosis": "inspect inception and the fit finding",
                "recovery": "repair the input and rerun inspection",
                "preservation": "unrelated repository state remains unchanged"
            },
            "preservation_expectation": "unrelated repository state remains unchanged",
            "repeat_use_expectation": "the same route can be safely re-observed"
        },
        "depth_triggers": [],
        "evidence_class": "source",
        "evidence_ladder": "source evidence only",
        "claim_ceiling": "source_only"
    })
    .to_string()
}

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?} failed: {output:?}");
}
