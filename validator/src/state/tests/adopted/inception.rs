use super::registry::copy_authority_inputs;
use crate::context::{inception_subject_identity, BuildRequest, EffectClass, LiveContext};
use crate::digest;
use crate::inventory::{
    InventoryBuilder, ADOPTED_HANDOFF_DIGEST_CONFIG_KEY, ADOPTED_HANDOFF_MANIFEST_SHA256,
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
    rebind_current_amendment_to_fixture_brief(&root);

    let context = context(&root);
    assert!(
        inception_subject_identity(&context.begin_read_session().unwrap())
            .unwrap()
            .dirty,
        "the fixture amendment rebind is identity-bearing while the brief keeps its historical start"
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

fn rebind_current_amendment_to_fixture_brief(root: &Path) {
    let amendment_path = root.join("AMENDMENTS.jsonl");
    let mut rows = fs::read_to_string(&amendment_path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .collect::<Vec<_>>();
    let amendment_hash = {
        let current = rows.last_mut().expect("fixture has current amendment");
        for binding in current["backlog_updates"].as_array_mut().unwrap() {
            if binding["path"] == "PRODUCT_SUCCESS_BRIEF.json" {
                binding["digest"] = serde_json::json!(digest::file(
                    &root.join("PRODUCT_SUCCESS_BRIEF.json")
                )
                .unwrap());
            }
        }
        current.as_object_mut().unwrap().remove("amendment_hash");
        let amendment_hash = digest::bytes(&serde_json::to_vec(current).unwrap());
        current.as_object_mut().unwrap().insert(
            "amendment_hash".to_owned(),
            serde_json::json!(amendment_hash),
        );
        amendment_hash
    };
    fs::write(
        amendment_path,
        format!(
            "{}\n",
            rows.iter()
                .map(serde_json::to_string)
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join("\n")
        ),
    )
    .unwrap();
    rebind_generated_authority(root, &amendment_hash);
}

fn rebind_generated_authority(root: &Path, amendment_hash: &str) {
    for relative in [
        "migration/generated-surface-authority.json",
        "migration/generated-surface-authority/product-success-contract.json",
    ] {
        let path = root.join(relative);
        let mut value =
            serde_json::from_slice::<serde_json::Value>(&fs::read(&path).unwrap()).unwrap();
        rebind_adopted_surface(&mut value, amendment_hash);
        fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    }
}

fn rebind_adopted_surface(value: &mut serde_json::Value, amendment_hash: &str) {
    if value["disposition"] == "adopted_schema_contract" {
        value["amendment_hash"] = serde_json::json!(amendment_hash.trim_start_matches("sha256:"));
    }
    if let Some(values) = value.as_object_mut() {
        for child in values.values_mut() {
            rebind_adopted_surface(child, amendment_hash);
        }
    } else if let Some(values) = value.as_array_mut() {
        for child in values {
            rebind_adopted_surface(child, amendment_hash);
        }
    }
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
            "dirty_state_expectation": "either",
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
