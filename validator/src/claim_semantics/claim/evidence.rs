use crate::audit::contract::Failure;
use crate::claim_semantics::{array_strings, evidence, good_status, str_field};
use crate::digest;
use serde_json::Value;
use std::path::Path;

pub(crate) fn evidence_checks(
    claim: &Value,
    cm: &Value,
    ready: &Value,
    root: &Path,
    out: &mut Vec<Failure>,
) {
    simulated_live_proof_checks(claim, out);
    for ev in evidence(claim) {
        if !array_strings(claim, "allowed_evidence_surfaces").contains(&str_field(ev, "surface")) {
            out.push(Failure::new(
                "claim-evidence-coupling",
                "proof_surface_substitution",
                str_field(ev, "id"),
            ));
        }
        freshness_checks(ev, cm, out);
        artifact_ref_checks(ev, root, out);
        if good_status(&str_field(claim, "status"))
            || str_field(claim, "claim_ceiling_effect") == "included"
        {
            command_join_check(ev, ready, root, out);
        }
    }
}

pub(crate) fn live_e2e_check(claim: &Value, ready: &Value, root: &Path, out: &mut Vec<Failure>) {
    let live = evidence(claim)
        .into_iter()
        .filter(|ev| str_field(ev, "kind") == "live_beneficial_e2e")
        .collect::<Vec<_>>();
    if live.is_empty() {
        out.push(Failure::new(
            "live-beneficial-e2e",
            "live_beneficial_e2e_missing",
            str_field(claim, "id"),
        ));
    }
    for ev in live {
        let task = &ev["live_beneficial_task"];
        let text = format!(
            "{} {} {}",
            str_field(ev, "path"),
            str_field(task, "real_input_path"),
            str_field(task, "output_artifact_path")
        );
        if str_field(ev, "digest") == digest::ZERO
            || ["fixture", "mock", "dummy", "hello-world"]
                .iter()
                .any(|s| text.contains(s))
        {
            out.push(Failure::new(
                "live-beneficial-e2e",
                "live_beneficial_e2e_not_live",
                str_field(ev, "id"),
            ));
        }
        command_join_check(ev, ready, root, out);
    }
}

fn freshness_checks(ev: &Value, cm: &Value, out: &mut Vec<Failure>) {
    if str_field(ev, "commit") != str_field(cm, "commit")
        || ev.pointer("/freshness/verdict").and_then(Value::as_str) != Some("current")
    {
        out.push(Failure::new(
            "evidence-freshness",
            "evidence_stale_or_invalidated",
            str_field(ev, "id"),
        ));
    }
    if str_field(ev, "workspace") != str_field(cm, "root") {
        out.push(Failure::new(
            "workspace-binding",
            "evidence_wrong_workspace",
            str_field(ev, "id"),
        ));
    }
}

fn artifact_ref_checks(ev: &Value, root: &Path, out: &mut Vec<Failure>) {
    if let Err(err) = crate::package::artifact::refs::validate_object(
        root,
        ev,
        &format!("claim evidence {}", str_field(ev, "id")),
    ) {
        out.push(Failure::new(
            "evidence-integrity",
            "evidence_digest_missing_or_mismatched",
            err,
        ));
    }
}

fn simulated_live_proof_checks(claim: &Value, out: &mut Vec<Failure>) {
    if str_field(claim, "claim_kind") != "feature_completion" {
        return;
    }
    for ev in evidence(claim) {
        if str_field(ev, "kind") == "live_beneficial_e2e"
            && str_field(ev, "path").contains("fixture")
        {
            out.push(Failure::new(
                "claim-evidence-coupling",
                "simulated_evidence_used_for_live_claim",
                str_field(ev, "path"),
            ));
        }
    }
}

fn command_join_check(ev: &Value, ready: &Value, root: &Path, out: &mut Vec<Failure>) {
    let commands = crate::claim_semantics::object_by_id(ready.get("commands"));
    if str_field(ev, "digest") == digest::ZERO {
        out.push(Failure::new(
            "evidence-integrity",
            "evidence_digest_missing_or_mismatched",
            str_field(ev, "id"),
        ));
    }
    let Some(command) = commands.get(&str_field(ev, "produced_by_command_id")) else {
        out.push(Failure::new(
            "command-evidence",
            "command_receipt_missing_or_failed",
            str_field(ev, "id"),
        ));
        return;
    };
    if command.get("exit").and_then(Value::as_i64) != Some(0) {
        out.push(Failure::new(
            "command-evidence",
            "command_receipt_missing_or_failed",
            str_field(ev, "id"),
        ));
        return;
    }
    command_artifact_checks(ev, command, root, out);
}

fn command_artifact_checks(ev: &Value, command: &Value, root: &Path, out: &mut Vec<Failure>) {
    if str_field(command, "artifact_digest") != str_field(ev, "digest")
        || str_field(command, "artifact_path") != str_field(ev, "path")
    {
        out.push(Failure::new(
            "evidence-integrity",
            "evidence_digest_missing_or_mismatched",
            str_field(ev, "id"),
        ));
    }
    if let Err(err) = crate::package::artifact::refs::validate_command_artifact(
        root,
        command,
        &format!("command artifact {}", str_field(command, "id")),
    ) {
        out.push(Failure::new(
            "evidence-integrity",
            "evidence_digest_missing_or_mismatched",
            err,
        ));
    }
}
