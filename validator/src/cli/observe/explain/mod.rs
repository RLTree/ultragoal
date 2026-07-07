use crate::cli::observe::telemetry;
use crate::cli::observe::types::ObserveCommand;
use serde_json::{Value, json};
use std::path::Path;

mod fallback;
mod next;
mod query;
mod receipt;
mod summary;
mod target;

pub(crate) fn query_evidence_for_target(
    root: &Path,
    observed: Option<&Value>,
    candidate: &str,
) -> Value {
    query::evidence::for_target(root, observed, candidate)
}

pub(crate) fn run(root: &Path, command: &ObserveCommand) -> Result<Value, String> {
    if command.operation == crate::cli::observe::types::ObserveOperation::ExplainNext {
        return next::run(root, command);
    }
    let audit = root.join("validation_artifacts/ultragoal-audit/validator-receipt.json");
    let candidate = crate::package::inventory::package_digest(root)?;
    let target_requested = target::requested(command);
    let observed = target::event(root, command);
    let stale_observed = target::stale_failure(observed.as_ref(), &candidate);
    let missing_observed = target_requested
        .as_ref()
        .filter(|_| observed.is_none())
        .map(|target| format!("requested telemetry target unavailable:{target}"));
    let opaque_observed = target::opaque_failure(observed.as_ref());
    let known_current_failure = known_failure(
        root,
        observed.as_ref(),
        stale_observed.as_deref(),
        missing_observed.as_deref(),
        opaque_observed.as_deref(),
    );
    let observed_failed = observed
        .as_ref()
        .and_then(|event| target::event_failure(event))
        .is_some();
    let failure_summary = failure_summary(
        observed.as_ref(),
        stale_observed.as_deref(),
        missing_observed.as_deref(),
        opaque_observed.as_deref(),
        &known_current_failure,
    );
    let receipt_status = if stale_observed.is_some()
        || missing_observed.is_some()
        || opaque_observed.is_some()
        || observed_failed
    {
        "fail"
    } else {
        "pass"
    };
    let mut receipt = telemetry::base_receipt_for_candidate(
        root,
        command,
        receipt_status,
        failure_summary.as_deref(),
        candidate.clone(),
    )?;
    let repair_guidance = summary::repair_guidance(
        observed.as_ref(),
        stale_observed.as_deref(),
        missing_observed.as_deref(),
        opaque_observed.as_deref(),
    );
    if let Some(event) = observed.as_ref() {
        target::promote_fields(&mut receipt, event);
        receipt["observed_run"] = event.clone();
        receipt["explanation_target"] = summary::target_summary(event, &candidate);
    }
    receipt["next_repair"] = json!(repair_guidance);
    receipt["explanation"] = summary::explanation(
        root,
        crate::digest::file(&audit).unwrap_or_else(|_| crate::digest::ZERO.to_string()),
        summary::ExplainContext {
            candidate: &candidate,
            target_requested: target_requested.as_deref(),
            observed: observed.as_ref(),
            stale_observed: stale_observed.as_deref(),
            missing_observed: missing_observed.as_deref(),
            opaque_observed: opaque_observed.as_deref(),
            known_current_failure: &known_current_failure,
            repair_guidance: &repair_guidance,
        },
    );
    receipt["explanation"]["requested_run_id"] = json!(command.run_id);
    receipt["explanation"]["requested_claim_id"] = json!(command.claim_id);
    receipt["explanation"]["requested_check_id"] = json!(command.check_id);
    receipt["explanation"]["requested_law_id"] = json!(command.law_id);
    Ok(receipt)
}

fn known_failure(
    root: &Path,
    observed: Option<&Value>,
    stale_observed: Option<&str>,
    missing_observed: Option<&str>,
    opaque_observed: Option<&str>,
) -> Value {
    if let Some(failure) = stale_observed {
        json!([failure])
    } else if let Some(failure) = missing_observed {
        json!([failure])
    } else if let Some(failure) = opaque_observed {
        json!([failure])
    } else if let Some(failure) = observed.and_then(target::event_failure) {
        failure
    } else if observed.is_some_and(target::event_passed) {
        json!(["requested telemetry target passed; no failure"])
    } else {
        fallback::current_failure(root)
    }
}

fn failure_summary(
    observed: Option<&Value>,
    stale_observed: Option<&str>,
    missing_observed: Option<&str>,
    opaque_observed: Option<&str>,
    known_current_failure: &Value,
) -> Option<String> {
    if observed.is_some_and(target::event_passed) && stale_observed.is_none() {
        None
    } else {
        stale_observed
            .map(str::to_string)
            .or_else(|| missing_observed.map(str::to_string))
            .or_else(|| opaque_observed.map(str::to_string))
            .or_else(|| fallback::failure_summary(known_current_failure))
    }
}

#[cfg(test)]
mod tests;
