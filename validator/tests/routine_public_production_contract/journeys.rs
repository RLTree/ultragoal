use super::scenario::{
    ContainedContender, Fixture, contain_contender, git, pass_node, prefix_route, routine_command,
    run_bounded_contender, set_mode, tree,
};
use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::fs::{OpenOptionsExt, symlink};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[test]
fn fixture_matrix_names_the_public_production_contract_without_claim_effect() {
    let value: Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/routine-public-production/cases.json"
    ))
    .unwrap();
    assert_eq!(value["schema_version"], "RoutinePublicProductionCases-v2");
    assert_eq!(value["supported_host"], "target_vendor=apple");
    assert_eq!(value["claim_effect"], "none");
    let cases = value["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    assert_eq!(
        cases
            .iter()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        7
    );
    let runtime_cases = value["immutable_runtime_cases"].as_array().unwrap();
    assert_eq!(runtime_cases.len(), 8);
    assert_eq!(
        runtime_cases
            .iter()
            .filter_map(Value::as_str)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        8
    );
    assert!(
        value["installed_runtime_ceiling"]
            .as_str()
            .unwrap()
            .contains("immutable installed ultragoal runtime")
    );
}
#[test]
fn clean_public_routine_is_a_zero_effect_noop() {
    let mut fixture = Fixture::new(
        "clean-no-op",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        false,
        false,
    );
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "clean-no-op");
    assert_eq!(value["effect"], "none");
    let after_root = tree(&fixture.root);
    for (path, value) in before_root {
        assert_eq!(
            after_root.get(&path),
            Some(&value),
            "source changed: {path}"
        );
    }
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    fixture.teardown_after_assertions();
}
#[test]
fn dirty_public_effect_executes_once_then_exact_repeat_reuses_without_mutation() {
    let mut fixture = Fixture::new(
        "execute-reuse",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let before_root = tree(&fixture.root);
    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    assert!(executed.stderr.is_empty(), "{executed:?}");
    let executed_value = Fixture::value(&executed);
    assert_eq!(executed_value["status"], "executed");
    assert_eq!(executed_value["nodes"][0]["disposition"], "executed");
    let after_root = tree(&fixture.root);
    for (path, value) in before_root {
        assert_eq!(
            after_root.get(&path),
            Some(&value),
            "source changed: {path}"
        );
    }

    let before_repeat = tree(&fixture.root);
    assert_reused_without_effect(&fixture.run());
    assert_eq!(tree(&fixture.root), before_repeat);
    fixture.teardown_after_assertions();
}

#[test]
fn executed_routine_publishes_one_owner_only_event_and_observe_is_zero_write() {
    let mut fixture = Fixture::new(
        "host-event-observe",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        false,
    );
    let evaluator_sentinel = fixture.home.join("evaluator-sentinel.txt");
    fs::write(&evaluator_sentinel, b"evaluator-owned\n").unwrap();
    let source_before = fs::read(fixture.root.join("src/lib.rs")).unwrap();
    let status_before = fixture.status();
    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    assert_eq!(Fixture::value(&executed)["status"], "executed");
    assert_eq!(fs::read(&evaluator_sentinel).unwrap(), b"evaluator-owned\n");
    assert_eq!(
        fs::read(fixture.root.join("src/lib.rs")).unwrap(),
        source_before
    );
    assert_eq!(fixture.status(), status_before);
    let _event_leaf = fixture.event_leaf();
    assert!(
        !fixture
            .root
            .join("validation_artifacts/observability/spool")
            .exists()
    );
    assert_eq!(
        fs::read_dir(fixture.state_root().join(".routine-authority-launch"))
            .unwrap()
            .count(),
        0,
        "successful launch cleanup leaves the shared root empty"
    );

    assert_reused_without_effect(&fixture.run());
    assert_eq!(fs::read(&evaluator_sentinel).unwrap(), b"evaluator-owned\n");
    assert_eq!(
        fs::read_dir(fixture.state_root().join(".routine-authority-launch"))
            .unwrap()
            .count(),
        0
    );

    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let observed = fixture.run_args(&[
        "--json",
        "observe",
        "query",
        "--filter",
        "check.routine.terminal",
    ]);
    assert_eq!(observed.status.code(), Some(0), "{observed:?}");
    assert!(observed.stderr.is_empty(), "{observed:?}");
    let observed = Fixture::value(&observed);
    assert_eq!(observed["schema_version"], "ObservabilityQuery-v1");
    assert_eq!(observed["store_status"], "available");
    assert_eq!(observed["event_count"], 1);
    assert_eq!(observed["events"][0]["operation"], "check.routine.terminal");
    assert_eq!(observed["claim_effect"], "none");
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    assert_eq!(fs::read(&evaluator_sentinel).unwrap(), b"evaluator-owned\n");
    fixture.teardown_after_assertions();
}

#[test]
fn joined_checkpoint_without_its_event_refuses_repeat_without_recreation() {
    let mut fixture = Fixture::new(
        "joined-event-deleted",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        false,
    );
    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    let event = fixture.event_leaf();
    fs::remove_file(&event).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();

    let refused = fixture.run();

    assert_ne!(refused.status.code(), Some(0), "{refused:?}");
    assert!(refused.stdout.is_empty(), "{refused:?}");
    let diagnostic: Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(
        diagnostic["diagnostic_id"],
        "successor_runtime_authority_required"
    );
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    assert!(
        !event.exists(),
        "joined event must not be recreated by a read/reuse path"
    );
    fixture.teardown_after_assertions();
}

#[test]
fn forged_duplicate_terminal_row_is_refused_without_read_side_effects() {
    let mut fixture = Fixture::new(
        "host-event-forged-row",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        false,
    );
    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    let leaf = fixture.event_leaf();
    let original = fs::read(&leaf).unwrap();
    let mut duplicated = original.clone();
    duplicated.extend_from_slice(&original);
    fs::write(&leaf, duplicated).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();

    let refused = fixture.run_args(&["--json", "observe", "query"]);

    assert_eq!(refused.status.code(), Some(4), "{refused:?}");
    assert!(refused.stdout.is_empty(), "{refused:?}");
    let value: Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(
        value["diagnostic_id"],
        "successor_runtime_observability_unavailable"
    );
    assert!(!String::from_utf8_lossy(&refused.stderr).contains(fixture.root.to_str().unwrap()));
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    fixture.teardown_after_assertions();
}

#[test]
fn terminal_failure_event_and_routine_diagnosis_share_one_authenticated_cause() {
    let mut fixture = Fixture::new(
        "host-event-failure-diagnosis",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        false,
        false,
    );
    fs::write(fixture.root.join("src/lib.rs"), b"pub fn broken(\n").unwrap();
    let failed = fixture.run();
    assert_eq!(failed.status.code(), Some(1), "{failed:?}");
    let failed_value = Fixture::value(&failed);
    assert_eq!(failed_value["status"], "incomplete");
    assert_eq!(failed_value["nodes"][0]["disposition"], "failed");
    let _event_leaf = fixture.event_leaf();

    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    let observed = fixture.run_args(&[
        "--json",
        "observe",
        "query",
        "--filter",
        "check.routine.terminal",
    ]);
    assert_eq!(observed.status.code(), Some(0), "{observed:?}");
    let observed = Fixture::value(&observed);
    assert_eq!(observed["event_count"], 1);
    assert_eq!(observed["events"][0]["outcome"], "fail");
    let diagnosis = fixture.run_args(&["--json", "diagnose"]);
    assert_eq!(diagnosis.status.code(), Some(1), "{diagnosis:?}");
    let diagnosis = Fixture::value(&diagnosis);
    assert_eq!(diagnosis["schema_version"], "RoutineDiagnosis-v1");
    assert_eq!(diagnosis["status"], "terminal_failure");
    assert_eq!(diagnosis["checkpoint"]["terminal_outcome"], "failed");
    assert_eq!(
        diagnosis["checkpoint"]["event_id"],
        observed["events"][0]["event_id"]
    );
    assert_eq!(diagnosis["claim_effect"], "none");
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    fixture.teardown_after_assertions();
}

#[test]
fn diagnosis_is_binding_scoped_while_mutation_remains_globally_strict() {
    let mut target = Fixture::new(
        "binding-scoped-diagnosis-target",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        false,
        false,
    );
    fs::write(target.root.join("src/lib.rs"), b"pub fn broken(\n").unwrap();
    let failed = target.run();
    assert_eq!(failed.status.code(), Some(1), "{failed:?}");
    let target_event = target.event_leaf();

    let mut sibling = Fixture::new(
        "binding-scoped-diagnosis-sibling",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut sibling_run = routine_command(&sibling.root, &target.home, sibling.binary_path());
    sibling_run.args(["--json", "check", "routine"]);
    let sibling_run = sibling_run.output().unwrap();
    assert_eq!(sibling_run.status.code(), Some(0), "{sibling_run:?}");
    let before_stale_sibling = target.run_args(&["--json", "diagnose"]);
    assert_eq!(
        before_stale_sibling.status.code(),
        Some(1),
        "{before_stale_sibling:?}"
    );

    let adapter = target.state_root().join("adapter");
    let sibling_event = fs::read_dir(&adapter)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("routine-events-") && name.ends_with(".jsonl"))
        })
        .find(|path| path != &target_event)
        .expect("shared host state did not retain the sibling event binding");
    let stale_sibling_event = adapter.join(format!("routine-events-{}.jsonl", "a".repeat(64)));
    fs::rename(sibling_event, &stale_sibling_event).unwrap();

    let before_root = tree(&target.root);
    let diagnosis = target.run_args(&["--json", "diagnose"]);
    assert_eq!(diagnosis.status.code(), Some(1), "{diagnosis:?}");
    let diagnosis = Fixture::value(&diagnosis);
    assert_eq!(diagnosis["schema_version"], "RoutineDiagnosis-v1");
    assert_eq!(diagnosis["status"], "terminal_failure");
    assert_eq!(diagnosis["checkpoint"]["terminal_outcome"], "failed");
    assert_eq!(diagnosis["claim_effect"], "none");
    assert_eq!(tree(&target.root), before_root);

    let mutation = target.run();
    assert_eq!(mutation.status.code(), Some(3), "{mutation:?}");
    assert_eq!(tree(&target.root), before_root);

    sibling.teardown_after_assertions();
    target.teardown_after_assertions();
}

#[test]
fn routine_host_state_rejects_unknown_format_before_effect() {
    let mut fixture = Fixture::new(
        "routine-state-format-fence",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::write(
        fixture.state_root().join("routine-state-format"),
        b"routine-host-state-v7\n",
    )
    .unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);

    let output = fixture.run();

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    fixture.teardown_after_assertions();
}

