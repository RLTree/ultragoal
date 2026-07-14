use super::*;

pub fn reconcile_report(
    context: &LiveContext,
    plan: &RoutinePlan,
    result_scope: &str,
    records: Vec<ReportRecord>,
) -> Result<RoutineReport, RoutineError> {
    let current = require_current_binding(context, plan.binding(), "report-plan-binding-is-stale")?;
    #[cfg(test)]
    test_authority_checkpoint();
    if result_scope.is_empty() || result_scope.len() > 128 {
        return Err(report_error("report-result-scope-invalid"));
    }
    let selected = plan
        .checks()
        .iter()
        .map(|check| check.node_id().to_owned())
        .collect::<Vec<_>>();
    let selected_set = selected.iter().cloned().collect::<BTreeSet<_>>();
    let mut seen = BTreeSet::new();
    let mut result_artifacts = BTreeMap::new();
    for record in &records {
        if !selected_set.contains(&record.node_id) || !seen.insert(record.node_id.clone()) {
            return Err(report_error("report-row-unknown-or-duplicated"));
        }
        match &record.disposition {
            ReportDisposition::Executed(result) => {
                if result.node_id() != record.node_id
                    || !result
                        .facts
                        .binding
                        .matches_plan(plan, &record.node_id, result_scope)?
                    || !valid(&result.capture_run_sha256)
                    || !valid(&result.facts.result_artifact_sha256)
                {
                    return Err(report_error("execution-witness-binding-invalid"));
                }
                result_artifacts.insert(
                    record.node_id.clone(),
                    (
                        result.facts.result_artifact_sha256.clone(),
                        result.outcome() == RunOutcome::Passed && result.behavior_observed(),
                    ),
                );
            }
            ReportDisposition::Reused(evidence) => {
                if evidence.node_id() != record.node_id
                    || !evidence
                        .facts
                        .binding
                        .matches_plan(plan, &record.node_id, result_scope)?
                    || !valid(evidence.behavior_sha256())
                    || !valid(&evidence.facts.result_artifact_sha256)
                {
                    return Err(report_error("reuse-witness-binding-invalid"));
                }
                result_artifacts.insert(
                    record.node_id.clone(),
                    (evidence.facts.result_artifact_sha256.clone(), true),
                );
            }
            ReportDisposition::Skipped(_) | ReportDisposition::Failed { .. } => {}
        }
    }
    for record in &records {
        let binding = match &record.disposition {
            ReportDisposition::Executed(result) => &result.facts.binding,
            ReportDisposition::Reused(evidence) => &evidence.facts.binding,
            ReportDisposition::Skipped(_) | ReportDisposition::Failed { .. } => continue,
        };
        let check = plan
            .check(&record.node_id)
            .ok_or_else(|| report_error("report-row-unknown-or-duplicated"))?;
        let mut exact_dependencies = BTreeMap::new();
        for dependency in check.depends_on() {
            let Some((artifact_sha256, behaviorally_passed)) = result_artifacts.get(dependency)
            else {
                return Err(report_error("report-dependency-result-chain-inconsistent"));
            };
            if !behaviorally_passed {
                return Err(report_error("report-dependency-result-chain-inconsistent"));
            }
            exact_dependencies.insert(dependency.clone(), artifact_sha256.clone());
        }
        if !binding.dependency_results_match(&exact_dependencies) {
            return Err(report_error("report-dependency-result-chain-inconsistent"));
        }
    }
    seen.clear();
    let mut executed = Vec::new();
    let mut reused = Vec::new();
    let mut skipped = BTreeMap::new();
    let mut failed = BTreeMap::new();
    for record in records {
        seen.insert(record.node_id.clone());
        match record.disposition {
            ReportDisposition::Executed(result) => {
                if result.outcome() == RunOutcome::Passed && result.behavior_observed() {
                    executed.push(record.node_id);
                } else {
                    failed.insert(record.node_id, "behavior-not-passed".to_owned());
                }
            }
            ReportDisposition::Reused(_evidence) => {
                reused.push(record.node_id);
            }
            ReportDisposition::Skipped(reason) => {
                skipped.insert(record.node_id, reason);
            }
            ReportDisposition::Failed { cause_code } => {
                if !valid_cause(&cause_code) {
                    return Err(report_error("failure-cause-code-invalid"));
                }
                failed.insert(record.node_id, cause_code);
            }
        }
    }
    for missing in selected_set.difference(&seen) {
        skipped.insert(missing.clone(), SkipReason::Cancelled);
    }
    executed.sort();
    reused.sort();
    let status = if skipped.is_empty()
        && failed.is_empty()
        && executed.len() + reused.len() == selected.len()
    {
        ReportStatus::CompleteExecution
    } else {
        ReportStatus::IncompleteExecution
    };
    let support_limit = "candidate-bound routine execution evidence only; no claim decision";
    let payload = ReportPayload {
        binding_id: current.binding_id(),
        context_id: current.context_id(),
        candidate_id: current.candidate_id(),
        plan_id: plan.plan_id(),
        result_scope,
        selected: &selected,
        executed: &executed,
        reused: &reused,
        skipped: &skipped,
        failed: &failed,
        status,
        support_limit,
    };
    let report_id = digest_of(&payload)?;
    ensure_unchanged(
        context,
        &current,
        "routine-binding-changed-during-report-reconciliation",
    )?;
    Ok(RoutineReport {
        report_id,
        binding_id: current.binding_id().to_owned(),
        context_id: current.context_id().to_owned(),
        candidate_id: current.candidate_id().to_owned(),
        plan_id: plan.plan_id().to_owned(),
        result_scope: result_scope.to_owned(),
        selected,
        executed,
        reused,
        skipped,
        failed,
        status,
        support_limit,
    })
}

pub(crate) fn valid_cause(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'-')
}

pub(crate) fn report_error(cause: &'static str) -> RoutineError {
    RoutineError::new(RoutineErrorId::InvalidRequest, cause, None)
}
