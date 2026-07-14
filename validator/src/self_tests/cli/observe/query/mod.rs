use crate::cli::observe;
use crate::cli::observe::command::ObserveOperation;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

mod command_inventory;
mod matching;
mod run;
mod spool;
mod text;

const HCT_OBSERVE_BLOCKER: &str = "HCT-OBSERVE successor catalog unavailable/not adopted";

#[test]
fn observe_green_prove_and_query_contracts_are_typed() {
    let root = super::minimal_root("observe-green-prove");
    let static_catalog = root.join("docs/generated/observability/command-inventory.json");
    fs::create_dir_all(static_catalog.parent().expect("catalog parent")).expect("catalog parent");
    fs::write(&static_catalog, "SECRET_CANARY").expect("static catalog bait");
    let health = command(&["observe", "stack", "health", "--run-id", "run-health"]);
    let smoke = command(&["observe", "stack", "smoke", "--run-id", "run-smoke"]);
    write_default_receipt(&root, &health, "pass");
    write_default_receipt(&root, &smoke, "pass");

    let prove_rel = Path::new("validation_artifacts/observability/test/prove.json");
    let prove_path = root.join(prove_rel);
    let prove = command_with_receipt(&["observe", "prove", "--run-id", "run-prove"], prove_rel);
    assert_eq!(observe::run(&root, &prove).expect("prove"), 1);
    let receipt = crate::json_boundary::read_json(&prove_path).expect("prove receipt");
    assert_eq!(receipt["status"], "fail");
    assert_eq!(receipt["supported_claims"], json!([]));
    assert!(
        receipt["why_failed"]
            .as_str()
            .is_some_and(|failure| failure.contains(HCT_OBSERVE_BLOCKER))
    );
    assert!(!receipt.to_string().contains("SECRET_CANARY"));
    assert!(
        observe::telemetry::exporter_failure_probe_for_test().starts_with("curl export failed:")
    );

    let dispatch = crate::Args {
        root: root.clone(),
        command: crate::Command::Observe(command_with_receipt(
            &["observe", "prove", "--run-id", "run-dispatch"],
            Path::new("validation_artifacts/observability/test/dispatch-prove.json"),
        )),
    };
    assert_eq!(crate::command_run::run_with_exit_code(dispatch).unwrap(), 1);
    assert!(matches!(
        crate::parse_command(&super::args(&["observe", "snapshot"])).unwrap(),
        crate::Command::Observe(_)
    ));

    let secret = command(&[
        "observe",
        "prove",
        "--claim-id",
        "token=secret",
        "--byte-limit",
        "1048577",
    ]);
    let secret_receipt =
        observe::telemetry::base_receipt(&root, &secret, "fail", Some("secret leak probe"))
            .expect("secret receipt");
    assert_eq!(secret_receipt["redaction_proof"], "fail");
    assert_eq!(secret_receipt["bounded_output_proof"], "fail");

    let health_receipt =
        crate::json_boundary::read_json(&root.join(ObserveOperation::StackHealth.receipt_rel()))
            .expect("health receipt");
    write_bad_live_receipt(
        &root,
        ObserveOperation::StackHealth,
        json!({"schema":"wrong"}),
    );
    assert!(!observe::telemetry::live_stack_receipts_current(
        &root,
        receipt["candidate_digest"].as_str().unwrap()
    ));
    write_bad_live_receipt(
        &root,
        ObserveOperation::StackHealth,
        missing_component_receipt("event", &health_receipt),
    );
    assert!(!observe::telemetry::live_stack_receipts_current(
        &root,
        receipt["candidate_digest"].as_str().unwrap()
    ));
    write_bad_live_receipt(
        &root,
        ObserveOperation::StackHealth,
        missing_component_receipt("metric", &health_receipt),
    );
    assert!(!observe::telemetry::live_stack_receipts_current(
        &root,
        receipt["candidate_digest"].as_str().unwrap()
    ));
    write_bad_live_receipt(
        &root,
        ObserveOperation::StackHealth,
        missing_component_receipt("trace", &health_receipt),
    );
    assert!(!observe::telemetry::live_stack_receipts_current(
        &root,
        receipt["candidate_digest"].as_str().unwrap()
    ));
    fs::remove_dir_all(root).expect("cleanup observe green");
}

pub(super) fn command(raw: &[&str]) -> observe::command::ObserveCommand {
    observe::parse(&super::args(raw))
        .expect("parse")
        .expect("observe command")
}

fn command_with_receipt(raw: &[&str], receipt: &Path) -> observe::command::ObserveCommand {
    let mut raw_args = super::args(raw);
    raw_args.extend(["--receipt".to_string(), receipt.display().to_string()]);
    observe::parse(&raw_args)
        .expect("parse")
        .expect("observe command")
}

fn write_default_receipt(root: &Path, command: &observe::command::ObserveCommand, status: &str) {
    let receipt =
        observe::telemetry::base_receipt(root, command, status, None).expect("base receipt");
    let path = root.join(command.operation.receipt_rel());
    crate::json_boundary::write_json(&path, &receipt).expect("write receipt");
}

fn write_bad_live_receipt(root: &Path, operation: ObserveOperation, value: Value) {
    crate::json_boundary::write_json(&root.join(operation.receipt_rel()), &value)
        .expect("bad live receipt");
}

fn missing_component_receipt(component: &str, source: &Value) -> Value {
    let mut copy = source.clone();
    copy.as_object_mut().unwrap().remove(component);
    copy
}