#[test]
fn format_absent_canonical_state_is_a_zero_write_migration_candidate() {
    let mut fixture = Fixture::new(
        "legacy-migration-candidate",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);

    let output = fixture.run_args(&["--json", "diagnose"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["schema_version"], "RoutineStateMigrationAdmission-v7");
    assert_eq!(value["status"], "migration_candidate_requires_approval");
    assert_eq!(value["format_status"], "absent_legacy");
    assert_eq!(value["legacy_singleton_count"], 0);
    assert_eq!(value["canonical_continuation_count"], 0);
    assert_eq!(value["event_journal_count"], 0);
    assert_eq!(value["history_relation"], "single_history_only");
    assert_eq!(value["migration_effect"], "none");
    assert_eq!(value["migration_authorized"], false);
    let plan = &value["quarantine_plan"];
    assert_eq!(plan["schema_version"], "RoutineStateQuarantinePlan-v5");
    assert_eq!(plan["history_relation"], "single_history_only");
    assert_eq!(
        plan["strategy"],
        "stage_fresh_v8_then_atomic_exchange_finalize_legacy_and_publish_settlement_receipt"
    );
    assert_eq!(plan["apply_capability"], "available_exact_record_only");
    let next_action = value["next_action"].as_str().unwrap();
    for required in [
        "ultragoal --json diagnose",
        "relative in-repository path",
        "ultragoal --json migrate apply",
        "quarantine_plan.plan_id",
        "migrate plan emits a separate ProductMigration projection",
    ] {
        assert!(
            next_action.contains(required),
            "missing {required}: {next_action}"
        );
    }
    assert!(
        plan["source_owner_tree_sha256"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:"))
    );
    assert!(
        plan["target_path_sha256"]
            .as_str()
            .is_some_and(|value| value.starts_with("sha256:"))
    );
    assert_eq!(value["claim_effect"], "none");
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    fixture.teardown_after_assertions();
}

#[test]
fn format_absent_diagnosis_ignores_unrelated_parent_siblings_without_plan_drift() {
    let mut fixture = Fixture::new(
        "legacy-parent-sibling-noise",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let before_baseline_root = tree(&fixture.root);
    let before_baseline_home = tree(&fixture.home);

    let baseline = fixture.run_args(&["--json", "diagnose"]);

    assert_eq!(baseline.status.code(), Some(1), "{baseline:?}");
    assert!(baseline.stderr.is_empty(), "{baseline:?}");
    let baseline_value = Fixture::value(&baseline);
    assert_eq!(
        baseline_value["schema_version"],
        "RoutineStateMigrationAdmission-v7"
    );
    assert_eq!(
        baseline_value["quarantine_plan"]["schema_version"],
        "RoutineStateQuarantinePlan-v5"
    );
    assert_eq!(tree(&fixture.root), before_baseline_root);
    assert_eq!(tree(&fixture.home), before_baseline_home);

    let owner_parent = fixture.state_root().parent().unwrap().to_path_buf();
    let unrelated_file = owner_parent.join("unrelated-metadata.bin");
    let unrelated_directory = owner_parent.join("unrelated-cache");
    fs::write(&unrelated_file, b"unrelated evaluator-owned bytes\n").unwrap();
    set_mode(&unrelated_file, 0o644);
    fs::create_dir(&unrelated_directory).unwrap();
    set_mode(&unrelated_directory, 0o755);
    let before_noise_root = tree(&fixture.root);
    let before_noise_home = tree(&fixture.home);

    let with_noise = fixture.run_args(&["--json", "diagnose"]);

    assert_eq!(with_noise.status.code(), Some(1), "{with_noise:?}");
    assert!(with_noise.stderr.is_empty(), "{with_noise:?}");
    assert_eq!(with_noise.stdout, baseline.stdout);
    let with_noise_value = Fixture::value(&with_noise);
    assert_eq!(with_noise_value, baseline_value);
    assert_eq!(
        with_noise_value["quarantine_plan"]["plan_id"],
        baseline_value["quarantine_plan"]["plan_id"]
    );
    assert_eq!(
        fs::read(&unrelated_file).unwrap(),
        b"unrelated evaluator-owned bytes\n"
    );
    assert_eq!(tree(&fixture.root), before_noise_root);
    assert_eq!(tree(&fixture.home), before_noise_home);
    fixture.teardown_after_assertions();
}

fn write_exact_quarantine_record(fixture: &Fixture) -> (String, String, &'static str) {
    let diagnosis = fixture.run_args(&["--json", "diagnose"]);
    assert_eq!(diagnosis.status.code(), Some(1), "{diagnosis:?}");
    let value = Fixture::value(&diagnosis);
    assert_eq!(value["schema_version"], "RoutineStateMigrationAdmission-v7");
    let plan = &value["quarantine_plan"];
    let plan_id = plan["plan_id"].as_str().unwrap().to_owned();
    let quarantine_owner = plan["quarantine_owner"].as_str().unwrap().to_owned();
    let relative = "migration/routine-state-quarantine.json";
    let path = fixture.root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, &diagnosis.stdout).unwrap();
    set_mode(&path, 0o600);
    (plan_id, quarantine_owner, relative)
}

fn create_fresh_transition_owner(path: &Path, plan: &Value) {
    #[derive(serde::Serialize)]
    struct CanonicalPlan<'a> {
        schema_version: &'a Value,
        plan_id: &'a Value,
        source_inventory_sha256: &'a Value,
        source_owner_tree_sha256: &'a Value,
        target_path_sha256: &'a Value,
        history_relation: &'a Value,
        authoritative_history: &'a Value,
        source_owner: &'a Value,
        quarantine_owner: &'a Value,
        target_format: &'a Value,
        strategy: &'a Value,
        apply_capability: &'a Value,
        operations: &'a Value,
        rollback: &'a Value,
    }

    fs::create_dir(path).unwrap();
    set_mode(path, 0o700);
    for directory in ["authority", "adapter", ".routine-authority-launch"] {
        let directory = path.join(directory);
        fs::create_dir(&directory).unwrap();
        set_mode(&directory, 0o700);
    }
    let format = path.join("routine-state-format");
    fs::write(&format, b"routine-host-state-v8\n").unwrap();
    set_mode(&format, 0o600);
    let lock = path.join("adapter/adapter.lock");
    fs::write(&lock, b"routine-public-lock-v1\n").unwrap();
    set_mode(&lock, 0o600);
    let marker = path.join(".routine-state-quarantine-pending.json");
    let canonical = CanonicalPlan {
        schema_version: &plan["schema_version"],
        plan_id: &plan["plan_id"],
        source_inventory_sha256: &plan["source_inventory_sha256"],
        source_owner_tree_sha256: &plan["source_owner_tree_sha256"],
        target_path_sha256: &plan["target_path_sha256"],
        history_relation: &plan["history_relation"],
        authoritative_history: &plan["authoritative_history"],
        source_owner: &plan["source_owner"],
        quarantine_owner: &plan["quarantine_owner"],
        target_format: &plan["target_format"],
        strategy: &plan["strategy"],
        apply_capability: &plan["apply_capability"],
        operations: &plan["operations"],
        rollback: &plan["rollback"],
    };
    fs::write(&marker, serde_json::to_vec(&canonical).unwrap()).unwrap();
    set_mode(&marker, 0o600);
}

#[test]
fn exact_diagnosis_record_quarantines_legacy_owner_bootstraps_v8_and_unblocks_next() {
    let mut fixture = Fixture::new(
        "legacy-quarantine-apply",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let legacy_tree = tree(&fixture.state_root());

    let (plan_id, _, plan_relative) = write_exact_quarantine_record(&fixture);

    let before_refusal = tree(&fixture.home);
    let wrong = format!("routine-quarantine-sha256:{}", "f".repeat(64));
    let refused = fixture.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        plan_relative,
        "--accept-plan",
        &wrong,
    ]);
    assert_eq!(refused.status.code(), Some(3), "{refused:?}");
    assert!(refused.stdout.is_empty(), "{refused:?}");
    assert_eq!(tree(&fixture.home), before_refusal);

    let applied = fixture.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        plan_relative,
        "--accept-plan",
        &plan_id,
    ]);
    assert_eq!(applied.status.code(), Some(0), "{applied:?}");
    let applied_value = Fixture::value(&applied);
    assert_eq!(
        applied_value["schema_version"],
        "RoutineStateQuarantineApplyOutcome-v3"
    );
    assert_eq!(applied_value["status"], "applied");
    assert_eq!(
        applied_value["effect"],
        "staged_fresh_v8_then_atomically_exchanged_quarantined_legacy_and_published_settlement_receipt"
    );
    assert_eq!(applied_value["settlement_state"], "settled_receipt_durable");
    assert_eq!(applied_value["plan_id"], plan_id);
    assert_eq!(applied_value["fresh_format_verified"], true);
    assert_eq!(applied_value["legacy_authority_imported"], false);
    let quarantine_owner = applied_value["quarantine_owner"].as_str().unwrap();
    let quarantine = fixture
        .state_root()
        .parent()
        .unwrap()
        .join(quarantine_owner);
    assert_eq!(tree(&quarantine), legacy_tree);
    assert_eq!(
        fs::read(fixture.state_root().join("routine-state-format")).unwrap(),
        b"routine-host-state-v8\n"
    );
    assert!(fixture.state_root().join("authority").is_dir());
    assert!(
        fixture
            .state_root()
            .join(".routine-authority-launch")
            .is_dir()
    );
    assert_eq!(
        fs::read_dir(fixture.state_root().join("authority"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        fs::read_dir(fixture.state_root().join(".routine-authority-launch"))
            .unwrap()
            .count(),
        0
    );
    let pending_receipt = fs::read(
        fixture
            .state_root()
            .join(".routine-state-quarantine-pending.json"),
    )
    .unwrap();
    let settled_receipt = fs::read(
        fixture
            .state_root()
            .join(".routine-state-quarantine-settled.json"),
    )
    .unwrap();
    assert_eq!(pending_receipt, settled_receipt);
    assert!(
        !fixture
            .state_root()
            .join(".routine-state-quarantine-settled.next")
            .exists()
    );

    let after_apply = tree(&fixture.home);
    let record_path = fixture.root.join(plan_relative);
    let original_record = fs::read(&record_path).unwrap();
    let accepted_record: Value = serde_json::from_slice(&original_record).unwrap();
    let receipt_plan: Value = serde_json::from_slice(&pending_receipt).unwrap();
    assert_eq!(receipt_plan, accepted_record["quarantine_plan"]);
    for mutation in ["outer", "nested"] {
        let mut tampered: Value = serde_json::from_slice(&original_record).unwrap();
        match mutation {
            "outer" => tampered["next_action"] = Value::String("substituted handoff".to_owned()),
            "nested" => {
                tampered["quarantine_plan"]["rollback"] =
                    Value::String("substituted rollback".to_owned())
            }
            _ => unreachable!(),
        }
        fs::write(&record_path, serde_json::to_vec(&tampered).unwrap()).unwrap();
        let refused = fixture.run_args(&[
            "--json",
            "migrate",
            "apply",
            "--plan",
            plan_relative,
            "--accept-plan",
            &plan_id,
        ]);
        assert_ne!(refused.status.code(), Some(0), "{mutation}: {refused:?}");
        assert_eq!(tree(&fixture.home), after_apply, "{mutation}");
    }
    fs::write(&record_path, &original_record).unwrap();
    let replay = fixture.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        plan_relative,
        "--accept-plan",
        &plan_id,
    ]);
    assert_eq!(replay.status.code(), Some(0), "{replay:?}");
    let replay_value = Fixture::value(&replay);
    assert_eq!(replay_value["status"], "already_applied_no_effect");
    assert_eq!(replay_value["effect"], "none");
    assert_eq!(tree(&fixture.home), after_apply);

    let next = fixture.run_args(&["--json", "next"]);
    assert_eq!(next.status.code(), Some(1), "{next:?}");
    let next = Fixture::value(&next);
    assert_eq!(next["schema_version"], "RoutineNext-v1");
    assert_eq!(next["status"], "no_record");
    assert_eq!(next["next_action"]["action"], "run_current_routine");

    fixture.teardown_after_assertions();
}

#[test]
fn exact_quarantine_record_recovers_a_committed_exchange_before_finalize() {
    let mut fixture = Fixture::new(
        "legacy-quarantine-committed-recovery",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let legacy_tree = tree(&fixture.state_root());
    let (plan_id, quarantine_owner, plan_relative) = write_exact_quarantine_record(&fixture);
    let record: Value =
        serde_json::from_slice(&fs::read(fixture.root.join(plan_relative)).unwrap()).unwrap();
    let state_root = fixture.state_root();
    let parent = state_root.parent().unwrap();
    let stage = parent.join(".routine-public-bootstrap");
    fs::rename(&state_root, &stage).unwrap();
    create_fresh_transition_owner(&state_root, &record["quarantine_plan"]);
    let record_path = fixture.root.join(plan_relative);
    let original_record = fs::read(&record_path).unwrap();
    let mut tampered: Value = serde_json::from_slice(&original_record).unwrap();
    tampered["support_limit"] = Value::String("substituted support limit".to_owned());
    fs::write(&record_path, serde_json::to_vec(&tampered).unwrap()).unwrap();
    let before_tamper = tree(&fixture.home);
    let refused = fixture.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        plan_relative,
        "--accept-plan",
        &plan_id,
    ]);
    assert_ne!(refused.status.code(), Some(0), "{refused:?}");
    assert_eq!(tree(&fixture.home), before_tamper);
    fs::write(&record_path, &original_record).unwrap();
    let before_unaccepted = tree(&fixture.home);
    let unaccepted = fixture.run();
    assert_eq!(unaccepted.status.code(), Some(1), "{unaccepted:?}");
    let unaccepted_diagnostic: Value = serde_json::from_slice(&unaccepted.stderr).unwrap();
    assert_eq!(
        unaccepted_diagnostic["diagnostic_id"],
        "successor_runtime_state_unavailable"
    );
    assert_eq!(
        unaccepted_diagnostic["effect"],
        "host_state_namespace_or_durability_transition_preserved"
    );
    assert_eq!(tree(&fixture.home), before_unaccepted);

    let recovered = fixture.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        plan_relative,
        "--accept-plan",
        &plan_id,
    ]);
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    let recovered = Fixture::value(&recovered);
    assert_eq!(recovered["status"], "recovered_then_applied");
    assert_eq!(
        recovered["effect"],
        "staged_fresh_v8_then_atomically_exchanged_quarantined_legacy_and_published_settlement_receipt"
    );
    assert_eq!(recovered["settlement_state"], "settled_receipt_durable");
    let quarantine = parent.join(&quarantine_owner);
    assert_eq!(tree(&quarantine), legacy_tree);
    assert_eq!(
        fs::read(fixture.state_root().join("routine-state-format")).unwrap(),
        b"routine-host-state-v8\n"
    );

    fixture.teardown_after_assertions();
}

#[test]
fn exact_quarantine_record_resumes_a_staged_settlement_receipt() {
    let mut fixture = Fixture::new(
        "legacy-quarantine-staged-settlement",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let legacy_tree = tree(&fixture.state_root());
    let (plan_id, quarantine_owner, plan_relative) = write_exact_quarantine_record(&fixture);
    let record: Value =
        serde_json::from_slice(&fs::read(fixture.root.join(plan_relative)).unwrap()).unwrap();
    let state_root = fixture.state_root();
    let parent = state_root.parent().unwrap();
    let quarantine = parent.join(&quarantine_owner);
    fs::rename(&state_root, &quarantine).unwrap();
    create_fresh_transition_owner(&state_root, &record["quarantine_plan"]);
    fs::copy(
        state_root.join(".routine-state-quarantine-pending.json"),
        state_root.join(".routine-state-quarantine-settled.next"),
    )
    .unwrap();
    set_mode(
        &state_root.join(".routine-state-quarantine-settled.next"),
        0o600,
    );

    let before_unaccepted = tree(&fixture.home);
    let unaccepted = fixture.run_args(&["--json", "next"]);
    assert_eq!(unaccepted.status.code(), Some(1), "{unaccepted:?}");
    let diagnostic: Value = serde_json::from_slice(&unaccepted.stderr).unwrap();
    assert_eq!(
        diagnostic["diagnostic_id"],
        "successor_runtime_state_unavailable"
    );
    assert_eq!(tree(&fixture.home), before_unaccepted);

    let recovered = fixture.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        plan_relative,
        "--accept-plan",
        &plan_id,
    ]);
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    let recovered = Fixture::value(&recovered);
    assert_eq!(recovered["status"], "recovered_then_applied");
    assert_eq!(recovered["settlement_state"], "settled_receipt_durable");
    assert_eq!(tree(&quarantine), legacy_tree);
    assert!(
        state_root
            .join(".routine-state-quarantine-pending.json")
            .is_file()
    );
    assert!(
        state_root
            .join(".routine-state-quarantine-settled.json")
            .is_file()
    );
    assert!(
        !state_root
            .join(".routine-state-quarantine-settled.next")
            .exists()
    );

    let next = fixture.run_args(&["--json", "next"]);
    assert_eq!(next.status.code(), Some(1), "{next:?}");
    let next = Fixture::value(&next);
    assert_eq!(next["next_action"]["action"], "run_current_routine");

    fixture.teardown_after_assertions();
}

#[test]
fn unproven_partial_stage_is_preserved_without_guessing_recovery() {
    let mut fixture = Fixture::new(
        "legacy-quarantine-unproven-partial-stage",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let (plan_id, _, plan_relative) = write_exact_quarantine_record(&fixture);
    let state_root = fixture.state_root();
    let parent = state_root.parent().unwrap();
    let stage = parent.join(".routine-public-bootstrap");
    fs::create_dir(&stage).unwrap();
    set_mode(&stage, 0o700);
    let before = tree(&fixture.home);

    let held = fixture.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        plan_relative,
        "--accept-plan",
        &plan_id,
    ]);
    assert!(matches!(held.status.code(), Some(1 | 3)), "{held:?}");
    assert_eq!(tree(&fixture.home), before);

    fixture.teardown_after_assertions();
}

#[test]
fn quarantine_apply_rejects_collision_source_drift_and_cross_target_before_effect() {
    let mut collision = Fixture::new(
        "legacy-quarantine-collision",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(collision.state_root().join("routine-state-format")).unwrap();
    let (collision_plan, quarantine_owner, relative) = write_exact_quarantine_record(&collision);
    let record_path = collision.root.join(relative);
    let original_record = fs::read(&record_path).unwrap();
    let relabelled = String::from_utf8(original_record.clone())
        .unwrap()
        .replace("independently review", "independently relabel");
    assert_ne!(relabelled.as_bytes(), original_record);
    fs::write(&record_path, relabelled).unwrap();
    let before_record_tamper = tree(&collision.home);
    let refused = collision.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        relative,
        "--accept-plan",
        &collision_plan,
    ]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert_eq!(tree(&collision.home), before_record_tamper);
    fs::write(&record_path, original_record).unwrap();
    let destination = collision
        .state_root()
        .parent()
        .unwrap()
        .join(quarantine_owner);
    fs::create_dir(&destination).unwrap();
    set_mode(&destination, 0o700);
    let before_collision = tree(&collision.home);
    let refused = collision.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        relative,
        "--accept-plan",
        &collision_plan,
    ]);
    assert_eq!(refused.status.code(), Some(3), "{refused:?}");
    assert_eq!(tree(&collision.home), before_collision);
    collision.teardown_after_assertions();

    let mut drift = Fixture::new(
        "legacy-quarantine-source-drift",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(drift.state_root().join("routine-state-format")).unwrap();
    let (drift_plan, _, relative) = write_exact_quarantine_record(&drift);
    let drift_leaf = drift.state_root().join("adapter/unbound-state");
    fs::write(&drift_leaf, b"substituted\n").unwrap();
    set_mode(&drift_leaf, 0o600);
    let before_drift = tree(&drift.home);
    let refused = drift.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        relative,
        "--accept-plan",
        &drift_plan,
    ]);
    assert!(matches!(refused.status.code(), Some(1 | 3)), "{refused:?}");
    assert_eq!(tree(&drift.home), before_drift);
    drift.teardown_after_assertions();

    let mut source = Fixture::new(
        "legacy-quarantine-source-target",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(source.state_root().join("routine-state-format")).unwrap();
    let (foreign_plan, _, relative) = write_exact_quarantine_record(&source);
    let foreign_bytes = fs::read(source.root.join(relative)).unwrap();
    let mut target = Fixture::new(
        "legacy-quarantine-other-target",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(target.state_root().join("routine-state-format")).unwrap();
    let foreign_path = target.root.join(relative);
    fs::create_dir_all(foreign_path.parent().unwrap()).unwrap();
    fs::write(&foreign_path, foreign_bytes).unwrap();
    set_mode(&foreign_path, 0o600);
    let before_target = tree(&target.home);
    let refused = target.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        relative,
        "--accept-plan",
        &foreign_plan,
    ]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert_eq!(tree(&target.home), before_target);

    target.teardown_after_assertions();
    source.teardown_after_assertions();
}

#[test]
fn malformed_mixed_singleton_and_canonical_history_is_unsafe_and_preserved() {
    let mut fixture = Fixture::new(
        "legacy-mixed-history",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    fs::create_dir(fixture.continuations_root()).unwrap();
    fs::set_permissions(
        fixture.continuations_root(),
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )
    .unwrap();
    let canonical = fixture
        .continuations_root()
        .join(format!("routine-continuation-{}.json", "a".repeat(64)));
    fs::write(&canonical, b"{}\n").unwrap();
    fs::set_permissions(
        &canonical,
        std::os::unix::fs::PermissionsExt::from_mode(0o600),
    )
    .unwrap();
    let singleton = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::write(&singleton, b"{}\n").unwrap();
    fs::set_permissions(
        singleton,
        std::os::unix::fs::PermissionsExt::from_mode(0o600),
    )
    .unwrap();
    let before = tree(&fixture.home);

    let output = fixture.run_args(&["--json", "diagnose"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "unsafe_layout_preserve_and_hold");
    assert_eq!(value["history_relation"], "unsafe_or_unauthenticated");
    assert_eq!(value["migration_authorized"], false);
    assert!(value["quarantine_plan"].is_null());
    assert_eq!(tree(&fixture.home), before);
    fixture.teardown_after_assertions();
}

#[test]
fn authenticated_equivalent_mixed_history_is_classified_and_preserved() {
    let mut fixture = Fixture::new(
        "legacy-equivalent-mixed-history",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let completed = fixture.run();
    assert!(
        matches!(completed.status.code(), Some(0 | 1)),
        "{completed:?}"
    );
    let canonical = fixture.checkpoint_path();
    let singleton = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&canonical, &singleton).unwrap();
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);

    let output = fixture.run_args(&["--json", "diagnose"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(
        value["status"],
        "history_relation_established_preserve_and_hold"
    );
    assert_eq!(value["history_relation"], "redundant_equivalent");
    assert_eq!(value["legacy_singleton_count"], 1);
    assert_eq!(value["canonical_continuation_count"], 1);
    assert_eq!(value["event_journal_count"], 1);
    assert_eq!(value["migration_effect"], "none");
    assert_eq!(value["migration_authorized"], false);
    let plan = &value["quarantine_plan"];
    assert_eq!(plan["schema_version"], "RoutineStateQuarantinePlan-v5");
    assert_eq!(plan["history_relation"], "redundant_equivalent");
    assert_eq!(
        plan["authoritative_history"],
        "canonical_authenticated_history"
    );
    assert_eq!(plan["source_owner"], "routine-public");
    assert!(
        plan["quarantine_owner"]
            .as_str()
            .is_some_and(|value| value.starts_with("routine-public.quarantine-"))
    );
    assert_eq!(plan["operations"].as_array().map(Vec::len), Some(10));
    assert_eq!(plan["apply_capability"], "available_exact_record_only");
    let repeated = fixture.run_args(&["--json", "diagnose"]);
    assert_eq!(repeated.status.code(), Some(1), "{repeated:?}");
    let repeated = Fixture::value(&repeated);
    assert_eq!(repeated["quarantine_plan"]["plan_id"], plan["plan_id"]);
    assert_eq!(
        repeated["quarantine_plan"]["source_inventory_sha256"],
        plan["source_inventory_sha256"]
    );
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    fixture.teardown_after_assertions();
}

#[test]
fn authenticated_stale_reconciled_mixed_history_is_classified_and_preserved() {
    let mut fixture = Fixture::new(
        "legacy-stale-reconciled-mixed-history",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let interrupted = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    let checkpoint_path = fixture.checkpoint_path();
    let rolled_back_head = roll_back_test_reservation(&fixture);
    rewrite_checkpoint_as_reconciled(&checkpoint_path, &rolled_back_head);

    let mut independent = Fixture::new(
        "legacy-stale-reconciled-ledger-advance",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut advance = routine_command(&independent.root, &fixture.home, independent.binary_path());
    advance.args(["--json", "check", "routine"]);
    let advance = advance.output().unwrap();
    assert!(matches!(advance.status.code(), Some(0 | 1)), "{advance:?}");
    assert!(matches!(
        Fixture::value(&advance)["status"].as_str(),
        Some("executed" | "incomplete")
    ));
    assert_ne!(test_authority_head(&fixture), rolled_back_head);

    let singleton = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&checkpoint_path, &singleton).unwrap();
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);

    let output = fixture.run_args(&["--json", "diagnose"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "conflicting_histories_preserve_and_hold");
    assert_eq!(value["history_relation"], "conflicting_histories_hold");
    assert_eq!(value["legacy_singleton_count"], 1);
    assert_eq!(value["canonical_continuation_count"], 2);
    assert_eq!(value["event_journal_count"], 1);
    assert_eq!(value["migration_effect"], "none");
    assert_eq!(value["migration_authorized"], false);
    assert!(value["quarantine_plan"].is_null());
    assert!(
        !value["next_action"]
            .as_str()
            .unwrap()
            .contains("migrate apply")
    );
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);

    independent.teardown_after_assertions();
    fixture.teardown_after_assertions();
}

#[test]
fn caller_relabelled_reconciled_history_without_a_rolled_back_attempt_is_unsafe() {
    let mut fixture = Fixture::new(
        "legacy-reconciled-state-relabel",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let interrupted = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    let checkpoint_path = fixture.checkpoint_path();
    let checkpoint: Value = serde_json::from_slice(&fs::read(&checkpoint_path).unwrap()).unwrap();
    let reserved_head = checkpoint["authenticated_ledger_head"]
        .as_str()
        .unwrap()
        .to_owned();
    rewrite_checkpoint_as_reconciled(&checkpoint_path, &reserved_head);

    let mut independent = Fixture::new(
        "legacy-reconciled-state-relabel-ledger-advance",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut advance = routine_command(&independent.root, &fixture.home, independent.binary_path());
    advance.args(["--json", "check", "routine"]);
    let advance = advance.output().unwrap();
    assert!(matches!(advance.status.code(), Some(0 | 1)), "{advance:?}");
    assert_ne!(test_authority_head(&fixture), reserved_head);

    let singleton = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&checkpoint_path, singleton).unwrap();
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);

    let output = fixture.run_args(&["--json", "diagnose"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "unsafe_layout_preserve_and_hold");
    assert_eq!(value["history_relation"], "unsafe_or_unauthenticated");
    assert_eq!(value["migration_authorized"], false);
    assert!(value["quarantine_plan"].is_null());
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);

    independent.teardown_after_assertions();
    fixture.teardown_after_assertions();
}

fn roll_back_test_reservation(fixture: &Fixture) -> String {
    let state = fixture.authority_root().join("routine-authority.state");
    let key = fs::read(fixture.authority_root().join("routine-authority.key")).unwrap();
    let original = String::from_utf8(fs::read(&state).unwrap()).unwrap();
    let original_envelope: Value = serde_json::from_str(&original).unwrap();
    let generation = original_envelope["payload"]["generation"].as_u64().unwrap();
    let last_tick = original_envelope["payload"]["last_tick"].as_u64().unwrap();
    let previous_head = original_envelope["payload"]["previous_head_sha256"]
        .as_str()
        .unwrap();
    let original_head = format!("sha256:{:x}", Sha256::digest(original.as_bytes()));
    assert_eq!(original.matches("\"state\":\"reserved\"").count(), 1);
    assert_eq!(
        original
            .matches(&format!("\"generation\":{generation}"))
            .count(),
        1
    );
    assert_eq!(
        original
            .matches(&format!("\"last_tick\":{last_tick}"))
            .count(),
        1
    );
    assert_eq!(original.matches(previous_head).count(), 1);
    let mut rewritten = original
        .replacen(
            &format!("\"generation\":{generation}"),
            &format!("\"generation\":{}", generation + 1),
            1,
        )
        .replacen(previous_head, &original_head, 1)
        .replacen(
            &format!("\"last_tick\":{last_tick}"),
            &format!("\"last_tick\":{}", last_tick + 1),
            1,
        )
        .replacen("\"state\":\"reserved\"", "\"state\":\"rolled_back\"", 1);
    let payload_prefix = "{\"payload\":";
    let hmac_marker = ",\"hmac_sha256\":\"";
    assert!(rewritten.starts_with(payload_prefix));
    let hmac_marker_offset = rewritten.rfind(hmac_marker).unwrap();
    let payload = &rewritten.as_bytes()[payload_prefix.len()..hmac_marker_offset];
    let mut mac = Hmac::<Sha256>::new_from_slice(&key).unwrap();
    mac.update(payload);
    let next_hmac = format!("sha256:{:x}", mac.finalize().into_bytes());
    let envelope: Value = serde_json::from_str(&rewritten).unwrap();
    let prior_hmac = envelope["hmac_sha256"].as_str().unwrap();
    rewritten = rewritten.replacen(prior_hmac, &next_hmac, 1);
    fs::write(&state, rewritten.as_bytes()).unwrap();
    format!("sha256:{:x}", Sha256::digest(rewritten.as_bytes()))
}

fn test_authority_head(fixture: &Fixture) -> String {
    let state = fs::read(fixture.authority_root().join("routine-authority.state")).unwrap();
    format!("sha256:{:x}", Sha256::digest(state))
}

fn rewrite_checkpoint_as_reconciled(path: &std::path::Path, authenticated_head: &str) {
    let mut checkpoint: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let generation = checkpoint["generation"].as_u64().unwrap() + 1;
    let continuation = checkpoint["continuation"].as_str().unwrap();
    let mut event_id = Sha256::new();
    event_id.update(b"routine-terminal-event-v1\0");
    event_id.update(continuation.as_bytes());
    event_id.update(b"\0");
    event_id.update(authenticated_head.as_bytes());
    checkpoint["generation"] = Value::from(generation);
    checkpoint["authenticated_ledger_head"] = Value::from(authenticated_head);
    checkpoint["state"] = Value::from("reconciled");
    checkpoint["terminal_outcome"] = Value::Null;
    checkpoint["event_id"] = Value::from(format!("routine-terminal-{:x}", event_id.finalize()));
    checkpoint["event_sequence"] = Value::from(generation);
    checkpoint["event_status"] = Value::from("blocked");
    checkpoint["event_transition"] = Value::from("interrupted");
    fs::write(path, serde_json::to_vec(&checkpoint).unwrap()).unwrap();
}

#[test]
fn mixed_history_with_a_missing_required_event_is_causal_and_preserved() {
    let mut fixture = Fixture::new(
        "legacy-mixed-history-missing-event",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let completed = fixture.run();
    assert!(
        matches!(completed.status.code(), Some(0 | 1)),
        "{completed:?}"
    );
    let canonical = fixture.checkpoint_path();
    let singleton = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&canonical, &singleton).unwrap();
    fs::remove_file(fixture.event_leaf()).unwrap();
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();

    let diagnosis = fixture.run_args(&["--json", "diagnose"]);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    let next = fixture.run_args(&["--json", "next"]);

    assert_eq!(diagnosis.status.code(), Some(1), "{diagnosis:?}");
    assert_eq!(next.status.code(), Some(1), "{next:?}");
    assert!(diagnosis.stderr.is_empty(), "{diagnosis:?}");
    assert!(next.stderr.is_empty(), "{next:?}");
    assert_eq!(diagnosis.stdout, next.stdout);
    let diagnosis_value = Fixture::value(&diagnosis);
    let next_value = Fixture::value(&next);
    assert_eq!(diagnosis_value, next_value);
    assert_eq!(
        diagnosis_value["schema_version"],
        "RoutineStateMigrationAdmission-v7"
    );
    assert_eq!(
        diagnosis_value["status"],
        "terminal_event_history_incomplete_preserve_and_hold"
    );
    assert_eq!(
        diagnosis_value["history_relation"],
        "terminal_event_history_incomplete"
    );
    assert_eq!(diagnosis_value["legacy_singleton_count"], 1);
    assert_eq!(diagnosis_value["canonical_continuation_count"], 1);
    assert_eq!(diagnosis_value["event_journal_count"], 0);
    assert_eq!(diagnosis_value["migration_effect"], "none");
    assert_eq!(diagnosis_value["migration_authorized"], false);
    assert_eq!(diagnosis_value["claim_effect"], "none");
    assert!(diagnosis_value["quarantine_plan"].is_null());
    assert!(
        !diagnosis_value["next_action"]
            .as_str()
            .unwrap()
            .contains("migrate apply")
    );
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    fixture.teardown_after_assertions();
}

#[test]
fn duplicate_terminal_event_row_is_causal_conflict_and_preserved() {
    let mut fixture = Fixture::new(
        "legacy-duplicate-terminal-event-row",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let completed = fixture.run();
    assert!(
        matches!(completed.status.code(), Some(0 | 1)),
        "{completed:?}"
    );
    let canonical = fixture.checkpoint_path();
    let singleton = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&canonical, &singleton).unwrap();
    let event_leaf = fixture.event_leaf();
    let original_event = fs::read(&event_leaf).unwrap();
    let private_event: Value = serde_json::from_slice(&original_event).unwrap();
    let private_values = [
        private_event["event"]["event_id"]
            .as_str()
            .unwrap()
            .to_owned(),
        private_event["event"]["context_id"]
            .as_str()
            .unwrap()
            .to_owned(),
        private_event["event"]["candidate_id"]
            .as_str()
            .unwrap()
            .to_owned(),
        event_leaf
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    ];
    let mut duplicated_event = original_event.clone();
    duplicated_event.extend_from_slice(&original_event);
    fs::write(&event_leaf, duplicated_event).unwrap();
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();

    let diagnosis = fixture.run_args(&["--json", "diagnose"]);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    let next = fixture.run_args(&["--json", "next"]);

    assert_eq!(diagnosis.status.code(), Some(1), "{diagnosis:?}");
    assert_eq!(next.status.code(), Some(1), "{next:?}");
    assert!(diagnosis.stderr.is_empty(), "{diagnosis:?}");
    assert!(next.stderr.is_empty(), "{next:?}");
    assert_eq!(diagnosis.stdout, next.stdout);
    let diagnosis_value = Fixture::value(&diagnosis);
    assert_eq!(diagnosis_value, Fixture::value(&next));
    assert_eq!(
        diagnosis_value["schema_version"],
        "RoutineStateMigrationAdmission-v7"
    );
    assert_eq!(
        diagnosis_value["status"],
        "terminal_event_history_conflicting_preserve_and_hold"
    );
    assert_eq!(
        diagnosis_value["history_relation"],
        "terminal_event_history_conflicting"
    );
    assert_eq!(diagnosis_value["legacy_singleton_count"], 1);
    assert_eq!(diagnosis_value["canonical_continuation_count"], 1);
    assert_eq!(diagnosis_value["event_journal_count"], 1);
    assert_eq!(diagnosis_value["migration_effect"], "none");
    assert_eq!(diagnosis_value["migration_authorized"], false);
    assert_eq!(diagnosis_value["claim_effect"], "none");
    assert!(diagnosis_value["quarantine_plan"].is_null());
    assert!(
        !diagnosis_value["next_action"]
            .as_str()
            .unwrap()
            .contains("migrate apply")
    );
    let public = String::from_utf8_lossy(&diagnosis.stdout);
    assert!(!public.contains(fixture.root.to_str().unwrap()));
    assert!(!public.contains(fixture.container.to_str().unwrap()));
    for private in private_values {
        assert!(
            !public.contains(&private),
            "private value escaped: {private}"
        );
    }
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    fixture.teardown_after_assertions();
}

#[test]
fn complete_multi_history_is_conflicting_and_never_mints_a_plan() {
    let mut fixture = Fixture::new(
        "legacy-complete-multi-history",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let completed = fixture.run();
    assert!(
        matches!(completed.status.code(), Some(0 | 1)),
        "{completed:?}"
    );
    assert!(matches!(
        Fixture::value(&completed)["status"].as_str(),
        Some("executed" | "incomplete")
    ));
    let matching_checkpoint = fixture.checkpoint_path();

    let mut disjoint = Fixture::new(
        "legacy-complete-multi-history-disjoint",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        false,
    );
    let mut disjoint_run = routine_command(&disjoint.root, &fixture.home, disjoint.binary_path());
    disjoint_run.args(["--json", "check", "routine"]);
    let disjoint_run = disjoint_run.output().unwrap();
    assert!(
        matches!(disjoint_run.status.code(), Some(0 | 1)),
        "{disjoint_run:?}"
    );
    assert!(matches!(
        Fixture::value(&disjoint_run)["status"].as_str(),
        Some("executed" | "incomplete")
    ));

    let singleton = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&matching_checkpoint, &singleton).unwrap();
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();

    let diagnosis = fixture.run_args(&["--json", "diagnose"]);
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    let next = fixture.run_args(&["--json", "next"]);

    assert_eq!(diagnosis.status.code(), Some(1), "{diagnosis:?}");
    assert_eq!(next.status.code(), Some(1), "{next:?}");
    assert!(diagnosis.stderr.is_empty(), "{diagnosis:?}");
    assert!(next.stderr.is_empty(), "{next:?}");
    assert_eq!(diagnosis.stdout, next.stdout);
    let diagnosis_value = Fixture::value(&diagnosis);
    let next_value = Fixture::value(&next);
    assert_eq!(diagnosis_value, next_value);
    assert_eq!(
        diagnosis_value["schema_version"],
        "RoutineStateMigrationAdmission-v7"
    );
    assert_eq!(
        diagnosis_value["status"],
        "conflicting_histories_preserve_and_hold"
    );
    assert_eq!(
        diagnosis_value["history_relation"],
        "conflicting_histories_hold"
    );
    assert_eq!(diagnosis_value["legacy_singleton_count"], 1);
    assert_eq!(diagnosis_value["canonical_continuation_count"], 2);
    assert_eq!(diagnosis_value["event_journal_count"], 2);
    assert_eq!(diagnosis_value["migration_effect"], "none");
    assert_eq!(diagnosis_value["migration_authorized"], false);
    assert_eq!(diagnosis_value["claim_effect"], "none");
    assert!(diagnosis_value["quarantine_plan"].is_null());
    assert!(
        !diagnosis_value["next_action"]
            .as_str()
            .unwrap()
            .contains("migrate apply")
    );
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);

    disjoint.teardown_after_assertions();
    fixture.teardown_after_assertions();
}

#[test]
fn unknown_format_is_a_typed_read_only_hold() {
    let mut fixture = Fixture::new(
        "legacy-unknown-format",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::write(
        fixture.state_root().join("routine-state-format"),
        b"routine-host-state-v7\n",
    )
    .unwrap();
    let before = tree(&fixture.home);

    let output = fixture.run_args(&["--json", "diagnose"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "unknown_format_hold");
    assert_eq!(value["format_status"], "unsupported");
    assert_eq!(value["migration_authorized"], false);
    assert_eq!(tree(&fixture.home), before);
    fixture.teardown_after_assertions();
}

#[test]
fn reserved_parent_transition_names_remain_planless_and_zero_write() {
    for (label, reserved_name) in [
        ("bootstrap", ".routine-public-bootstrap"),
        (
            "quarantine",
            "routine-public.quarantine-untrusted-collision",
        ),
    ] {
        let mut fixture = Fixture::new(
            &format!("legacy-reserved-parent-{label}"),
            &[pass_node("compile", &[])],
            &[prefix_route("route-src", "src", &["compile"])],
            true,
            true,
        );
        fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
        let outside = fixture.container.join(format!("outside-{label}"));
        fs::write(&outside, b"outside evaluator-owned bytes\n").unwrap();
        let collision = fixture.state_root().parent().unwrap().join(reserved_name);
        symlink(&outside, &collision).unwrap();
        let before_root = tree(&fixture.root);
        let before_home = tree(&fixture.home);
        let before_status = fixture.status();
        let before_outside = fs::read(&outside).unwrap();

        let diagnosis = fixture.run_args(&["--json", "diagnose"]);
        assert_eq!(fs::read(&outside).unwrap(), before_outside);
        assert_eq!(tree(&fixture.root), before_root);
        assert_eq!(tree(&fixture.home), before_home);
        assert_eq!(fixture.status(), before_status);
        let next = fixture.run_args(&["--json", "next"]);

        assert_eq!(diagnosis.status.code(), Some(1), "{diagnosis:?}");
        assert_eq!(next.status.code(), Some(1), "{next:?}");
        assert!(diagnosis.stdout.is_empty(), "{diagnosis:?}");
        assert!(next.stdout.is_empty(), "{next:?}");
        let diagnosis_value: Value = serde_json::from_slice(&diagnosis.stderr).unwrap();
        let next_value: Value = serde_json::from_slice(&next.stderr).unwrap();
        for value in [&diagnosis_value, &next_value] {
            assert_eq!(value["schema_version"], "HarnessDiagnostic-v1");
            assert_eq!(
                value["diagnostic_id"],
                "successor_runtime_state_unavailable"
            );
            assert_eq!(value["exit_class"], "actionable_finding");
            assert_eq!(value["effect"], "read");
            assert!(value.get("quarantine_plan").is_none());
            let public = value.to_string();
            assert!(!public.contains(fixture.container.to_str().unwrap()));
            assert!(!public.contains(reserved_name));
            assert!(!public.contains("migrate apply"));
        }
        assert_eq!(diagnosis_value["exact_rerun"], "ultragoal --json diagnose");
        assert_eq!(next_value["exact_rerun"], "ultragoal --json next");
        assert_eq!(fs::read(&outside).unwrap(), before_outside);
        assert_eq!(tree(&fixture.root), before_root);
        assert_eq!(tree(&fixture.home), before_home);
        assert_eq!(fixture.status(), before_status);
        fixture.teardown_after_assertions();
    }
}

#[test]
fn unsafe_legacy_entry_is_not_promoted_to_a_migration_candidate() {
    let mut fixture = Fixture::new(
        "legacy-unsafe-entry",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let outside = fixture.container.join("outside");
    fs::write(&outside, b"outside\n").unwrap();
    symlink(&outside, fixture.state_root().join("adapter/unsafe-entry")).unwrap();
    let before = tree(&fixture.home);

    let output = fixture.run_args(&["--json", "diagnose"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "unsafe_layout_preserve_and_hold");
    assert_eq!(value["migration_authorized"], false);
    assert_eq!(tree(&fixture.home), before);
    fixture.teardown_after_assertions();
}

#[test]
fn active_legacy_writer_returns_a_preserve_and_retry_hold() {
    let mut fixture = Fixture::new(
        "legacy-active-writer",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW)
        .open(fixture.lock_path())
        .unwrap();
    // SAFETY: the descriptor is owned by `lock` and the advisory lock is
    // released when it is dropped at the end of this test.
    assert_eq!(unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) }, 0);
    let before = tree(&fixture.home);

    let output = fixture.run_args(&["--json", "diagnose"]);

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "stale_writer_preserve_and_retry");
    assert_eq!(value["migration_authorized"], false);
    assert_eq!(tree(&fixture.home), before);
    drop(lock);
    fixture.teardown_after_assertions();
}

#[test]
fn restored_source_bytes_with_a_replaced_read_identity_rerun_then_reuse() {
    let mut fixture = Fixture::new(
        "restored-source-identity-rerun",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let first = fixture.run();
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    assert_eq!(Fixture::value(&first)["status"], "executed");

    let source = fixture.root.join("src/lib.rs");
    let replacement = fixture.container.join("restored-lib.rs");
    let original = fs::read(&source).unwrap();
    fs::write(&replacement, &original).unwrap();
    fs::rename(&replacement, &source).unwrap();
    assert_eq!(fs::read(&source).unwrap(), original);

    let diagnosis = fixture.run_args(&["--json", "diagnose"]);
    assert_eq!(diagnosis.status.code(), Some(1), "{diagnosis:?}");
    let diagnosis = Fixture::value(&diagnosis);
    assert_eq!(diagnosis["schema_version"], "RoutineDiagnosis-v1");
    assert_eq!(diagnosis["status"], "no_record");
    assert_ne!(diagnosis["repeat_use"], "safe_reuse");

    let rerun = fixture.run();
    assert_eq!(rerun.status.code(), Some(0), "{rerun:?}");
    let rerun = Fixture::value(&rerun);
    assert_eq!(rerun["status"], "executed");
    assert_eq!(rerun["nodes"][0]["disposition"], "executed");

    assert_reused_without_effect(&fixture.run());
    fixture.teardown_after_assertions();
}

#[test]
fn legacy_complete_checkpoint_with_changed_execution_identity_reruns_then_reuses() {
    let mut fixture = Fixture::new(
        "legacy-complete-identity-rerun",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let first = fixture.run();
    assert_eq!(first.status.code(), Some(0), "{first:?}");

    migrate_complete_checkpoint_to_v6(&fixture, true);

    let source = fixture.root.join("src/lib.rs");
    let replacement = fixture.container.join("legacy-restored-lib.rs");
    let original = fs::read(&source).unwrap();
    fs::write(&replacement, &original).unwrap();
    fs::rename(&replacement, &source).unwrap();
    assert_eq!(fs::read(&source).unwrap(), original);

    let rerun = fixture.run();
    assert_eq!(rerun.status.code(), Some(0), "{rerun:?}");
    assert_eq!(Fixture::value(&rerun)["status"], "executed");
    assert_reused_without_effect(&fixture.run());
    fixture.teardown_after_assertions();
}

#[test]
fn legacy_complete_checkpoint_with_the_same_execution_identity_and_no_event_refuses() {
    let mut fixture = Fixture::new(
        "legacy-complete-exact-reuse",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let first = fixture.run();
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    migrate_complete_checkpoint_to_v6(&fixture, true);

    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    assert_terminal_refusal(&fixture.run());
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);
    fixture.teardown_after_assertions();
}

#[test]
fn compatibility_v6_checkpoint_with_a_retained_v7_event_leaf_refuses_pre_effect() {
    let mut fixture = Fixture::new(
        "legacy-complete-retained-current-event",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let first = fixture.run();
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    migrate_complete_checkpoint_to_v6(&fixture, false);

    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    let before_status = fixture.status();
    assert_terminal_refusal(&fixture.run());
    assert_eq!(tree(&fixture.root), before_root);
    assert_eq!(tree(&fixture.home), before_home);
    assert_eq!(fixture.status(), before_status);

    fixture.teardown_after_assertions();
}

fn migrate_complete_checkpoint_to_v6(fixture: &Fixture, remove_event_leaf: bool) {
    let event_leaf = remove_event_leaf.then(|| fixture.event_leaf());
    let checkpoint = fixture.checkpoint_path();
    let mut legacy: Value = serde_json::from_slice(&fs::read(&checkpoint).unwrap()).unwrap();
    legacy["schema_version"] = Value::String("RoutineContinuationCheckpoint-v6".to_owned());
    let object = legacy.as_object_mut().unwrap();
    object.remove("execution_id");
    let legacy_name = legacy_checkpoint_name(&legacy);
    let legacy_path = fixture.continuations_root().join(legacy_name);
    fs::remove_file(&checkpoint).unwrap();
    fs::write(&legacy_path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    fs::set_permissions(
        &legacy_path,
        std::os::unix::fs::PermissionsExt::from_mode(0o600),
    )
    .unwrap();
    if let Some(event_leaf) = event_leaf {
        fs::remove_file(event_leaf).unwrap();
    }
}

fn legacy_checkpoint_name(checkpoint: &Value) -> String {
    let mut digest = Sha256::new();
    digest.update(b"routine-continuation-record-v1\0");
    for field in ["target", "context_id", "candidate_id", "plan_id"] {
        digest.update(checkpoint[field].as_str().unwrap().as_bytes());
        digest.update([0]);
    }
    digest.update(checkpoint["snapshot_id"].as_str().unwrap().as_bytes());
    format!("routine-continuation-{:x}.json", digest.finalize())
}
#[test]
fn authorized_fresh_execution_then_exact_repeat_reuses_without_a_second_effect() {
    let mut fixture = Fixture::new(
        "authorized-fresh-repeat",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    assert!(!fixture.root.join("target").exists());

    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    assert!(executed.stderr.is_empty(), "{executed:?}");
    let executed = Fixture::value(&executed);
    assert_eq!(executed["status"], "executed");
    assert_eq!(executed["nodes"][0]["disposition"], "executed");
    let scope = fixture.root.join("target/routine/compile");
    assert!(scope.is_dir());
    assert_eq!(fs::read_dir(&scope).unwrap().count(), 0);

    assert_reused_without_effect(&fixture.run());
    assert_eq!(fs::read_dir(scope).unwrap().count(), 0);
    fixture.teardown_after_assertions();
}

#[test]
fn reservation_interruption_reconciles_once_through_the_public_continuation_route() {
    let mut fixture = Fixture::new(
        "reservation-interruption-continuation",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let before = tree(&fixture.root);
    let mut interrupted = fixture.base_command();
    interrupted.args([
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    let interrupted = interrupted.output().unwrap();
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    assert!(interrupted.stderr.is_empty(), "{interrupted:?}");
    let interrupted_value = Fixture::value(&interrupted);
    assert_eq!(
        interrupted_value["schema_version"],
        "RoutinePublicProductionOutcome-v2"
    );
    assert_eq!(interrupted_value["status"], "interrupted-reservation");
    assert_eq!(interrupted_value["effect"], "none");
    assert_eq!(interrupted_value["recovery_required"], true);
    let continuation = interrupted_value["continuation"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(continuation.starts_with("routine-cont-"));
    let reserved: Value = serde_json::from_slice(&fs::read(fixture.checkpoint_path()).unwrap())
        .expect("reservation checkpoint is not JSON");
    assert_eq!(reserved["state"], "reserved");
    assert_eq!(reserved["operation"], "terminal");
    assert!(reserved["terminal_outcome"].is_null());
    assert_eq!(reserved["continuation"], continuation);
    assert!(reserved["attempt_grant"].as_str().is_some());
    assert!(reserved["authenticated_ledger_head"].as_str().is_some());
    assert_eq!(tree(&fixture.root), before);

    let before_foreign = tree(&fixture.root);
    let mut foreign = fixture.base_command();
    foreign.args([
        "--json",
        "check",
        "routine",
        "--continuation",
        "routine-cont-foreign",
    ]);
    let foreign = foreign.output().unwrap();
    assert_eq!(foreign.status.code(), Some(3), "{foreign:?}");
    assert!(foreign.stdout.is_empty(), "{foreign:?}");
    assert_eq!(tree(&fixture.root), before_foreign);

    let mut recovered = fixture.base_command();
    recovered.args([
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    let recovered = recovered.output().unwrap();
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    assert!(recovered.stderr.is_empty(), "{recovered:?}");
    let recovered_value = Fixture::value(&recovered);
    assert_eq!(recovered_value["status"], "executed");
    assert_eq!(recovered_value["nodes"][0]["disposition"], "executed");

    let before_replay = tree(&fixture.root);
    let mut replay = fixture.base_command();
    replay.args([
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    let replay = replay.output().unwrap();
    assert_eq!(replay.status.code(), Some(3), "{replay:?}");
    assert!(replay.stdout.is_empty(), "{replay:?}");
    assert_eq!(tree(&fixture.root), before_replay);
    fixture.teardown_after_assertions();
}

#[test]
fn caller_known_continuation_survives_a_replacement_reservation_before_result_delivery() {
    let mut fixture = Fixture::new(
        "replacement-reservation-caller-handoff",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let first = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(first.status.code(), Some(1), "{first:?}");
    let continuation = Fixture::value(&first)["continuation"]
        .as_str()
        .unwrap()
        .to_owned();

    let replacement = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(replacement.status.code(), Some(1), "{replacement:?}");
    let replacement_value = Fixture::value(&replacement);
    assert_eq!(replacement_value["status"], "interrupted-reservation");
    assert_ne!(replacement_value["continuation"], continuation);
    let replacement_continuation = replacement_value["continuation"]
        .as_str()
        .unwrap()
        .to_owned();
    let second_replacement = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--continuation",
        &replacement_continuation,
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(
        second_replacement.status.code(),
        Some(1),
        "{second_replacement:?}"
    );
    assert_eq!(
        Fixture::value(&second_replacement)["status"],
        "interrupted-reservation"
    );
    let replacement_checkpoint: Value =
        serde_json::from_slice(&fs::read(fixture.checkpoint_path()).unwrap()).unwrap();
    let mut expected_predecessors = vec![continuation.clone(), replacement_continuation.clone()];
    expected_predecessors.sort();
    assert_eq!(
        replacement_checkpoint["predecessor_continuations"],
        serde_json::json!(expected_predecessors)
    );
    assert_eq!(
        fs::read_dir(fixture.continuations_root())
            .unwrap()
            .filter_map(Result::ok)
            .filter(
                |entry| entry.path().extension().and_then(|value| value.to_str()) == Some("json")
            )
            .count(),
        1,
    );

    let recovered = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    assert_eq!(Fixture::value(&recovered)["status"], "executed");

    let stale = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    assert_eq!(stale.status.code(), Some(3), "{stale:?}");
    fixture.teardown_after_assertions();
}

#[test]
fn reconciled_reservation_republishes_when_legacy_filename_belongs_to_another_binding() {
    let mut interrupted = Fixture::new(
        "reconciled-republish-with-foreign-legacy",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut foreign = Fixture::new(
        "foreign-legacy-for-reconciled-republish",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut foreign_run = routine_command(&foreign.root, &interrupted.home, foreign.binary_path());
    foreign_run.args(["--json", "check", "routine"]);
    let foreign_run = foreign_run.output().unwrap();
    assert_eq!(foreign_run.status.code(), Some(0), "{foreign_run:?}");
    assert_eq!(Fixture::value(&foreign_run)["status"], "executed");
    let foreign_record = interrupted.checkpoint_path();
    let legacy = interrupted
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&foreign_record, &legacy).unwrap();
    let foreign_legacy = fs::read(&legacy).unwrap();

    let first = interrupted.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(first.status.code(), Some(1), "{first:?}");
    let continuation = Fixture::value(&first)["continuation"]
        .as_str()
        .unwrap()
        .to_owned();

    let recovered = interrupted.run_args(&[
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    assert_eq!(Fixture::value(&recovered)["status"], "executed");
    assert_eq!(fs::read(&legacy).unwrap(), foreign_legacy);

    foreign.teardown_after_assertions();
    interrupted.teardown_after_assertions();
}

#[test]
fn public_output_creation_is_observed_only_after_the_durable_journal() {
    let mut fixture = Fixture::new(
        "durable-output-order",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let scope = fixture.root.join("target/routine/compile");
    let state = fixture.authority_root().join("routine-authority.state");
    let stopped = Arc::new(AtomicBool::new(false));
    let watcher_stopped = Arc::clone(&stopped);
    let watcher = std::thread::spawn(move || {
        loop {
            if scope.is_dir() {
                let durable =
                    fs::read(&state).map_err(|_| "output appeared before durable state")?;
                let text = String::from_utf8(durable).map_err(|_| "durable state is not UTF-8")?;
                if !text.contains("\"output_journal\"") || !text.contains("target/routine/compile")
                {
                    return Err("output appeared before its durable journal binding");
                }
                return Ok(());
            }
            if watcher_stopped.load(Ordering::Acquire) {
                return Err("child exited without provisioning output");
            }
            std::thread::sleep(Duration::from_millis(2));
        }
    });
    let mut command = fixture.base_command();
    command.args(["--json", "check", "routine"]);
    let observed = contain_contender(
        run_bounded_contender(&mut command, Duration::from_secs(60)),
        Duration::from_secs(2),
    );
    stopped.store(true, Ordering::Release);
    let journal = watcher.join().expect("journal watcher panicked");
    let outcome = match observed {
        ContainedContender::Exited(output) => Ok(output),
        ContainedContender::TerminatedAndReaped(_) => Err("public command timed out"),
        ContainedContender::ExitedAtDeadline(_) => Err("public command exited after deadline"),
        ContainedContender::ReapedWithFailure(_) => Err("public command cleanup failed"),
    };
    fixture.teardown_after_assertions();
    assert_eq!(journal, Ok(()));
    let output = outcome.unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
}

#[test]
fn terminal_failure_serializes_no_recovery_and_fresh_process_refuses_takeover() {
    let mut fixture = Fixture::new(
        "terminal-failure-retry",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::write(fixture.root.join("src/lib.rs"), b"pub fn broken(\n").unwrap();

    let output = fixture.run();
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value = Fixture::value(&output);
    assert_eq!(value["status"], "incomplete");
    assert_eq!(value["recovery_required"], false);
    assert_eq!(value["nodes"][0]["disposition"], "failed");
    let terminal_event = read_terminal_event(&fixture);
    assert_eq!(terminal_event["event"]["outcome"], "fail");
    assert_eq!(
        terminal_event["event"]["public_attributes"]["routine_transition"],
        "failed"
    );
    assert_eq!(
        terminal_event["event"]["public_attributes"]["routine_terminal_outcome"],
        "failed"
    );
    let checkpoint: Value = serde_json::from_slice(&fs::read(fixture.checkpoint_path()).unwrap())
        .expect("terminal checkpoint is not JSON");
    assert_eq!(checkpoint["state"], "terminal-event-joined");
    assert_eq!(checkpoint["terminal_outcome"], "failed");
    assert_terminal_refusal(&fixture.run());
    fixture.teardown_after_assertions();
}

#[test]
fn untracked_legacy_target_spool_is_ignored_and_preserved_by_execution_and_reuse() {
    let mut fixture = Fixture::new(
        "missing-observability-ignore",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    fs::write(fixture.root.join(".gitignore"), b"target/\n").unwrap();
    let legacy = fixture
        .root
        .join("validation_artifacts/observability/spool/legacy.jsonl");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, b"untracked-legacy-store\n").unwrap();
    let before_status = fixture.status();
    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    assert_eq!(Fixture::value(&executed)["status"], "executed");
    assert_reused_without_effect(&fixture.run());
    assert_eq!(fixture.status(), before_status);
    assert_eq!(fs::read(legacy).unwrap(), b"untracked-legacy-store\n");
    assert_eq!(
        fs::read(fixture.root.join(".gitignore")).unwrap(),
        b"target/\n"
    );
    let _host_event = fixture.event_leaf();
    fixture.teardown_after_assertions();
}

#[test]
fn settled_history_survives_removal_of_its_target_before_a_new_journey() {
    let mut fixture = Fixture::new(
        "settled-history-missing-target",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let completed = fixture.run();
    assert_eq!(completed.status.code(), Some(0), "{completed:?}");
    assert_eq!(Fixture::value(&completed)["status"], "executed");

    let relocated = fixture.container.join("relocated-repo");
    fs::rename(&fixture.root, &relocated).unwrap();
    assert!(!fixture.root.exists());
    let relocated = fs::canonicalize(relocated).unwrap();
    fs::remove_dir_all(relocated.join("target")).unwrap();
    let mut command = routine_command(&relocated, &fixture.home, fixture.binary_path());
    command.args(["--json", "check", "routine"]);
    let next = command.output().unwrap();
    assert_eq!(next.status.code(), Some(0), "{next:?}");
    assert_eq!(Fixture::value(&next)["status"], "executed");

    fixture.teardown_after_assertions();
}

#[test]
fn tracked_legacy_target_spool_is_ignored_and_preserved_by_execution_and_reuse() {
    let mut fixture = Fixture::new(
        "tracked-observability-store",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let store = fixture
        .root
        .join("validation_artifacts/observability/spool/successor-events-forged.jsonl");
    fs::create_dir_all(store.parent().unwrap()).unwrap();
    fs::write(&store, b"tracked-runtime-store\n").unwrap();
    git(
        &fixture.root,
        &[
            "add",
            "-f",
            "validation_artifacts/observability/spool/successor-events-forged.jsonl",
        ],
    );
    git(
        &fixture.root,
        &["commit", "--quiet", "-m", "tracked runtime store"],
    );

    let before_status = fixture.status();
    let executed = fixture.run();
    assert_eq!(executed.status.code(), Some(0), "{executed:?}");
    assert_eq!(Fixture::value(&executed)["status"], "executed");
    assert_reused_without_effect(&fixture.run());
    assert_eq!(fixture.status(), before_status);
    assert_eq!(fs::read(store).unwrap(), b"tracked-runtime-store\n");
    let _host_event = fixture.event_leaf();
    fixture.teardown_after_assertions();
}

fn read_terminal_event(fixture: &Fixture) -> Value {
    let path = fixture.event_leaf();
    let text = fs::read_to_string(path).expect("terminal event was not appended");
    serde_json::from_str(text.trim()).expect("terminal event is not JSON")
}

#[test]
fn independent_fresh_authority_roots_execute_once_and_reuse_their_own_bindings() {
    let mut first = Fixture::new(
        "binding-first",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut second = Fixture::new(
        "binding-second",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    assert_status(&first, "executed");
    assert_status(&second, "executed");
    assert_reused_without_effect(&first.run());
    assert_reused_without_effect(&second.run());
    second.teardown_after_assertions();
    first.teardown_after_assertions();
}

#[test]
fn shared_host_continuations_keep_foreign_legacy_and_repository_records_independent() {
    let mut first = Fixture::new(
        "shared-home-first",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut second = Fixture::new(
        "shared-home-second",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut interrupted = first.base_command();
    interrupted.args([
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    let interrupted = interrupted.output().unwrap();
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    let first_record = first.checkpoint_path();
    let legacy = first.state_root().join("adapter/routine-continuation.json");
    fs::copy(&first_record, &legacy).unwrap();

    let mut second_run = routine_command(&second.root, &first.home, second.binary_path());
    second_run.args(["--json", "check", "routine"]);
    let output = second_run.output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(Fixture::value(&output)["status"], "executed");
    assert_eq!(fs::read_dir(first.continuations_root()).unwrap().count(), 2);
    assert_eq!(fs::read(&legacy).unwrap(), fs::read(&first_record).unwrap());

    second.teardown_after_assertions();
    first.teardown_after_assertions();
}

#[test]
fn one_repository_can_reopen_a_distinct_binding_without_replacing_the_prior_record() {
    let mut fixture = Fixture::new(
        "same-repository-distinct-binding",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let first = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(first.status.code(), Some(1), "{first:?}");
    let first_record = fixture.checkpoint_path();
    fs::write(
        fixture.root.join("src/lib.rs"),
        b"pub fn value() -> u8 { 3 }\n",
    )
    .unwrap();
    let second = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(second.status.code(), Some(1), "{second:?}");
    let records = fs::read_dir(fixture.continuations_root())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 2);
    assert!(records.iter().any(|path| path == &first_record));
    fixture.teardown_after_assertions();
}

#[test]
fn completed_binding_reuses_after_a_distinct_binding_recovers_without_relaxing_stale_reservations()
{
    let mut fixture = Fixture::new(
        "completed-binding-reuses-after-distinct-recovery",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let original = fixture.run();
    assert_eq!(original.status.code(), Some(0), "{original:?}");
    assert_eq!(Fixture::value(&original)["status"], "executed");

    let control = fixture.root.join("src/recovery-control.rs");
    fs::write(&control, b"pub const RECOVERY_CONTROL: u8 = 1;\n").unwrap();
    let interrupted = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    let continuation = Fixture::value(&interrupted)["continuation"]
        .as_str()
        .unwrap()
        .to_owned();
    let recovered = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    assert_eq!(Fixture::value(&recovered)["status"], "executed");

    fs::remove_file(&control).unwrap();
    let before_reuse = tree(&fixture.root);
    assert_reused_without_effect(&fixture.run());
    assert_eq!(tree(&fixture.root), before_reuse);

    fixture.teardown_after_assertions();
}

#[test]
fn stale_reserved_binding_refuses_after_a_distinct_binding_advances_the_ledger() {
    let mut fixture = Fixture::new(
        "stale-reserved-binding-refusal",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let interrupted = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    let continuation = Fixture::value(&interrupted)["continuation"]
        .as_str()
        .unwrap()
        .to_owned();
    let reserved_checkpoint = fixture.checkpoint_path();

    let control = fixture.root.join("src/ledger-advance.rs");
    fs::write(&control, b"pub const LEDGER_ADVANCE: u8 = 1;\n").unwrap();
    let advanced = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(advanced.status.code(), Some(1), "{advanced:?}");
    let advanced_value = Fixture::value(&advanced);
    assert_eq!(advanced_value["status"], "interrupted-reservation");
    assert_eq!(advanced_value["effect"], "none");
    assert_ne!(advanced_value["continuation"], continuation);
    let advanced_checkpoint = fs::read_dir(fixture.continuations_root())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path != &reserved_checkpoint && path.extension().is_some_and(|ext| ext == "json")
        })
        .unwrap();

    fs::remove_file(&control).unwrap();
    let stale = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    assert_eq!(stale.status.code(), Some(3), "{stale:?}");
    assert!(stale.stdout.is_empty(), "{stale:?}");
    fs::remove_file(advanced_checkpoint).unwrap();

    let singleton = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&reserved_checkpoint, &singleton).unwrap();
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);

    for command in [&["--json", "diagnose"][..], &["--json", "next"][..]] {
        let output = fixture.run_args(command);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let value = Fixture::value(&output);
        assert_eq!(value["schema_version"], "RoutineStateMigrationAdmission-v7");
        assert_eq!(
            value["status"], "stale_reserved_abandoned_candidate_preserve_and_hold",
            "{output:?}"
        );
        assert_eq!(value["history_relation"], "redundant_equivalent");
        assert_eq!(value["migration_effect"], "none");
        assert_eq!(value["migration_authorized"], false);
        assert_eq!(
            value["quarantine_plan"]["schema_version"],
            "RoutineStateQuarantinePlan-v5"
        );
        assert_eq!(
            value["quarantine_plan"]["authoritative_history"],
            "none_noncurrent_value_only_historical_head_unproven"
        );
        assert_eq!(
            value["quarantine_plan"]["apply_capability"],
            "available_exact_record_only"
        );
        let assessment = &value["reserved_recovery"];
        assert_eq!(
            assessment["schema_version"],
            "RoutineReservedRecoveryAssessment-v1"
        );
        assert_eq!(
            assessment["checkpoint_relation"],
            "noncurrent_head_matching_private_attempt"
        );
        assert_eq!(
            assessment["checkpoint_head_evidence"],
            "noncurrent_value_only_historical_head_unproven"
        );
        assert_eq!(assessment["private_attempt_state"], "reserved");
        assert_eq!(assessment["effect_evidence"], "pristine_no_effect_observed");
        assert_eq!(
            assessment["owner_observation"],
            "owner_process_not_observed"
        );
        assert_eq!(assessment["recovery_effect"], "none");
        assert_eq!(assessment["recovery_authorized"], false);
        assert_eq!(assessment["claim_effect"], "none");
        assert_eq!(tree(&fixture.root), before_root);
        assert_eq!(tree(&fixture.home), before_home);
    }

    let unproven_head = format!("sha256:{}", "c".repeat(64));
    let mut relabelled: Value =
        serde_json::from_slice(&fs::read(&reserved_checkpoint).unwrap()).unwrap();
    let mut event_id = Sha256::new();
    event_id.update(b"routine-terminal-event-v1\0");
    event_id.update(continuation.as_bytes());
    event_id.update(b"\0");
    event_id.update(unproven_head.as_bytes());
    relabelled["authenticated_ledger_head"] = Value::from(unproven_head);
    relabelled["event_id"] = Value::from(format!("routine-terminal-{:x}", event_id.finalize()));
    let relabelled = serde_json::to_vec(&relabelled).unwrap();
    fs::write(&reserved_checkpoint, &relabelled).unwrap();
    fs::write(&singleton, &relabelled).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    for command in [&["--json", "diagnose"][..], &["--json", "next"][..]] {
        let output = fixture.run_args(command);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let value = Fixture::value(&output);
        assert_eq!(
            value["status"],
            "stale_reserved_abandoned_candidate_preserve_and_hold"
        );
        assert_eq!(
            value["reserved_recovery"]["checkpoint_head_evidence"],
            "noncurrent_value_only_historical_head_unproven"
        );
        assert_eq!(value["reserved_recovery"]["recovery_effect"], "none");
        assert_eq!(value["reserved_recovery"]["recovery_authorized"], false);
        assert_eq!(tree(&fixture.root), before_root);
        assert_eq!(tree(&fixture.home), before_home);
    }

    let mut semantically_equal = fs::read(&singleton).unwrap();
    semantically_equal.extend_from_slice(b" \n");
    fs::write(&singleton, semantically_equal).unwrap();
    let before_root = tree(&fixture.root);
    let before_home = tree(&fixture.home);
    for command in [&["--json", "diagnose"][..], &["--json", "next"][..]] {
        let output = fixture.run_args(command);
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let value = Fixture::value(&output);
        assert_eq!(value["schema_version"], "RoutineStateMigrationAdmission-v7");
        assert_eq!(
            value["status"],
            "stale_reserved_abandoned_candidate_preserve_and_hold"
        );
        assert_eq!(value["history_relation"], "redundant_equivalent");
        assert!(value["reserved_recovery"].is_object());
        assert_eq!(value["migration_effect"], "none");
        assert_eq!(value["migration_authorized"], false);
        assert!(value["quarantine_plan"].is_null());
        assert!(
            !value["next_action"]
                .as_str()
                .unwrap()
                .contains("migrate apply")
        );
        assert_eq!(tree(&fixture.root), before_root);
        assert_eq!(tree(&fixture.home), before_home);
    }
    fixture.teardown_after_assertions();
}

#[test]
fn exact_abandoned_stale_reserved_record_quarantines_without_importing_history() {
    let mut fixture = Fixture::new(
        "stale-reserved-quarantine-apply",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let interrupted = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(interrupted.status.code(), Some(1), "{interrupted:?}");
    let reserved_checkpoint = fixture.checkpoint_path();

    let control = fixture.root.join("src/stale-plan-advance.rs");
    fs::write(&control, b"pub const STALE_PLAN_ADVANCE: u8 = 1;\n").unwrap();
    let advanced = fixture.run_args(&[
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    assert_eq!(advanced.status.code(), Some(1), "{advanced:?}");
    let advanced_checkpoint = fs::read_dir(fixture.continuations_root())
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| {
            path != &reserved_checkpoint && path.extension().is_some_and(|ext| ext == "json")
        })
        .unwrap();
    fs::remove_file(control).unwrap();
    fs::remove_file(advanced_checkpoint).unwrap();

    let singleton = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&reserved_checkpoint, &singleton).unwrap();
    fs::remove_file(fixture.state_root().join("routine-state-format")).unwrap();
    let legacy_tree = tree(&fixture.state_root());

    let diagnosis = fixture.run_args(&["--json", "diagnose"]);
    assert_eq!(diagnosis.status.code(), Some(1), "{diagnosis:?}");
    let value = Fixture::value(&diagnosis);
    assert_eq!(
        value["status"],
        "stale_reserved_abandoned_candidate_preserve_and_hold"
    );
    assert_eq!(
        value["reserved_recovery"]["effect_evidence"],
        "pristine_no_effect_observed"
    );
    assert_eq!(
        value["reserved_recovery"]["owner_observation"],
        "owner_process_not_observed"
    );
    let plan = &value["quarantine_plan"];
    assert_eq!(plan["schema_version"], "RoutineStateQuarantinePlan-v5");
    assert_eq!(
        plan["authoritative_history"],
        "none_noncurrent_value_only_historical_head_unproven"
    );
    let plan_id = plan["plan_id"].as_str().unwrap().to_owned();
    let quarantine_owner = plan["quarantine_owner"].as_str().unwrap().to_owned();
    let relative = "migration/stale-reserved-quarantine.json";
    let record_path = fixture.root.join(relative);
    fs::create_dir_all(record_path.parent().unwrap()).unwrap();
    fs::write(&record_path, &diagnosis.stdout).unwrap();
    set_mode(&record_path, 0o600);

    let applied = fixture.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        relative,
        "--accept-plan",
        &plan_id,
    ]);
    assert_eq!(applied.status.code(), Some(0), "{applied:?}");
    let applied = Fixture::value(&applied);
    assert_eq!(applied["status"], "applied");
    assert_eq!(applied["legacy_authority_imported"], false);
    let quarantine = fixture
        .state_root()
        .parent()
        .unwrap()
        .join(quarantine_owner);
    assert_eq!(tree(&quarantine), legacy_tree);
    assert_eq!(
        fs::read(fixture.state_root().join("routine-state-format")).unwrap(),
        b"routine-host-state-v8\n"
    );

    let replay = fixture.run_args(&[
        "--json",
        "migrate",
        "apply",
        "--plan",
        relative,
        "--accept-plan",
        &plan_id,
    ]);
    assert_eq!(replay.status.code(), Some(0), "{replay:?}");
    assert_eq!(
        Fixture::value(&replay)["status"],
        "already_applied_no_effect"
    );

    fixture.teardown_after_assertions();
}

#[test]
fn exact_legacy_checkpoint_remains_recoverable_at_the_compatibility_boundary() {
    let mut fixture = Fixture::new(
        "legacy-continuation-compatibility",
        &[pass_node("compile", &[])],
        &[prefix_route("route-src", "src", &["compile"])],
        true,
        true,
    );
    let mut interrupted = fixture.base_command();
    interrupted.args([
        "--json",
        "check",
        "routine",
        "--interrupt-after",
        "reservation",
    ]);
    let interrupted = interrupted.output().unwrap();
    let continuation = Fixture::value(&interrupted)["continuation"]
        .as_str()
        .unwrap()
        .to_owned();
    let canonical = fixture.checkpoint_path();
    let legacy = fixture
        .state_root()
        .join("adapter/routine-continuation.json");
    fs::copy(&canonical, &legacy).unwrap();
    fs::remove_file(canonical).unwrap();

    let mut recovered = fixture.base_command();
    recovered.args([
        "--json",
        "check",
        "routine",
        "--continuation",
        &continuation,
    ]);
    let recovered = recovered.output().unwrap();
    assert_eq!(recovered.status.code(), Some(0), "{recovered:?}");
    assert_eq!(Fixture::value(&recovered)["status"], "executed");
    assert!(legacy.is_file());
    assert_eq!(
        fs::read_dir(fixture.continuations_root()).unwrap().count(),
        0
    );
    fixture.teardown_after_assertions();
}

fn assert_terminal_refusal(output: &std::process::Output) {
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(
        serde_json::from_slice::<Value>(&output.stderr).is_ok(),
        "{output:?}"
    );
}

fn assert_reused_without_effect(output: &std::process::Output) {
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let value = Fixture::value(output);
    assert_eq!(value["status"], "reused");
    assert_eq!(value["effect"], "none");
    assert_eq!(value["nodes"][0]["disposition"], "reused");
}

fn assert_status(fixture: &Fixture, expected: &str) {
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(Fixture::value(&output)["status"], expected);
}
